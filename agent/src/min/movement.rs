use std::cmp::Ordering;
use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::PositionChange;
use formation_chess_core::action::PositionChanges;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

use super::MIN_TACTICAL_SEARCH_DEPTH;
use super::MIN_TERMINAL_UTILITY;
use super::MinEvaluator;
use super::MinMovementSearchConfig;
use super::outcome::ResolvedActionKind;
use super::outcome::action_move;
use super::outcome::resolved_action_kind_with_destination;
use super::outcome::result_after_changes;
use crate::AgentError;
use crate::ScoredAction;
use crate::legal_movement_actions;

const ROOT_VERIFICATION_WIDTH: usize = 4;
const ROOT_VERIFICATION_BUDGET_DIVISOR: usize = 3;

pub(super) fn analyze_movements(
    game: &Game, legal_actions: &[Action], top_k: NonZeroU8, config: MinMovementSearchConfig,
    evaluator: MinEvaluator,
) -> Result<Vec<ScoredAction>, AgentError> {
    if game.phase() != Phase::Move || game.result() != GameResult::Unfinished {
        return Err(AgentError::Decision(
            "Min movement search requires an unfinished movement position".to_owned(),
        ));
    }
    if legal_actions.is_empty() {
        return Err(AgentError::Decision("movement action list is empty".to_owned()));
    }

    let root_player = game.player();
    let mut search_game = game.clone();
    let mut candidates = Vec::with_capacity(legal_actions.len());
    for (ordinal, &action) in legal_actions.iter().enumerate() {
        if candidates.iter().any(|candidate: &RootCandidate| candidate.action == action) {
            continue;
        }

        let reaction = search_game.action(action).map_err(|message| {
            AgentError::Decision(format!("supplied Min movement action was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(&search_game, root_player).utility;
        let game_result = reaction.game_result;
        search_game.undo(reaction);
        candidates.push(RootCandidate {
            action,
            game_result,
            ordering_utility,
            utility: ordering_utility,
            ordinal,
        });
    }
    if candidates.is_empty() {
        return Err(AgentError::Decision("movement action list is empty".to_owned()));
    }

    candidates.sort_by(root_ordering);
    let max_depth = config.max_depth.get();
    if max_depth > 1 {
        let context =
            RootSearchContext { root_player, evaluator, config, depth_remaining: max_depth - 1 };
        let mut action_buffer = Vec::with_capacity(128);
        let total_nodes = usize::try_from(config.max_nodes.get())
            .expect("validated Min node budget must fit usize");
        let mut preliminary_nodes = preliminary_root_budget(total_nodes);
        let mut branches_left = candidates
            .iter()
            .filter(|candidate| candidate.game_result == GameResult::Unfinished)
            .count();
        for candidate in &mut candidates {
            if candidate.game_result != GameResult::Unfinished {
                continue;
            }

            let branch_budget = fair_share(preliminary_nodes, branches_left);
            branches_left -= 1;
            if branch_budget == 0 {
                continue;
            }
            let nodes = search_root_candidate(
                &mut search_game,
                candidate,
                context,
                branch_budget,
                &mut action_buffer,
            )?;
            preliminary_nodes -= nodes;
        }

        candidates.sort_by(root_search_ordering);
        let mut remaining_nodes =
            total_nodes - preliminary_root_budget(total_nodes) + preliminary_nodes;
        let verification_count = candidates
            .iter()
            .filter(|candidate| candidate.game_result == GameResult::Unfinished)
            .take(ROOT_VERIFICATION_WIDTH)
            .count();
        let mut branches_left = verification_count;
        for candidate in candidates
            .iter_mut()
            .filter(|candidate| candidate.game_result == GameResult::Unfinished)
            .take(ROOT_VERIFICATION_WIDTH)
        {
            if remaining_nodes == 0 {
                break;
            }
            let branch_budget = fair_share(remaining_nodes, branches_left);
            branches_left -= 1;
            let nodes = search_root_candidate(
                &mut search_game,
                candidate,
                context,
                branch_budget,
                &mut action_buffer,
            )?;
            remaining_nodes -= nodes;
        }
    }

    candidates.sort_by(root_search_ordering);
    candidates.truncate(usize::from(top_k.get()));

    Ok(candidates
        .into_iter()
        .map(|candidate| ScoredAction {
            action: candidate.action,
            score: candidate.utility as f32 / f32::from(MIN_TERMINAL_UTILITY),
        })
        .collect())
}

fn preliminary_root_budget(total_nodes: usize) -> usize {
    total_nodes - total_nodes / ROOT_VERIFICATION_BUDGET_DIVISOR
}

fn search_root_candidate(
    game: &mut Game, candidate: &mut RootCandidate, context: RootSearchContext, budget: usize,
    action_buffer: &mut Vec<Action>,
) -> Result<usize, AgentError> {
    let reaction = game.action(candidate.action).map_err(|message| {
        AgentError::Decision(format!(
            "supplied Min movement action was rejected during search: {message}"
        ))
    })?;
    let search = search_position(
        game,
        context.root_player,
        context.evaluator,
        context.config,
        context.depth_remaining,
        budget,
        action_buffer,
    );
    game.undo(reaction);
    let search = search?;
    candidate.utility = search.utility;
    Ok(search.nodes)
}

fn root_search_ordering(left: &RootCandidate, right: &RootCandidate) -> Ordering {
    right
        .utility
        .cmp(&left.utility)
        .then_with(|| right.ordering_utility.cmp(&left.ordering_utility))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn search_position(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, config: MinMovementSearchConfig,
    depth_remaining: u8, budget: usize, action_buffer: &mut Vec<Action>,
) -> Result<SearchResult, AgentError> {
    if depth_remaining == 0 || budget == 0 || game.result() != GameResult::Unfinished {
        return Ok(SearchResult {
            utility: evaluator.evaluate(game, root_player).utility,
            nodes: 0,
        });
    }

    let maximizing = game.player() == root_player;
    let width = if maximizing {
        usize::from(config.response_width.get())
    } else {
        usize::from(config.opponent_width.get())
    };
    let has_selective_extension = depth_remaining == 1 && !maximizing;
    let candidate_probe_limit =
        probe_limit(budget, width, depth_remaining > 1 || has_selective_extension);
    let selection = select_children(
        game,
        root_player,
        evaluator,
        width,
        maximizing,
        candidate_probe_limit,
        action_buffer,
    )?;
    if selection.children.is_empty() {
        return Err(AgentError::Decision("recursive Min movement action list is empty".to_owned()));
    }

    let mut nodes = selection.probed;
    if let Some(utility) = preferred_terminal_bound(&selection.children, maximizing) {
        return Ok(SearchResult { utility, nodes });
    }

    let continuations = if depth_remaining == 1 {
        classify_tactical_continuations(
            game,
            root_player,
            &selection.children,
            true,
            action_buffer,
        )?
    } else {
        vec![TacticalContinuation::default(); selection.children.len()]
    };
    let mut continuation_branches_left =
        continuations.iter().filter(|continuation| continuation.search).count();
    let mut remaining_nodes = budget - nodes;
    let child_count = selection.children.len();
    let mut utility: Option<i32> = None;
    for (index, child) in selection.children.into_iter().enumerate() {
        let continuation = continuations[index];
        let branch_budget = if depth_remaining > 1 {
            fair_share(remaining_nodes, child_count - index)
        } else if continuation.search {
            let branch_budget = fair_share(remaining_nodes, continuation_branches_left);
            continuation_branches_left -= 1;
            branch_budget
        } else {
            0
        };
        let reaction = game.action(child.action).map_err(|message| {
            AgentError::Decision(format!("generated Min movement was rejected: {message}"))
        })?;
        let child_search = if reaction.game_result != GameResult::Unfinished || branch_budget == 0 {
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0 })
        } else if depth_remaining > 1 {
            search_position(
                game,
                root_player,
                evaluator,
                config,
                depth_remaining - 1,
                branch_budget,
                action_buffer,
            )
        } else if continuation.search {
            search_tactical_responses(
                game,
                root_player,
                evaluator,
                TacticalSearchLimits {
                    width: usize::from(config.response_width.get()),
                    depth_remaining: MIN_TACTICAL_SEARCH_DEPTH,
                },
                branch_budget,
                continuation.recapture_target,
                action_buffer,
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
        if is_preferred_terminal_bound(
            utility.expect("searched movement child must produce a utility"),
            maximizing,
        ) {
            break;
        }
    }

    Ok(SearchResult {
        utility: utility.expect("nonempty movement selection must produce a utility"),
        nodes,
    })
}
fn search_tactical_responses(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, limits: TacticalSearchLimits,
    budget: usize, recapture_target: Option<(u8, u8)>, actions: &mut Vec<Action>,
) -> Result<SearchResult, AgentError> {
    if limits.depth_remaining == 0 || budget == 0 || game.result() != GameResult::Unfinished {
        return Ok(SearchResult {
            utility: evaluator.evaluate(game, root_player).utility,
            nodes: 0,
        });
    }

    let maximizing = game.player() == root_player;
    let candidate_probe_limit = tactical_probe_limit(budget, limits.depth_remaining > 1);
    let selection = select_response_children(
        game,
        root_player,
        evaluator,
        limits.width,
        candidate_probe_limit,
        recapture_target,
        actions,
    )?;
    if selection.children.is_empty() {
        return Ok(SearchResult {
            utility: evaluator.evaluate(game, root_player).utility,
            nodes: selection.probed,
        });
    }

    let mut nodes = selection.probed;
    if let Some(utility) = preferred_terminal_bound(&selection.children, maximizing) {
        return Ok(SearchResult { utility, nodes });
    }

    let continuations = if limits.depth_remaining > 1 {
        classify_tactical_continuations(game, root_player, &selection.children, false, actions)?
    } else {
        vec![TacticalContinuation::default(); selection.children.len()]
    };
    let mut continuation_branches_left =
        continuations.iter().filter(|continuation| continuation.search).count();
    let mut remaining_nodes = budget - nodes;
    let mut utility: Option<i32> = None;
    for (index, child) in selection.children.into_iter().enumerate() {
        let continuation = continuations[index];
        let branch_budget = if continuation.search {
            let branch_budget = fair_share(remaining_nodes, continuation_branches_left);
            continuation_branches_left -= 1;
            branch_budget
        } else {
            0
        };
        let reaction = game.action(child.action).map_err(|message| {
            AgentError::Decision(format!("generated Min tactical response was rejected: {message}"))
        })?;
        let child_search = if reaction.game_result != GameResult::Unfinished
            || branch_budget == 0
            || !continuation.search
        {
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0 })
        } else {
            search_tactical_responses(
                game,
                root_player,
                evaluator,
                TacticalSearchLimits { depth_remaining: limits.depth_remaining - 1, ..limits },
                branch_budget,
                continuation.recapture_target,
                actions,
            )
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
        if is_preferred_terminal_bound(
            utility.expect("searched tactical child must produce a utility"),
            maximizing,
        ) {
            break;
        }
    }

    Ok(SearchResult {
        utility: utility.expect("nonempty tactical selection must produce a utility"),
        nodes,
    })
}

fn classify_tactical_continuations(
    game: &mut Game, root_player: Player, children: &[SearchChild], require_root_turn: bool,
    actions: &mut Vec<Action>,
) -> Result<Vec<TacticalContinuation>, AgentError> {
    let mut continuations = Vec::with_capacity(children.len());
    for child in children {
        let destination_occupied = action_destination_occupied(game.board(), child.action);
        let reaction = game.action(child.action).map_err(|message| {
            AgentError::Decision(format!(
                "generated Min tactical candidate was rejected: {message}"
            ))
        })?;
        let kind = resolved_action_kind_with_destination(
            child.action,
            reaction.changes,
            destination_occupied,
        );
        let search = reaction.game_result == GameResult::Unfinished
            && (!require_root_turn || game.player() == root_player)
            && should_continue_tactical_search(game, root_player, kind, actions);
        let recapture_target =
            if search { recapture_target(game.board(), child.action, kind) } else { None };
        game.undo(reaction);
        continuations.push(TacticalContinuation { search, recapture_target });
    }
    Ok(continuations)
}
fn select_response_children(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, width: usize,
    probe_limit: usize, recapture_target: Option<(u8, u8)>, actions: &mut Vec<Action>,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    actions.clear();
    legal_movement_actions(game, actions);
    if actions.is_empty() {
        return Ok(ChildSelection::default());
    }

    let maximizing = game.player() == root_player;
    let probes = actions.len().min(probe_limit);
    let ordinals =
        movement_probe_ordinals(game.board(), game.player(), actions, probes, recapture_target);
    let mut candidates = Vec::with_capacity(probes);
    for (sample_index, ordinal) in ordinals.into_iter().enumerate() {
        let action = actions[ordinal];
        let tactical_priority =
            tactical_action_priority(game.board(), game.player(), action, recapture_target);
        let reaction = game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min response was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(game, root_player).utility;
        game.undo(reaction);
        let child = SearchChild { action, ordering_utility, ordinal };
        if is_preferred_terminal_bound(ordering_utility, maximizing) {
            return Ok(ChildSelection { children: vec![child], probed: sample_index + 1 });
        }
        candidates.push(TacticalCandidate { child, priority: tactical_priority });
    }

    let children = select_tactical_children(candidates, width, maximizing);
    Ok(ChildSelection { children, probed: probes })
}

fn tactical_action_priority(
    board: &Board, player: Player, action: Action, recapture_target: Option<(u8, u8)>,
) -> u8 {
    if let Some(target) = recapture_target
        && let Some(move_) = action_move(action)
        && move_.to == target
    {
        return 2;
    }

    let preview = preview_action(board, action);
    if changes_remove_player_piece(preview.changes.as_slice(), opponent(player)) {
        return 1;
    }
    0
}

fn select_tactical_children(
    candidates: Vec<TacticalCandidate>, width: usize, maximizing: bool,
) -> Vec<SearchChild> {
    let tactical_quota = width.div_ceil(2);
    let mut reserved = Vec::with_capacity(tactical_quota);
    for &candidate in &candidates {
        if candidate.priority == 0 {
            continue;
        }
        insert_tactical_candidate(&mut reserved, candidate, tactical_quota, maximizing);
    }

    let filler_width = width - reserved.len();
    let mut fillers = Vec::with_capacity(filler_width);
    for candidate in candidates {
        if reserved.iter().any(|selected| selected.child.action == candidate.child.action) {
            continue;
        }
        insert_child(&mut fillers, candidate.child, filler_width, maximizing);
    }

    let mut children = Vec::with_capacity(reserved.len() + fillers.len());
    for candidate in reserved {
        children.push(candidate.child);
    }
    children.extend(fillers);
    children.sort_by(|left, right| child_ordering(left, right, maximizing));
    children
}

fn insert_tactical_candidate(
    candidates: &mut Vec<TacticalCandidate>, candidate: TacticalCandidate, width: usize,
    maximizing: bool,
) {
    if width == 0 {
        return;
    }
    let index = candidates
        .iter()
        .position(|existing| tactical_candidate_precedes(&candidate, existing, maximizing))
        .unwrap_or(candidates.len());
    candidates.insert(index, candidate);
    if candidates.len() > width {
        candidates.pop();
    }
}

fn tactical_candidate_precedes(
    left: &TacticalCandidate, right: &TacticalCandidate, maximizing: bool,
) -> bool {
    match left.priority.cmp(&right.priority) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => child_precedes(&left.child, &right.child, maximizing),
    }
}
fn movement_probe_ordinals(
    board: &Board, player: Player, actions: &[Action], probes: usize,
    preferred_target: Option<(u8, u8)>,
) -> Vec<usize> {
    let probes = probes.min(actions.len());
    if probes == 0 {
        return Vec::new();
    }

    let mut capture_ordinals = Vec::new();
    for (ordinal, &action) in actions.iter().enumerate() {
        let preview = preview_action(board, action);
        if is_win(preview.result, player) {
            return vec![ordinal];
        }
        if changes_remove_player_piece(preview.changes.as_slice(), opponent(player)) {
            capture_ordinals.push(ordinal);
        }
    }

    let mut ordinals = Vec::with_capacity(probes);
    let mut selected = vec![false; actions.len()];
    if let Some(target) = preferred_target {
        let mut preferred_ordinals = Vec::new();
        for (ordinal, &action) in actions.iter().enumerate() {
            let Some(move_) = action_move(action) else {
                continue;
            };
            if move_.to == target {
                preferred_ordinals.push(ordinal);
            }
        }
        append_spread_ordinals(&mut ordinals, &mut selected, &preferred_ordinals, probes);
    }

    let mut remaining_captures = Vec::with_capacity(capture_ordinals.len());
    for ordinal in capture_ordinals {
        if !selected[ordinal] {
            remaining_captures.push(ordinal);
        }
    }
    append_spread_ordinals(&mut ordinals, &mut selected, &remaining_captures, probes);

    let mut remaining = Vec::with_capacity(actions.len() - ordinals.len());
    for (ordinal, &was_selected) in selected.iter().enumerate() {
        if !was_selected {
            remaining.push(ordinal);
        }
    }
    append_spread_ordinals(&mut ordinals, &mut selected, &remaining, probes);
    ordinals
}

fn append_spread_ordinals(
    ordinals: &mut Vec<usize>, selected: &mut [bool], candidates: &[usize], limit: usize,
) {
    let available = limit.saturating_sub(ordinals.len());
    let sample_count = available.min(candidates.len());
    for sample_index in 0 .. sample_count {
        let index = spread_index(sample_index, sample_count, candidates.len());
        let ordinal = candidates[index];
        ordinals.push(ordinal);
        selected[ordinal] = true;
    }
}

fn action_destination_occupied(board: &Board, action: Action) -> bool {
    let Some(move_) = action_move(action) else {
        return false;
    };
    board.get(move_.to).is_some()
}

fn recapture_target(board: &Board, action: Action, kind: ResolvedActionKind) -> Option<(u8, u8)> {
    if kind != ResolvedActionKind::Capture {
        return None;
    }
    let move_ = action_move(action)?;
    board.get(move_.to)?;
    Some(move_.to)
}

fn should_continue_tactical_search(
    game: &Game, root_player: Player, kind: ResolvedActionKind, actions: &mut Vec<Action>,
) -> bool {
    if kind == ResolvedActionKind::Capture {
        return true;
    }
    if has_immediate_win(game.board(), root_player, actions) {
        return true;
    }
    has_immediate_win(game.board(), opponent(root_player), actions)
}

fn has_immediate_win(board: &Board, player: Player, actions: &mut Vec<Action>) -> bool {
    for (position, _) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        if !effective.can_controlled_by(player) {
            continue;
        }
        actions.clear();
        board.valid_moves(player, position, actions);
        for &action in actions.iter() {
            if is_win(preview_action(board, action).result, player) {
                return true;
            }
        }
    }
    false
}

fn preview_action(board: &Board, action: Action) -> ActionPreview {
    let changes = match action {
        Action::Move(move_) => {
            board.try_move(move_.from, move_.to).expect("enumerated move must remain valid")
        },
        Action::Capture(move_) => {
            board.try_capture(move_.from, move_.to).expect("enumerated capture must remain valid")
        },
        Action::Push(move_) => {
            board.try_push(move_.from, move_.to).expect("enumerated push must remain valid")
        },
        Action::Pull(move_) => {
            board.try_pull(move_.from, move_.to).expect("enumerated pull must remain valid")
        },
        Action::Draw(_) => {
            return ActionPreview { changes: PositionChanges::empty(), result: GameResult::Draw };
        },
        Action::Resign(x, y) => {
            let piece = board
                .effective((x, y))
                .expect("enumerated resignation must retain its vital piece");
            let result = match piece.player {
                Player::Red => GameResult::BlackWin,
                Player::Black => GameResult::RedWin,
            };
            return ActionPreview { changes: PositionChanges::empty(), result };
        },
        Action::Place(_) => unreachable!("movement analysis cannot contain placement"),
    };
    ActionPreview { changes, result: result_after_changes(changes.as_slice()) }
}

fn changes_remove_player_piece(changes: &[PositionChange], player: Player) -> bool {
    let mut old_count = 0;
    let mut new_count = 0;
    for change in changes {
        if let Some(piece) = change.old
            && piece.player == player
        {
            old_count += 1;
        }
        if let Some(piece) = change.new
            && piece.player == player
        {
            new_count += 1;
        }
    }
    new_count < old_count
}

fn is_win(result: GameResult, player: Player) -> bool {
    matches!(
        (result, player),
        (GameResult::RedWin, Player::Red) | (GameResult::BlackWin, Player::Black)
    )
}

fn opponent(player: Player) -> Player {
    match player {
        Player::Red => Player::Black,
        Player::Black => Player::Red,
    }
}

fn select_children(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, width: usize, maximizing: bool,
    probe_limit: usize, actions: &mut Vec<Action>,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    actions.clear();
    legal_movement_actions(game, actions);
    if actions.is_empty() {
        return Ok(ChildSelection::default());
    }

    let probes = actions.len().min(probe_limit);
    let ordinals = movement_probe_ordinals(game.board(), game.player(), actions, probes, None);
    let mut children = Vec::with_capacity(width.min(probes));
    for (sample_index, ordinal) in ordinals.into_iter().enumerate() {
        let action = actions[ordinal];
        let reaction = game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min movement was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(game, root_player).utility;
        game.undo(reaction);
        let child = SearchChild { action, ordering_utility, ordinal };
        if is_preferred_terminal_bound(ordering_utility, maximizing) {
            return Ok(ChildSelection { children: vec![child], probed: sample_index + 1 });
        }
        insert_child(&mut children, child, width, maximizing);
    }

    Ok(ChildSelection { children, probed: probes })
}

fn root_ordering(left: &RootCandidate, right: &RootCandidate) -> Ordering {
    right
        .ordering_utility
        .cmp(&left.ordering_utility)
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn spread_index(sample_index: usize, sample_count: usize, total: usize) -> usize {
    let numerator = sample_index as u128 * total as u128;
    usize::try_from(numerator / sample_count as u128)
        .expect("sampled movement index must fit usize")
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
    child_ordering(left, right, maximizing) == Ordering::Less
}

fn child_ordering(left: &SearchChild, right: &SearchChild, maximizing: bool) -> Ordering {
    match left.ordering_utility.cmp(&right.ordering_utility) {
        Ordering::Equal => left.ordinal.cmp(&right.ordinal),
        Ordering::Greater if maximizing => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
        Ordering::Less if maximizing => Ordering::Greater,
        Ordering::Less => Ordering::Less,
    }
}

fn preferred_terminal_bound(children: &[SearchChild], maximizing: bool) -> Option<i32> {
    let bound =
        if maximizing { i32::from(MIN_TERMINAL_UTILITY) } else { -i32::from(MIN_TERMINAL_UTILITY) };
    children.iter().any(|child| child.ordering_utility == bound).then_some(bound)
}

fn is_preferred_terminal_bound(utility: i32, maximizing: bool) -> bool {
    if maximizing {
        utility == i32::from(MIN_TERMINAL_UTILITY)
    } else {
        utility == -i32::from(MIN_TERMINAL_UTILITY)
    }
}

fn tactical_probe_limit(budget: usize, has_deeper_search: bool) -> usize {
    if !has_deeper_search {
        return budget;
    }
    budget.div_ceil(2)
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

#[derive(Copy, Clone)]
struct RootSearchContext {
    root_player: Player,
    evaluator: MinEvaluator,
    config: MinMovementSearchConfig,
    depth_remaining: u8,
}

struct RootCandidate {
    action: Action,
    game_result: GameResult,
    ordering_utility: i32,
    utility: i32,
    ordinal: usize,
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

#[derive(Copy, Clone)]
struct TacticalCandidate {
    child: SearchChild,
    priority: u8,
}

struct SearchResult {
    utility: i32,
    nodes: usize,
}

#[derive(Copy, Clone)]
struct TacticalSearchLimits {
    width: usize,
    depth_remaining: u8,
}

#[derive(Copy, Clone, Default)]
struct TacticalContinuation {
    search: bool,
    recapture_target: Option<(u8, u8)>,
}

struct ActionPreview {
    changes: PositionChanges,
    result: GameResult,
}

#[cfg(test)]
mod tests {
    use formation_chess_core::ability::Ability;
    use formation_chess_core::action::Action;
    use formation_chess_core::action::GameResult;
    use formation_chess_core::action::Move;
    use formation_chess_core::board::Board;
    use formation_chess_core::game::Game;
    use formation_chess_core::game::GameConfig;
    use formation_chess_core::piece::Piece;
    use formation_chess_core::piece::Player;

    use super::ResolvedActionKind;
    use super::TacticalSearchLimits;
    use super::preliminary_root_budget;
    use super::search_tactical_responses;
    use super::select_children;
    use super::should_continue_tactical_search;
    use crate::MinConfig;
    use crate::MinEvaluator;
    use crate::legal_movement_actions;

    fn movement_game(pieces: &[((u8, u8), Piece)]) -> Game {
        let mut board = Board::new(5, 5);
        for &(position, piece) in pieces {
            board[position] = Some(piece);
        }
        Game::new(GameConfig {
            player: Player::Red,
            board,
            red_pool: Vec::new(),
            black_pool: Vec::new(),
            result: GameResult::Unfinished,
        })
        .expect("valid movement position")
    }

    #[test]
    fn root_search_reserves_one_third_for_verification() {
        assert_eq!(preliminary_root_budget(20_000), 13_334);
        assert_eq!(preliminary_root_budget(3), 2);
        assert_eq!(preliminary_root_budget(1), 1);
    }

    #[test]
    fn internal_probe_keeps_immediate_win_outside_spread_sample() {
        let game = movement_game(&[
            ((4, 4), Piece::RED_GENERAL),
            ((0, 0), Piece::RED_ROOK),
            ((4, 0), Piece::BLACK_GENERAL),
        ]);
        let win = Action::Capture(Move { from: (0, 0), to: (4, 0) });
        let mut legal_actions = Vec::new();
        legal_movement_actions(&game, &mut legal_actions);
        let win_ordinal = legal_actions
            .iter()
            .position(|&action| action == win)
            .expect("winning capture must be legal");
        assert_ne!(win_ordinal, 0, "single spread probe must otherwise miss the win");

        let evaluator = MinEvaluator::new(&MinConfig::best()).expect("best evaluator");
        let mut search_game = game;
        let mut action_buffer = Vec::new();
        let selection = select_children(
            &mut search_game,
            Player::Red,
            evaluator,
            1,
            true,
            1,
            &mut action_buffer,
        )
        .expect("internal child selection");

        assert_eq!(selection.probed, 1);
        assert_eq!(selection.children.len(), 1);
        assert_eq!(selection.children[0].action, win);
    }

    #[test]
    fn internal_probe_keeps_push_escalation_that_really_captures() {
        let mut attacker = Piece::RED_ROOK;
        attacker.ability &= !Ability::CAPTURE;
        attacker.ability |= Ability::PUSH_ENEMY | Ability::CAPTURE_ON_PUSH_BLOCKED;
        let mut target = Piece::BLACK_ROOK;
        target.ability |= Ability::PUSHED_BY_ENEMY;
        let game = movement_game(&[
            ((0, 4), Piece::RED_GENERAL),
            ((3, 2), attacker),
            ((0, 0), Piece::BLACK_GENERAL),
            ((4, 2), target),
        ]);
        let capture = Action::Push(Move { from: (3, 2), to: (4, 2) });
        let mut legal_actions = Vec::new();
        legal_movement_actions(&game, &mut legal_actions);
        let capture_ordinal = legal_actions
            .iter()
            .position(|&action| action == capture)
            .expect("escalating push must be legal");
        assert_ne!(capture_ordinal, 0, "single spread probe must otherwise miss the capture");

        let evaluator = MinEvaluator::new(&MinConfig::best()).expect("best evaluator");
        let mut search_game = game;
        let mut action_buffer = Vec::new();
        let selection = select_children(
            &mut search_game,
            Player::Red,
            evaluator,
            1,
            true,
            1,
            &mut action_buffer,
        )
        .expect("internal child selection");

        assert_eq!(selection.probed, 1);
        assert_eq!(selection.children.len(), 1);
        assert_eq!(selection.children[0].action, capture);
    }

    #[test]
    fn quiet_leaf_extends_for_side_to_move_immediate_win() {
        let game = movement_game(&[
            ((4, 4), Piece::RED_GENERAL),
            ((0, 0), Piece::RED_ROOK),
            ((4, 0), Piece::BLACK_GENERAL),
        ]);
        let mut actions = Vec::new();

        assert!(should_continue_tactical_search(
            &game,
            Player::Red,
            ResolvedActionKind::QuietMove,
            &mut actions,
        ));
    }

    #[test]
    fn tactical_response_prioritizes_same_point_recapture() {
        let mut board = Board::new(7, 7);
        board[(6, 6)] = Some(Piece::RED_GENERAL);
        board[(6, 0)] = Some(Piece::BLACK_GENERAL);
        board[(0, 4)] = Some(Piece::RED_ROOK);
        board[(3, 4)] = Some(Piece::BLACK_ROOK);
        let game = Game::new(GameConfig {
            player: Player::Red,
            board,
            red_pool: Vec::new(),
            black_pool: Vec::new(),
            result: GameResult::Unfinished,
        })
        .expect("valid recapture position");
        let evaluator = MinEvaluator::new(&MinConfig::best()).expect("best evaluator");
        let recapture = Action::Capture(Move { from: (0, 4), to: (3, 4) });
        let mut expected_game = game.clone();
        expected_game.action(recapture).expect("recapture must be legal");
        let expected = evaluator.evaluate(&expected_game, Player::Red).utility;

        let mut search_game = game;
        let mut actions = Vec::new();
        let result = search_tactical_responses(
            &mut search_game,
            Player::Red,
            evaluator,
            TacticalSearchLimits { width: 1, depth_remaining: 1 },
            1,
            Some((3, 4)),
            &mut actions,
        )
        .expect("tactical response search");

        assert_eq!(result.nodes, 1);
        assert_eq!(result.utility, expected);
    }

    #[test]
    fn recursive_tactical_search_extends_beyond_static_exchange_chain() {
        let game = movement_game(&[
            ((0, 4), Piece::RED_GENERAL),
            ((1, 2), Piece::RED_ROOK),
            ((3, 2), Piece::RED_ROOK),
            ((4, 0), Piece::BLACK_GENERAL),
            ((2, 1), Piece::BLACK_ROOK),
            ((2, 2), Piece::BLACK_ROOK),
        ]);
        let evaluator = MinEvaluator::new(&MinConfig::best()).expect("best evaluator");
        let mut actions = Vec::new();

        let mut shallow_game = game.clone();
        let shallow = search_tactical_responses(
            &mut shallow_game,
            Player::Red,
            evaluator,
            TacticalSearchLimits { width: 1, depth_remaining: 1 },
            15,
            Some((2, 2)),
            &mut actions,
        )
        .expect("one-ply tactical search");

        let mut counter_game = game.clone();
        let counter = search_tactical_responses(
            &mut counter_game,
            Player::Red,
            evaluator,
            TacticalSearchLimits { width: 1, depth_remaining: 2 },
            15,
            Some((2, 2)),
            &mut actions,
        )
        .expect("two-ply tactical search");

        let mut recapture_game = game;
        let recapture = search_tactical_responses(
            &mut recapture_game,
            Player::Red,
            evaluator,
            TacticalSearchLimits { width: 1, depth_remaining: 3 },
            15,
            Some((2, 2)),
            &mut actions,
        )
        .expect("three-ply tactical search");

        assert!(
            counter.utility < shallow.utility,
            "counter={} shallow={}",
            counter.utility,
            shallow.utility,
        );
        assert_ne!(
            recapture.utility, counter.utility,
            "the third tactical ply must change the evaluated continuation",
        );
        assert!(recapture.nodes > shallow.nodes);
    }
}
