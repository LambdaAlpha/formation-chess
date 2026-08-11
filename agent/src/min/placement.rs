use std::cmp::Ordering;
use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Place;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;

use super::MIN_TERMINAL_UTILITY;
use super::MinEvaluator;
use super::MinPlacementSearchConfig;
use super::evaluator::placement_candidate_potential_units;
use crate::AgentError;
use crate::PlacementArea;
use crate::ScoredAction;
use crate::placement_area;

const ROOT_DIVERSITY_POOL_MULTIPLIER: usize = 8;
const NON_TERMINAL_PLACEMENT_UTILITY_LIMIT: i32 = MIN_TERMINAL_UTILITY as i32 - 1;
const PLACEMENT_WHOLE_LAYOUT_DIVISOR: i64 = 8;
const PLACEMENT_WHOLE_LAYOUT_LIMIT: i64 = 128;

pub(super) fn analyze_placements(
    game: &Game, area: PlacementArea, top_k: NonZeroU8, config: MinPlacementSearchConfig,
    evaluator: MinEvaluator,
) -> Result<Vec<ScoredAction>, AgentError> {
    if game.phase() != Phase::Place || game.result() != GameResult::Unfinished {
        return Err(AgentError::Decision(
            "Min placement search requires an unfinished placement position".to_owned(),
        ));
    }

    let root_player = game.player();
    let mut search_game = game.clone();
    let max_depth = config.max_depth.get();
    let max_nodes =
        usize::try_from(config.max_nodes.get()).expect("validated Min node budget must fit usize");
    let root_width = usize::from(config.root_width.get());
    let root_probe_limit = probe_limit(max_nodes, root_width, max_depth > 1);
    let root_selection = select_root_children(
        &mut search_game,
        area,
        root_player,
        evaluator,
        root_width,
        root_probe_limit,
    )?;
    if root_selection.children.is_empty() {
        return Err(no_placement_error(game));
    }

    let mut remaining_nodes = max_nodes - root_selection.probed;
    let root_count = root_selection.children.len();
    let mut results = Vec::with_capacity(root_count);
    for (index, child) in root_selection.children.into_iter().enumerate() {
        let branches_left = root_count - index;
        let branch_budget =
            if max_depth > 1 { fair_share(remaining_nodes, branches_left) } else { 0 };
        let reaction = search_game.action(child.action).map_err(|message| {
            AgentError::Decision(format!(
                "generated Min placement was rejected during search: {message}"
            ))
        })?;
        let search = if branch_budget > 0 && search_game.phase() == Phase::Place {
            search_position(
                &mut search_game,
                root_player,
                evaluator,
                config,
                max_depth - 1,
                branch_budget,
            )
        } else {
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0 })
        };
        search_game.undo(reaction);
        let search = search?;
        remaining_nodes -= search.nodes;
        results.push(RootResult {
            action: child.action,
            ordering_utility: child.ordering_utility,
            expert_utility: child.expert_utility,
            utility: search.utility,
            ordinal: child.ordinal,
        });
    }

    results.sort_by(|left, right| {
        right
            .utility
            .cmp(&left.utility)
            .then_with(|| right.ordering_utility.cmp(&left.ordering_utility))
            .then_with(|| right.expert_utility.cmp(&left.expert_utility))
            .then_with(|| left.ordinal.cmp(&right.ordinal))
    });
    results.truncate(usize::from(top_k.get()));

    Ok(results
        .into_iter()
        .map(|result| ScoredAction {
            action: result.action,
            score: result.utility as f32 / f32::from(MIN_TERMINAL_UTILITY),
        })
        .collect())
}

