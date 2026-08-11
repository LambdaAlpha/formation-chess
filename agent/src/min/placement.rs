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
use crate::AgentError;
use crate::PlacementArea;
use crate::ScoredAction;
use crate::placement_area;

const ROOT_DIVERSITY_POOL_MULTIPLIER: usize = 8;

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
            utility: search.utility,
            ordinal: child.ordinal,
        });
    }

    results.sort_by(|left, right| {
        right
            .utility
            .cmp(&left.utility)
            .then_with(|| right.ordering_utility.cmp(&left.ordering_utility))
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

    let (candidates, probed) = probe_children(game, area, root_player, evaluator, probe_limit)?;
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

    let (candidates, probed) = probe_children(game, area, root_player, evaluator, probe_limit)?;
    let mut children = Vec::with_capacity(width.min(probed));
    for candidate in candidates {
        insert_child(&mut children, candidate, width, maximizing);
    }
    Ok(ChildSelection { children, probed })
}

fn probe_children(
    game: &mut Game, area: PlacementArea, root_player: Player, evaluator: MinEvaluator,
    probe_limit: usize,
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
        candidates.push(SearchChild { action, ordering_utility, ordinal });
    }
    Ok((candidates, probes))
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

fn unique_pool(game: &Game) -> Vec<Piece> {
    let pool = match game.player() {
        Player::Red => game.red_pool(),
        Player::Black => game.black_pool(),
    };
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
    match left.ordering_utility.cmp(&right.ordering_utility) {
        Ordering::Equal => left.ordinal < right.ordinal,
        Ordering::Greater => maximizing,
        Ordering::Less => !maximizing,
    }
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
    ordinal: usize,
}

struct RootResult {
    action: Action,
    ordering_utility: i32,
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