fn search_position(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator,
    config: MinPlacementSearchConfig, depth_remaining: u8, budget: usize,
) -> Result<SearchResult, AgentError> {
    if depth_remaining == 0
        || budget == 0
        || game.phase() != Phase::Place
        || game.result() != GameResult::Unfinished
    {
        return Ok(SearchResult {
            utility: evaluator.evaluate(game, root_player).utility,
            nodes: 0,
        });
    }

    let area = placement_area(game).ok_or_else(|| {
        AgentError::Decision("recursive Min placement input is unavailable".to_owned())
    })?;
    let maximizing = game.player() == root_player;
    let width = if maximizing {
        usize::from(config.response_width.get())
    } else {
        usize::from(config.opponent_width.get())
    };
    let candidate_probe_limit = probe_limit(budget, width, depth_remaining > 1);
    let selection = select_children(
        game,
        area,
        root_player,
        evaluator,
        width,
        maximizing,
        candidate_probe_limit,
    )?;
    if selection.children.is_empty() {
        return Err(no_placement_error(game));
    }

    let mut nodes = selection.probed;
    if depth_remaining == 1 {
        let utility = preferred_utility(&selection.children, maximizing);
        return Ok(SearchResult { utility, nodes });
    }

    let mut remaining_nodes = budget - nodes;
    let child_count = selection.children.len();
    let mut utility: Option<i32> = None;
    for (index, child) in selection.children.into_iter().enumerate() {
        let branches_left = child_count - index;
        let branch_budget = fair_share(remaining_nodes, branches_left);
        let reaction = game.action(child.action).map_err(|message| {
            AgentError::Decision(format!("generated Min placement was rejected: {message}"))
        })?;
        let child_search = if branch_budget > 0 && game.phase() == Phase::Place {
            search_position(
                game,
                root_player,
                evaluator,
                config,
                depth_remaining - 1,
                branch_budget,
            )
        } else {
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0 })
        };
        game.undo(reaction);
        let child_search = child_search?;
        remaining_nodes -= child_search.nodes;
        nodes += child_search.nodes;
        utility = Some(match utility {
            None => child_search.utility,
            Some(current) if maximizing => current.max(child_search.utility),
            Some(current) => current.min(child_search.utility),
        });
    }

    Ok(SearchResult {
        utility: utility.expect("nonempty placement selection must produce a utility"),
        nodes,
    })
}

fn select_root_children(
    game: &mut Game, area: PlacementArea, root_player: Player, evaluator: MinEvaluator,
    width: usize, probe_limit: usize,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    let (candidates, probed) =
        probe_children(game, area, root_player, evaluator, true, probe_limit)?;
    let children = select_diverse_children(candidates, area, width, true);
    Ok(ChildSelection { children, probed })
}

fn select_children(
    game: &mut Game, area: PlacementArea, root_player: Player, evaluator: MinEvaluator,
    width: usize, maximizing: bool, probe_limit: usize,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    let (candidates, probed) =
        probe_children(game, area, root_player, evaluator, false, probe_limit)?;
    let mut children = Vec::with_capacity(width.min(probed));
    for candidate in candidates {
        insert_child(&mut children, candidate, width, maximizing);
    }
    Ok(ChildSelection { children, probed })
}

fn probe_children(
    game: &mut Game, area: PlacementArea, root_player: Player, evaluator: MinEvaluator,
    use_expert: bool, probe_limit: usize,
) -> Result<(Vec<SearchChild>, usize), AgentError> {
    let pieces = unique_pool(game);
    let positions = area
        .positions()
        .filter(|position| game.board().get(*position).is_none())
        .collect::<Vec<_>>();
    if pieces.is_empty() || positions.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let total = pieces.len().checked_mul(positions.len()).ok_or_else(|| {
        AgentError::Decision("placement candidate count overflowed usize".to_owned())
    })?;
    let probes = total.min(probe_limit);
    let mut candidates = Vec::with_capacity(probes);
    for sample_index in 0 .. probes {
        let ordinal = spread_index(sample_index, probes, total);
        let piece = pieces[ordinal / positions.len()];
        let to = positions[ordinal % positions.len()];
        let action = Action::Place(Place { piece: piece.id(), to });
        let reaction = game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min placement was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(game, root_player).utility;
        game.undo(reaction);
        candidates.push(SearchChild { action, ordering_utility, expert_utility: 0, ordinal });
    }
    if use_expert {
        assign_allocation_utilities(game, root_player, &positions, &mut candidates);
    }
    Ok((candidates, probes))
}

fn assign_allocation_utilities(
    game: &Game, root_player: Player, positions: &[(u8, u8)], candidates: &mut [SearchChild],
) {
    let player = game.player();
    let maximizing = player == root_player;
    let current_pool = current_pool(game);
    if candidates.len() < 2 || current_pool.is_empty() || current_pool.len() > positions.len() {
        return;
    }

    let board = game.board();
    let current_matrix = placement_potential_matrix(board, player, current_pool, positions);
    let opponent = other_player(player);
    let opponent_pool = pool_for_player(game, opponent);
    let opponent_positions = placement_positions(board, opponent);
    let opponent_matrix =
        placement_potential_matrix(board, opponent, opponent_pool, &opponent_positions);
    let opponent_baseline = greedy_assignment(&opponent_matrix);
    let baseline_board = project_assignment(
        board,
        opponent_pool,
        &opponent_positions,
        &opponent_baseline.assignments,
    );
    let own_baseline = greedy_assignment(&placement_potential_matrix(
        &baseline_board,
        player,
        current_pool,
        positions,
    ));

    for candidate in candidates {
        let piece_id = placement_piece(candidate.action);
        let Some(piece_index) = current_pool.iter().position(|piece| piece.id() == piece_id) else {
            unreachable!("placement search candidate piece must come from the pool")
        };
        let position = placement_position(candidate.action);
        let Some(position_index) = positions.iter().position(|candidate| *candidate == position)
        else {
            unreachable!("placement search candidate position must be empty")
        };
        let opportunity_units =
            allocation_opportunity_cost(&current_matrix, piece_index, position_index);
        let root_opportunity_units =
            if maximizing { -opportunity_units } else { opportunity_units };
        let whole_layout_units = if affects_opponent_layout(board, player, position) {
            bilateral_layout_delta(
                board,
                AllocationSide { player, pool: current_pool, positions, baseline: &own_baseline },
                AllocationSide {
                    player: opponent,
                    pool: opponent_pool,
                    positions: &opponent_positions,
                    baseline: &opponent_baseline,
                },
                piece_index,
                position,
            )
        } else {
            0
        };
        let root_layout_units = if maximizing { whole_layout_units } else { -whole_layout_units };
        candidate.expert_utility =
            i32::try_from(root_opportunity_units + normalize_whole_layout_units(root_layout_units))
                .expect("placement expert utility must fit i32");
    }
}

struct AllocationSide<'a> {
    player: Player,
    pool: &'a [Piece],
    positions: &'a [(u8, u8)],
    baseline: &'a Allocation,
}

fn bilateral_layout_delta(
    board: &formation_chess_core::board::Board, current: AllocationSide<'_>,
    opponent: AllocationSide<'_>, piece_index: usize, position: (u8, u8),
) -> i64 {
    let piece = current.pool[piece_index];
    let mut candidate_board = board.clone();
    candidate_board
        .place(piece, position)
        .expect("placement candidate must be legal on the projected board");
    let opponent_after = greedy_assignment(&placement_potential_matrix(
        &candidate_board,
        opponent.player,
        opponent.pool,
        opponent.positions,
    ));
    let opponent_after_board = project_assignment(
        &candidate_board,
        opponent.pool,
        opponent.positions,
        &opponent_after.assignments,
    );

    let mut remaining_pool = Vec::with_capacity(current.pool.len().saturating_sub(1));
    for (index, &pool_piece) in current.pool.iter().enumerate() {
        if index != piece_index {
            remaining_pool.push(pool_piece);
        }
    }
    let remaining_positions = current
        .positions
        .iter()
        .copied()
        .filter(|candidate| *candidate != position)
        .collect::<Vec<_>>();
    let own_after_remaining = greedy_assignment(&placement_potential_matrix(
        &opponent_after_board,
        current.player,
        &remaining_pool,
        &remaining_positions,
    ));
    let candidate_units =
        placement_candidate_potential_units(&opponent_after_board, current.player, piece, position);
    let own_after = candidate_units + own_after_remaining.total;
    let opponent_delta = opponent_after.total - opponent.baseline.total;
    let own_delta = own_after - current.baseline.total;
    own_delta - opponent_delta
}

fn normalize_whole_layout_units(units: i64) -> i64 {
    let normalized = units / PLACEMENT_WHOLE_LAYOUT_DIVISOR;
    normalized.clamp(-PLACEMENT_WHOLE_LAYOUT_LIMIT, PLACEMENT_WHOLE_LAYOUT_LIMIT)
}

fn greedy_assignment(values: &[Vec<i64>]) -> Allocation {
    if values.is_empty() || values[0].is_empty() {
        return Allocation { total: 0, assignments: Vec::new() };
    }

    let mut row_order = (0 .. values.len()).collect::<Vec<_>>();
    row_order.sort_by(|left, right| {
        assignment_regret(&values[*right]).cmp(&assignment_regret(&values[*left]))
    });
    let mut used_columns = vec![false; values[0].len()];
    let mut assignments = Vec::with_capacity(values.len());
    let mut total = 0i64;
    for row in row_order {
        let mut best_column = None;
        for column in 0 .. values[row].len() {
            if used_columns[column] {
                continue;
            }
            if best_column.is_none_or(|best| values[row][column] > values[row][best]) {
                best_column = Some(column);
            }
        }
        let Some(column) = best_column else { break };
        used_columns[column] = true;
        total += values[row][column];
        assignments.push((row, column));
    }
    Allocation { total, assignments }
}

fn assignment_regret(values: &[i64]) -> i64 {
    let mut best = i64::MIN;
    let mut second_best = i64::MIN;
    for &value in values {
        if value > best {
            second_best = best;
            best = value;
        } else if value > second_best {
            second_best = value;
        }
    }
    best.saturating_sub(second_best)
}

fn project_assignment(
    board: &formation_chess_core::board::Board, pool: &[Piece], positions: &[(u8, u8)],
    assignments: &[(usize, usize)],
) -> formation_chess_core::board::Board {
    let mut projected = board.clone();
    for &(piece_index, position_index) in assignments {
        projected
            .place(pool[piece_index], positions[position_index])
            .expect("projected placement assignment must be legal");
    }
    projected
}

fn affects_opponent_layout(
    board: &formation_chess_core::board::Board, player: Player, position: (u8, u8),
) -> bool {
    let opponent = other_player(player);
    let (y_start, y_end) = match opponent {
        Player::Red => (board.height().div_ceil(2), board.height()),
        Player::Black => (0, board.height() / 2),
    };
    for dy in -1i8 ..= 1i8 {
        let y = i16::from(position.1) + i16::from(dy);
        if y >= i16::from(y_start) && y < i16::from(y_end) {
            return true;
        }
    }
    false
}

fn placement_potential_matrix(
    board: &formation_chess_core::board::Board, player: Player, pool: &[Piece],
    positions: &[(u8, u8)],
) -> Vec<Vec<i64>> {
    let mut matrix = Vec::with_capacity(pool.len());
    for &piece in pool {
        let mut row = Vec::with_capacity(positions.len());
        for &position in positions {
            row.push(placement_candidate_potential_units(board, player, piece, position));
        }
        matrix.push(row);
    }
    matrix
}

fn allocation_opportunity_cost(values: &[Vec<i64>], row: usize, column: usize) -> i64 {
    let mut reservation_cost = 0i64;
    for (other_row, other_values) in values.iter().enumerate() {
        if other_row == row {
            continue;
        }
        let mut best_alternative = i64::MIN;
        for (other_column, &value) in other_values.iter().enumerate() {
            if other_column == column {
                continue;
            }
            best_alternative = best_alternative.max(value);
        }
        reservation_cost = reservation_cost.max(other_values[column] - best_alternative);
    }

    reservation_cost.max(0)
}
#[cfg(test)]
fn maximum_assignment_units(
    values: &[Vec<i64>], skipped_row: Option<usize>, skipped_column: Option<usize>,
) -> i64 {
    let row_indices =
        (0 .. values.len()).filter(|index| Some(*index) != skipped_row).collect::<Vec<_>>();
    if row_indices.is_empty() {
        return 0;
    }

    let column_count = values[0].len();
    let column_indices =
        (0 .. column_count).filter(|index| Some(*index) != skipped_column).collect::<Vec<_>>();
    assert!(row_indices.len() <= column_indices.len(), "assignment requires enough positions");

    let row_count = row_indices.len();
    let available_columns = column_indices.len();
    let mut row_potential = vec![0i64; row_count + 1];
    let mut column_potential = vec![0i64; available_columns + 1];
    let mut matched_row = vec![0usize; available_columns + 1];
    let mut previous_column = vec![0usize; available_columns + 1];

    for assignment_row in 1 ..= row_count {
        matched_row[0] = assignment_row;
        let mut minimum_slack = vec![i64::MAX; available_columns + 1];
        let mut used = vec![false; available_columns + 1];
        let mut current_column = 0usize;
        loop {
            used[current_column] = true;
            let current_row = matched_row[current_column];
            let source_row = row_indices[current_row - 1];
            let mut delta = i64::MAX;
            let mut next_column = 0usize;
            for candidate_column in 1 ..= available_columns {
                if used[candidate_column] {
                    continue;
                }
                let source_column = column_indices[candidate_column - 1];
                let cost = -values[source_row][source_column];
                let slack = cost - row_potential[current_row] - column_potential[candidate_column];
                if slack < minimum_slack[candidate_column] {
                    minimum_slack[candidate_column] = slack;
                    previous_column[candidate_column] = current_column;
                }
                if minimum_slack[candidate_column] < delta {
                    delta = minimum_slack[candidate_column];
                    next_column = candidate_column;
                }
            }

            for candidate_column in 0 ..= available_columns {
                if used[candidate_column] {
                    row_potential[matched_row[candidate_column]] += delta;
                    column_potential[candidate_column] -= delta;
                } else {
                    minimum_slack[candidate_column] -= delta;
                }
            }
            current_column = next_column;
            if matched_row[current_column] == 0 {
                break;
            }
        }

        loop {
            let next_column = previous_column[current_column];
            matched_row[current_column] = matched_row[next_column];
            current_column = next_column;
            if current_column == 0 {
                break;
            }
        }
    }

    let mut total = 0i64;
    for assignment_column in 1 ..= available_columns {
        let assignment_row = matched_row[assignment_column];
        if assignment_row == 0 {
            continue;
        }
        total += values[row_indices[assignment_row - 1]][column_indices[assignment_column - 1]];
    }
    total
}
fn select_diverse_children(
    mut candidates: Vec<SearchChild>, area: PlacementArea, width: usize, maximizing: bool,
) -> Vec<SearchChild> {
    candidates.sort_by(|left, right| compare_children(left, right, maximizing));
    if candidates.len() <= width {
        return candidates;
    }

    let diversity_pool_len =
        candidates.len().min(width.saturating_mul(ROOT_DIVERSITY_POOL_MULTIPLIER));
    let piece_target =
        width.div_ceil(2).min(unique_piece_count(&candidates[.. diversity_pool_len]));
    let spatial_grid = spatial_grid_side(width.saturating_sub(piece_target));
    let mut selected = Vec::with_capacity(width);
    let mut selected_indices = vec![false; candidates.len()];
    let mut selected_pieces = Vec::with_capacity(piece_target);
    for index in 0 .. diversity_pool_len {
        if selected.len() >= piece_target {
            break;
        }
        let piece = placement_piece(candidates[index].action);
        if selected_pieces.contains(&piece) {
            continue;
        }
        selected_indices[index] = true;
        selected_pieces.push(piece);
        selected.push(candidates[index]);
    }

    let mut selected_buckets = Vec::with_capacity(width);
    for candidate in &selected {
        let bucket = placement_bucket(area, placement_position(candidate.action), spatial_grid);
        if !selected_buckets.contains(&bucket) {
            selected_buckets.push(bucket);
        }
    }
    for index in 0 .. diversity_pool_len {
        if selected.len() >= width {
            break;
        }
        if selected_indices[index] {
            continue;
        }
        let bucket =
            placement_bucket(area, placement_position(candidates[index].action), spatial_grid);
        if selected_buckets.contains(&bucket) {
            continue;
        }
        selected_indices[index] = true;
        selected_buckets.push(bucket);
        selected.push(candidates[index]);
    }

    for (index, candidate) in candidates.into_iter().enumerate() {
        if selected.len() >= width {
            break;
        }
        if selected_indices[index] {
            continue;
        }
        selected.push(candidate);
    }
    selected.sort_by(|left, right| compare_children(left, right, maximizing));
    selected
}

fn unique_piece_count(candidates: &[SearchChild]) -> usize {
    let mut pieces = Vec::new();
    for candidate in candidates {
        let piece = placement_piece(candidate.action);
        if !pieces.contains(&piece) {
            pieces.push(piece);
        }
    }
    pieces.len()
}

fn placement_piece(action: Action) -> PieceId {
    let Action::Place(place) = action else {
        unreachable!("placement search candidate must be a placement")
    };
    place.piece
}

fn placement_position(action: Action) -> (u8, u8) {
    let Action::Place(place) = action else {
        unreachable!("placement search candidate must be a placement")
    };
    place.to
}

fn spatial_grid_side(slot_count: usize) -> usize {
    if slot_count <= 1 {
        return 1;
    }
    let mut side: usize = 1;
    while side.saturating_mul(side) < slot_count {
        side += 1;
    }
    side
}

fn placement_bucket(area: PlacementArea, (x, y): (u8, u8), grid_side: usize) -> PlacementBucket {
    let x_range = area.x_range();
    let y_range = area.y_range();
    PlacementBucket {
        x: bucket_coordinate(x, x_range.start, x_range.end, grid_side),
        y: bucket_coordinate(y, y_range.start, y_range.end, grid_side),
    }
}

fn bucket_coordinate(value: u8, start: u8, end: u8, grid_side: usize) -> usize {
    if grid_side <= 1 || start >= end {
        return 0;
    }
    let span = usize::from(end - start);
    let offset = usize::from(value.saturating_sub(start));
    (offset.saturating_mul(grid_side) / span).min(grid_side - 1)
}

fn compare_children(left: &SearchChild, right: &SearchChild, maximizing: bool) -> Ordering {
    if child_precedes(left, right, maximizing) {
        return Ordering::Less;
    }
    if child_precedes(right, left, maximizing) {
        return Ordering::Greater;
    }
    Ordering::Equal
}

fn current_pool(game: &Game) -> &[Piece] {
    pool_for_player(game, game.player())
}

fn pool_for_player(game: &Game, player: Player) -> &[Piece] {
    match player {
        Player::Red => game.red_pool(),
        Player::Black => game.black_pool(),
    }
}

fn other_player(player: Player) -> Player {
    match player {
        Player::Red => Player::Black,
        Player::Black => Player::Red,
    }
}

fn placement_positions(
    board: &formation_chess_core::board::Board, player: Player,
) -> Vec<(u8, u8)> {
    let (y_start, y_end) = match player {
        Player::Red => (board.height().div_ceil(2), board.height()),
        Player::Black => (0, board.height() / 2),
    };
    let mut positions = Vec::new();
    for x in 0 .. board.width() {
        for y in y_start .. y_end {
            if board.get((x, y)).is_none() {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn unique_pool(game: &Game) -> Vec<Piece> {
    let pool = current_pool(game);
    let mut pieces = Vec::with_capacity(pool.len());
    for &piece in pool {
        if !pieces.contains(&piece) {
            pieces.push(piece);
        }
    }
    pieces
}

fn spread_index(sample_index: usize, sample_count: usize, total: usize) -> usize {
    let numerator = sample_index as u128 * total as u128;
    usize::try_from(numerator / sample_count as u128)
        .expect("sampled placement index must fit usize")
}

fn insert_child(
    children: &mut Vec<SearchChild>, candidate: SearchChild, width: usize, maximizing: bool,
) {
    let index = children
        .iter()
        .position(|existing| child_precedes(&candidate, existing, maximizing))
        .unwrap_or(children.len());
    children.insert(index, candidate);
    if children.len() > width {
        children.pop();
    }
}

fn child_precedes(left: &SearchChild, right: &SearchChild, maximizing: bool) -> bool {
    let left_utility = adjusted_placement_utility(left.ordering_utility, left.expert_utility);
    let right_utility = adjusted_placement_utility(right.ordering_utility, right.expert_utility);
    let ordering = left_utility
        .cmp(&right_utility)
        .then_with(|| left.ordering_utility.cmp(&right.ordering_utility))
        .then_with(|| left.expert_utility.cmp(&right.expert_utility));
    match ordering {
        Ordering::Equal => left.ordinal < right.ordinal,
        Ordering::Greater => maximizing,
        Ordering::Less => !maximizing,
    }
}

fn adjusted_placement_utility(utility: i32, expert_utility: i32) -> i32 {
    utility
        .saturating_add(expert_utility)
        .clamp(-NON_TERMINAL_PLACEMENT_UTILITY_LIMIT, NON_TERMINAL_PLACEMENT_UTILITY_LIMIT)
}
fn preferred_utility(children: &[SearchChild], maximizing: bool) -> i32 {
    children
        .iter()
        .map(|child| child.ordering_utility)
        .reduce(|left, right| if maximizing { left.max(right) } else { left.min(right) })
        .expect("nonempty placement selection must have a preferred utility")
}

fn probe_limit(budget: usize, width: usize, has_deeper_search: bool) -> usize {
    if !has_deeper_search {
        return budget;
    }
    (budget / 2).max(width).min(budget)
}

fn fair_share(remaining: usize, branches_left: usize) -> usize {
    if remaining == 0 { 0 } else { remaining.div_ceil(branches_left) }
}

fn no_placement_error(game: &Game) -> AgentError {
    let pool_empty = match game.player() {
        Player::Red => game.red_pool().is_empty(),
        Player::Black => game.black_pool().is_empty(),
    };
    let message = if pool_empty {
        format!("{} has no pieces left to place", game.player())
    } else {
        "placement area has no empty point".to_owned()
    };
    AgentError::Decision(message)
}

#[derive(Default)]
struct ChildSelection {
    children: Vec<SearchChild>,
    probed: usize,
}

#[derive(Copy, Clone)]
struct SearchChild {
    action: Action,
    ordering_utility: i32,
    expert_utility: i32,
    ordinal: usize,
}

struct RootResult {
    action: Action,
    ordering_utility: i32,
    expert_utility: i32,
    utility: i32,
    ordinal: usize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct PlacementBucket {
    x: usize,
    y: usize,
}

struct SearchResult {
    utility: i32,
    nodes: usize,
}

struct Allocation {
    total: i64,
    assignments: Vec<(usize, usize)>,
}

#[cfg(test)]
mod tests {
    use formation_chess_core::action::Action;
    use formation_chess_core::action::Place;
    use formation_chess_core::game::Game;
    use formation_chess_core::game::GameConfig;
    use formation_chess_core::piece::Piece;
    use formation_chess_core::piece::Player;

    use super::AllocationSide;
    use super::SearchChild;
    use super::affects_opponent_layout;
    use super::allocation_opportunity_cost;
    use super::bilateral_layout_delta;
    use super::child_precedes;
    use super::greedy_assignment;
    use super::maximum_assignment_units;
    use super::placement_potential_matrix;
    use super::project_assignment;
    use crate::placement_area;

    #[test]
    fn standard_opening_contains_real_position_opportunity_costs() {
        let game = Game::new(GameConfig::default()).expect("standard game");
        let area = placement_area(&game).expect("standard placement area");
        let positions = area.positions().collect::<Vec<_>>();
        let matrix =
            placement_potential_matrix(game.board(), Player::Red, game.red_pool(), &positions);
        let baseline = maximum_assignment_units(&matrix, None, None);
        let mut worst_cost = 0i64;
        for piece_index in 0 .. matrix.len() {
            for position_index in 0 .. positions.len() {
                let forced = matrix[piece_index][position_index]
                    + maximum_assignment_units(&matrix, Some(piece_index), Some(position_index));
                worst_cost = worst_cost.min(forced - baseline);
            }
        }

        assert!(worst_cost < 0, "standard opening should contain scarce positions");
    }

    #[test]
    fn opportunity_cost_heuristic_reserves_a_scarce_position_for_the_specialist() {
        let values = vec![vec![20, 16], vec![14, 1]];

        assert_eq!(allocation_opportunity_cost(&values, 0, 0), 13);
        assert_eq!(allocation_opportunity_cost(&values, 1, 0), 4);
    }

    #[test]
    fn maximum_assignment_reserves_a_scarce_position_for_the_specialist() {
        let values = vec![vec![20, 16], vec![14, 1]];

        let baseline = maximum_assignment_units(&values, None, None);
        let flexible_piece_takes_scarce =
            values[0][0] + maximum_assignment_units(&values, Some(0), Some(0));
        let specialist_takes_scarce =
            values[1][0] + maximum_assignment_units(&values, Some(1), Some(0));

        assert_eq!(baseline, 30);
        assert_eq!(flexible_piece_takes_scarce, 21);
        assert_eq!(specialist_takes_scarce, 30);
    }

    #[test]
    fn maximum_assignment_treats_duplicate_pieces_as_separate_instances() {
        let values = vec![vec![9, 8, 0], vec![9, 8, 0], vec![7, 0, 6]];

        assert_eq!(maximum_assignment_units(&values, None, None), 23);
        assert_eq!(maximum_assignment_units(&values, Some(0), Some(0)), 14);
    }

    #[test]
    fn opportunity_cost_can_overrule_a_small_static_advantage() {
        let greedy = SearchChild {
            action: Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 5) }),
            ordering_utility: 105,
            expert_utility: -9,
            ordinal: 0,
        };
        let globally_better = SearchChild {
            action: Action::Place(Place { piece: Piece::RED_SHIELD.id(), to: (0, 5) }),
            ordering_utility: 100,
            expert_utility: 0,
            ordinal: 1,
        };

        assert!(child_precedes(&globally_better, &greedy, true));
    }

    #[test]
    fn expert_utility_breaks_static_ties() {
        let weaker = SearchChild {
            action: Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 5) }),
            ordering_utility: 100,
            expert_utility: 4,
            ordinal: 0,
        };
        let stronger = SearchChild {
            action: Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (1, 5) }),
            ordering_utility: 100,
            expert_utility: 8,
            ordinal: 1,
        };

        assert!(child_precedes(&stronger, &weaker, true));
        assert!(child_precedes(&weaker, &stronger, false));
    }

    #[test]
    fn boundary_placement_changes_the_opponents_whole_layout_projection() {
        use formation_chess_core::board::Board;

        let board = Board::new(9, 10);
        let current_pool = [Piece::RED_WIND];
        let current_positions = [(4, 5)];
        let opponent_pool = [Piece::BLACK_PAWN];
        let opponent_positions = [(4, 4), (0, 0)];
        let opponent_baseline = greedy_assignment(&placement_potential_matrix(
            &board,
            Player::Black,
            &opponent_pool,
            &opponent_positions,
        ));
        let baseline_board = project_assignment(
            &board,
            &opponent_pool,
            &opponent_positions,
            &opponent_baseline.assignments,
        );
        let own_baseline = greedy_assignment(&placement_potential_matrix(
            &baseline_board,
            Player::Red,
            &current_pool,
            &current_positions,
        ));
        let delta = bilateral_layout_delta(
            &board,
            AllocationSide {
                player: Player::Red,
                pool: &current_pool,
                positions: &current_positions,
                baseline: &own_baseline,
            },
            AllocationSide {
                player: Player::Black,
                pool: &opponent_pool,
                positions: &opponent_positions,
                baseline: &opponent_baseline,
            },
            0,
            (4, 5),
        );

        assert_ne!(delta, 0, "boundary placement should alter the bilateral projection");
    }

    #[test]
    fn non_boundary_placement_does_not_trigger_opponent_layout_projection() {
        use formation_chess_core::board::Board;

        let board = Board::new(9, 10);
        assert!(!affects_opponent_layout(&board, Player::Red, (4, 8)));
        assert!(affects_opponent_layout(&board, Player::Red, (4, 5)));
    }
}
