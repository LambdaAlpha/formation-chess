use std::cmp::Ordering;
use std::num::NonZeroU8;

use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::PositionChange;
use formation_chess_core::action::PositionChanges;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

use super::MIN_TACTICAL_SEARCH_DEPTH;
use super::MIN_TERMINAL_UTILITY;
use super::MinEvaluator;
use super::MinMovementSearchConfig;
use super::evaluator::tactical_piece_units;
use super::outcome::ResolvedActionKind;
use super::outcome::action_move;
use super::outcome::resolved_action_kind_with_destination;
use super::outcome::result_after_changes;
use crate::AgentError;
use crate::ScoredAction;
use crate::legal_movement_actions;

const ROOT_VERIFICATION_WIDTH: usize = 8;
const ROOT_VERIFICATION_BUDGET_DIVISOR: usize = 3;
const ROOT_PRELIMINARY_SEARCH_DEPTH: u8 = 2;

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
            exact: true,
            ordinal,
        });
    }
    if candidates.is_empty() {
        return Err(AgentError::Decision("movement action list is empty".to_owned()));
    }

    candidates.sort_by(root_ordering);
    let max_depth = config.max_depth.get();
    if max_depth > 1 {
        let movement = MovementSearchContext { root_player, evaluator, config };
        let preliminary_context = RootSearchContext {
            movement,
            depth_remaining: max_depth.min(ROOT_PRELIMINARY_SEARCH_DEPTH) - 1,
        };
        let verification_context = RootSearchContext { movement, depth_remaining: max_depth - 1 };
        let mut action_buffer = Vec::with_capacity(128);
        let total_nodes = usize::try_from(config.max_nodes.get())
            .expect("validated Min node budget must fit usize");
        let mut preliminary_nodes = preliminary_root_budget(total_nodes);
        let mut root_alpha = candidates
            .iter()
            .filter(|candidate| candidate.game_result != GameResult::Unfinished)
            .map(|candidate| candidate.utility)
            .max()
            .unwrap_or(-i32::from(MIN_TERMINAL_UTILITY));
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
            let search = search_root_candidate(
                &mut search_game,
                candidate,
                preliminary_context,
                branch_budget,
                SearchWindow { alpha: root_alpha, beta: i32::from(MIN_TERMINAL_UTILITY) },
                &mut action_buffer,
            )?;
            preliminary_nodes -= search.nodes;
            if search.exact && candidate.utility > root_alpha {
                root_alpha = candidate.utility;
            }
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
            let search = search_root_candidate(
                &mut search_game,
                candidate,
                verification_context,
                branch_budget,
                SearchWindow::full(),
                &mut action_buffer,
            )?;
            remaining_nodes -= search.nodes;
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
    window: SearchWindow, action_buffer: &mut Vec<Action>,
) -> Result<SearchResult, AgentError> {
    let reaction = game.action(candidate.action).map_err(|message| {
        AgentError::Decision(format!(
            "supplied Min movement action was rejected during search: {message}"
        ))
    })?;
    let search = search_position(
        game,
        context.movement,
        context.depth_remaining,
        budget,
        window,
        action_buffer,
    );
    game.undo(reaction);
    let search = search?;
    candidate.utility = search.utility;
    candidate.exact = search.exact;
    Ok(search)
}

fn root_search_ordering(left: &RootCandidate, right: &RootCandidate) -> Ordering {
    right
        .utility
        .cmp(&left.utility)
        .then_with(|| right.exact.cmp(&left.exact))
        .then_with(|| right.ordering_utility.cmp(&left.ordering_utility))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn search_position(
    game: &mut Game, context: MovementSearchContext, depth_remaining: u8, budget: usize,
    window: SearchWindow, action_buffer: &mut Vec<Action>,
) -> Result<SearchResult, AgentError> {
    if depth_remaining == 0 || budget == 0 || game.result() != GameResult::Unfinished {
        return Ok(SearchResult {
            utility: context.evaluator.evaluate(game, context.root_player).utility,
            nodes: 0,
            exact: true,
        });
    }

    let maximizing = game.player() == context.root_player;
    let width = if maximizing {
        usize::from(context.config.response_width.get())
    } else {
        usize::from(context.config.opponent_width.get())
    };
    let has_selective_extension = depth_remaining == 1 && !maximizing;
    let candidate_probe_limit =
        probe_limit(budget, width, depth_remaining > 1 || has_selective_extension);
    let selection = select_children(
        game,
        context.root_player,
        context.evaluator,
        width,
        maximizing,
        candidate_probe_limit,
        action_buffer,
    )?;
    if selection.children.is_empty() {
        return Err(AgentError::Decision("recursive Min movement action list is empty".to_owned()));
    }

    let mut nodes = selection.probed;
    let mut alpha = window.alpha;
    let mut beta = window.beta;
    let mut exact = true;
    if let Some(utility) = preferred_terminal_bound(&selection.children, maximizing) {
        return Ok(SearchResult { utility, nodes, exact: true });
    }

    let continuations = if depth_remaining == 1 {
        classify_tactical_continuations(
            game,
            context.root_player,
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
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0, exact: true })
        } else if depth_remaining > 1 {
            search_position(
                game,
                context,
                depth_remaining - 1,
                branch_budget,
                SearchWindow { alpha, beta },
                action_buffer,
            )
        } else if continuation.search {
            search_tactical_responses(
                game,
                context,
                TacticalSearchLimits {
                    width: usize::from(context.config.response_width.get()),
                    depth_remaining: MIN_TACTICAL_SEARCH_DEPTH,
                },
                branch_budget,
                SearchWindow { alpha, beta },
                continuation.recapture_target,
                action_buffer,
            )
        } else {
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0, exact: true })
        };
        game.undo(reaction);
        let child_search = child_search?;
        remaining_nodes -= child_search.nodes;
        nodes += child_search.nodes;
        if !child_search.exact {
            exact = false;
        }
        let child_utility = child_search.utility;
        utility = Some(match utility {
            None => child_utility,
            Some(current) if maximizing => current.max(child_utility),
            Some(current) => current.min(child_utility),
        });
        let node_utility = utility.expect("searched movement child must produce a utility");
        let cutoff = if maximizing {
            alpha = alpha.max(node_utility);
            alpha >= beta
        } else {
            beta = beta.min(node_utility);
            beta <= alpha
        };
        if cutoff {
            return Ok(SearchResult { utility: node_utility, nodes, exact: false });
        }
        if is_preferred_terminal_bound(node_utility, maximizing) {
            break;
        }
    }

    Ok(SearchResult {
        utility: utility.expect("nonempty movement selection must produce a utility"),
        nodes,
        exact,
    })
}
fn search_tactical_responses(
    game: &mut Game, context: MovementSearchContext, limits: TacticalSearchLimits, budget: usize,
    window: SearchWindow, recapture_target: Option<(u8, u8)>, actions: &mut Vec<Action>,
) -> Result<SearchResult, AgentError> {
    if limits.depth_remaining == 0 || budget == 0 || game.result() != GameResult::Unfinished {
        return Ok(SearchResult {
            utility: context.evaluator.evaluate(game, context.root_player).utility,
            nodes: 0,
            exact: true,
        });
    }

    let maximizing = game.player() == context.root_player;
    let candidate_probe_limit = tactical_probe_limit(budget, limits.depth_remaining > 1);
    let selection = select_response_children(
        game,
        context.root_player,
        context.evaluator,
        limits.width,
        candidate_probe_limit,
        recapture_target,
        actions,
    )?;
    if selection.children.is_empty() {
        return Ok(SearchResult {
            utility: context.evaluator.evaluate(game, context.root_player).utility,
            nodes: selection.probed,
            exact: true,
        });
    }

    let mut nodes = selection.probed;
    let mut alpha = window.alpha;
    let mut beta = window.beta;
    let mut exact = true;
    if let Some(utility) = preferred_terminal_bound(&selection.children, maximizing) {
        return Ok(SearchResult { utility, nodes, exact: true });
    }

    let continuations = if limits.depth_remaining > 1 {
        classify_tactical_continuations(
            game,
            context.root_player,
            &selection.children,
            false,
            actions,
        )?
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
            Ok(SearchResult { utility: child.ordering_utility, nodes: 0, exact: true })
        } else {
            search_tactical_responses(
                game,
                context,
                TacticalSearchLimits { depth_remaining: limits.depth_remaining - 1, ..limits },
                branch_budget,
                SearchWindow { alpha, beta },
                continuation.recapture_target,
                actions,
            )
        };
        game.undo(reaction);
        let child_search = child_search?;
        remaining_nodes -= child_search.nodes;
        nodes += child_search.nodes;
        if !child_search.exact {
            exact = false;
        }
        let child_utility = child_search.utility;
        utility = Some(match utility {
            None => child_utility,
            Some(current) if maximizing => current.max(child_utility),
            Some(current) => current.min(child_utility),
        });
        let node_utility = utility.expect("searched tactical child must produce a utility");
        let cutoff = if maximizing {
            alpha = alpha.max(node_utility);
            alpha >= beta
        } else {
            beta = beta.min(node_utility);
            beta <= alpha
        };
        if cutoff {
            return Ok(SearchResult { utility: node_utility, nodes, exact: false });
        }
        if is_preferred_terminal_bound(node_utility, maximizing) {
            break;
        }
    }

    Ok(SearchResult {
        utility: utility.expect("nonempty tactical selection must produce a utility"),
        nodes,
        exact,
    })
}
fn classify_tactical_continuations(
    game: &mut Game, root_player: Player, children: &[SearchChild], require_root_turn: bool,
    actions: &mut Vec<Action>,
) -> Result<Vec<TacticalContinuation>, AgentError> {
    let mut continuations = Vec::with_capacity(children.len());
    for child in children {
        let board_before = game.board().clone();
        let destination_occupied = action_destination_occupied(&board_before, child.action);
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
        let forcing_noncapture =
            has_forcing_noncapture_impact(&board_before, child.action, reaction.changes, kind);
        let search = reaction.game_result == GameResult::Unfinished
            && (!require_root_turn || game.player() == root_player)
            && should_continue_tactical_search(game, kind, forcing_noncapture, actions);
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

    let player = game.player();
    let mut threat_actions = Vec::new();
    let threats = immediate_action_map(game.board(), &mut threat_actions);
    let maximizing = player == root_player;
    let ordinals =
        movement_probe_ordinals(game.board(), player, actions, actions.len(), recapture_target);
    let shortlist_width = ordinals.len().min(quick_shortlist_width(width, probe_limit));
    let shortlist = quick_action_shortlist(
        QuickSelectionContext {
            board: game.board(),
            root_player,
            player,
            recapture_target,
            threats: &threats,
            maximizing,
        },
        actions,
        &ordinals,
        shortlist_width,
    );
    let mut children = Vec::with_capacity(shortlist.len().min(width));
    let mut probed = 0;
    for quick_child in shortlist {
        let action = quick_child.action;
        let reaction = game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min response was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(game, root_player).utility;
        game.undo(reaction);
        probed += 1;
        let child = SearchChild { action, ordering_utility, ordinal: quick_child.ordinal };
        if is_preferred_terminal_bound(ordering_utility, maximizing) {
            return Ok(ChildSelection { children: vec![child], probed });
        }
        insert_child(&mut children, child, width, maximizing);
    }

    Ok(ChildSelection { children, probed })
}

fn quick_shortlist_width(width: usize, probe_limit: usize) -> usize {
    let extra_width = width.div_ceil(2);
    width.saturating_add(extra_width).min(probe_limit).max(1)
}

fn quick_action_shortlist(
    context: QuickSelectionContext<'_>, actions: &[Action], ordinals: &[usize], width: usize,
) -> Vec<SearchChild> {
    if width == 0 {
        return Vec::new();
    }

    let before_position_utility = quick_board_utility(context.board, context.root_player);
    let before_hanging_utility =
        quick_hanging_utility(context.board, context.root_player, context.threats);
    let mut candidates = Vec::with_capacity(ordinals.len());
    let mut preview_board = context.board.clone();
    let mut preview_actions = Vec::new();
    for &ordinal in ordinals {
        let action = actions[ordinal];
        let preview = preview_action(context.board, action);
        let ordering_utility = quick_action_utility(
            context,
            &mut preview_board,
            &mut preview_actions,
            action,
            preview,
            before_position_utility,
            before_hanging_utility,
        );
        let priority = tactical_action_priority_from_preview(
            context.board,
            &mut preview_board,
            context.player,
            action,
            preview,
            context.recapture_target,
            context.threats.side(opponent(context.player)),
        );
        candidates.push(TacticalCandidate {
            child: SearchChild { action, ordering_utility, ordinal },
            priority,
        });
    }
    select_tactical_children(candidates, width, context.maximizing)
}

fn quick_action_utility(
    context: QuickSelectionContext<'_>, preview_board: &mut Board,
    preview_actions: &mut Vec<Action>, action: Action, preview: ActionPreview,
    before_position_utility: i32, before_hanging_utility: i32,
) -> i32 {
    if preview.result != GameResult::Unfinished {
        return result_utility(preview.result, context.root_player);
    }

    apply_position_changes(preview_board, preview.changes.as_slice());
    let after_position_utility = quick_board_utility(preview_board, context.root_player);
    let after_threats = immediate_action_map(preview_board, preview_actions);
    let after_hanging_utility =
        quick_hanging_utility(preview_board, context.root_player, &after_threats);
    restore_position_changes(preview_board, preview.changes.as_slice());

    let position_delta = after_position_utility - before_position_utility;
    let hanging_delta = after_hanging_utility - before_hanging_utility;
    let mut utility = position_delta * 8 + hanging_delta * 4;
    let kind = resolved_action_kind_with_destination(
        action,
        preview.changes,
        action_destination_occupied(context.board, action),
    );
    let action_bias = match kind {
        ResolvedActionKind::Capture => 8,
        ResolvedActionKind::Push | ResolvedActionKind::Pull => 2,
        ResolvedActionKind::QuietMove | ResolvedActionKind::Other => 0,
    };
    if context.player == context.root_player {
        utility += action_bias;
    } else {
        utility -= action_bias;
    }
    utility
}

fn quick_board_utility(board: &Board, perspective: Player) -> i32 {
    let mut utility = 0;
    for (position, _) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        let units = tactical_piece_units(effective);
        if effective.player == perspective {
            utility += units;
        } else {
            utility -= units;
        }
    }
    utility
}

fn quick_hanging_utility(board: &Board, perspective: Player, threats: &ImmediateActionMap) -> i32 {
    let mut favorable = [0; 3];
    let mut unfavorable = [0; 3];
    for attacker in [Player::Red, Player::Black] {
        for &target in &threats.side(attacker).profitable_capture_targets {
            let Some(piece) = board.effective(target) else { continue };
            if piece.player == attacker {
                continue;
            }
            let units = tactical_piece_units(piece);
            if piece.player == perspective {
                insert_quick_pressure(&mut unfavorable, units);
            } else {
                insert_quick_pressure(&mut favorable, units);
            }
        }
    }
    weighted_quick_pressure(favorable) - weighted_quick_pressure(unfavorable)
}

fn insert_quick_pressure(top: &mut [i32; 3], units: i32) {
    if units > top[0] {
        top[2] = top[1];
        top[1] = top[0];
        top[0] = units;
        return;
    }
    if units > top[1] {
        top[2] = top[1];
        top[1] = units;
        return;
    }
    if units > top[2] {
        top[2] = units;
    }
}

fn weighted_quick_pressure(top: [i32; 3]) -> i32 {
    top[0] + top[1] / 2 + top[2] / 4
}

fn result_utility(result: GameResult, player: Player) -> i32 {
    match (result, player) {
        (GameResult::RedWin, Player::Red) | (GameResult::BlackWin, Player::Black) => {
            i32::from(MIN_TERMINAL_UTILITY)
        },
        (GameResult::RedWin, Player::Black) | (GameResult::BlackWin, Player::Red) => {
            -i32::from(MIN_TERMINAL_UTILITY)
        },
        (GameResult::Draw | GameResult::Unfinished, _) => 0,
    }
}

fn tactical_action_priority_from_preview(
    board: &Board, preview_board: &mut Board, player: Player, action: Action,
    preview: ActionPreview, recapture_target: Option<(u8, u8)>, threats: &ImmediateActions,
) -> u8 {
    if let Some(target) = recapture_target
        && let Some(move_) = action_move(action)
        && move_.to == target
    {
        return 3;
    }

    if is_win(preview.result, player) {
        return 3;
    }
    if threats.winning && preview_avoids_immediate_win(preview_board, player, preview) {
        return 2;
    }
    if let Some(move_) = action_move(action)
        && threats.capture_targets.contains(&move_.from)
    {
        return 2;
    }

    let kind = resolved_action_kind_with_destination(
        action,
        preview.changes,
        action_destination_occupied(board, action),
    );
    if has_forcing_noncapture_impact_with_scratch(
        board,
        preview_board,
        action,
        preview.changes,
        kind,
    ) {
        return 2;
    }
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
    game: &Game, kind: ResolvedActionKind, forcing_noncapture: bool, actions: &mut Vec<Action>,
) -> bool {
    if kind == ResolvedActionKind::Capture || forcing_noncapture {
        return true;
    }

    let player = game.player();
    let immediate = immediate_actions(game.board(), player, actions);
    if immediate.winning || !immediate.capture_targets.is_empty() {
        return true;
    }
    immediate_actions(game.board(), opponent(player), actions).winning
}

fn immediate_action_map(board: &Board, actions: &mut Vec<Action>) -> ImmediateActionMap {
    ImmediateActionMap {
        red: immediate_actions(board, Player::Red, actions),
        black: immediate_actions(board, Player::Black, actions),
    }
}

fn immediate_actions(board: &Board, player: Player, actions: &mut Vec<Action>) -> ImmediateActions {
    let mut immediate = ImmediateActions::default();
    for (position, _) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        if !effective.can_controlled_by(player) {
            continue;
        }

        actions.clear();
        board.valid_moves(player, position, actions);
        for &action in actions.iter() {
            let preview = preview_action(board, action);
            if is_win(preview.result, player) {
                immediate.winning = true;
            }
            if !changes_remove_player_piece(preview.changes.as_slice(), opponent(player)) {
                continue;
            }
            let Some(move_) = action_move(action) else { continue };
            if !immediate.capture_targets.contains(&move_.to) {
                immediate.capture_targets.push(move_.to);
            }
            let profitable = is_win(preview.result, player)
                || quick_change_utility(board, preview.changes.as_slice(), player) > 0;
            if profitable && !immediate.profitable_capture_targets.contains(&move_.to) {
                immediate.profitable_capture_targets.push(move_.to);
            }
        }
    }
    immediate
}

fn preview_avoids_immediate_win(
    preview_board: &mut Board, player: Player, preview: ActionPreview,
) -> bool {
    if is_win(preview.result, player) || preview.result == GameResult::Draw {
        return true;
    }
    if preview.result != GameResult::Unfinished {
        return false;
    }

    apply_position_changes(preview_board, preview.changes.as_slice());
    let mut actions = Vec::new();
    let avoids_win = !immediate_actions(preview_board, opponent(player), &mut actions).winning;
    restore_position_changes(preview_board, preview.changes.as_slice());
    avoids_win
}

fn has_forcing_noncapture_impact(
    board: &Board, action: Action, changes: PositionChanges, kind: ResolvedActionKind,
) -> bool {
    let mut preview_board = board.clone();
    has_forcing_noncapture_impact_with_scratch(board, &mut preview_board, action, changes, kind)
}

fn has_forcing_noncapture_impact_with_scratch(
    board: &Board, preview_board: &mut Board, action: Action, changes: PositionChanges,
    kind: ResolvedActionKind,
) -> bool {
    if !matches!(
        kind,
        ResolvedActionKind::QuietMove | ResolvedActionKind::Push | ResolvedActionKind::Pull
    ) {
        return false;
    }
    if displaces_tactical_piece(board, action, changes, kind) {
        return true;
    }
    changes_flip_local_tactical_ability(board, preview_board, changes.as_slice())
}

fn displaces_tactical_piece(
    board: &Board, action: Action, changes: PositionChanges, kind: ResolvedActionKind,
) -> bool {
    if !matches!(kind, ResolvedActionKind::Push | ResolvedActionKind::Pull) {
        return false;
    }
    let Some(move_) = action_move(action) else { return false };
    for change in changes.as_slice() {
        if change.at == move_.from {
            continue;
        }
        let Some(piece) = change.old else { continue };
        let displaced = changes
            .as_slice()
            .iter()
            .any(|candidate| candidate.at != change.at && candidate.new == Some(piece));
        if !displaced {
            continue;
        }
        let effective = board.effective(change.at).unwrap_or(piece);
        if effective.ability.has(Ability::VITAL) || effective.ability.has(Ability::ANY_DISTANCE) {
            return true;
        }
    }
    false
}

fn changes_flip_local_tactical_ability(
    board: &Board, preview_board: &mut Board, changes: &[PositionChange],
) -> bool {
    apply_position_changes(preview_board, changes);
    let mut changed = false;
    'changes: for change in changes {
        for dx in -1i8 ..= 1 {
            for dy in -1i8 ..= 1 {
                let x = change.at.0 as i8 + dx;
                let y = change.at.1 as i8 + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let position = (x as u8, y as u8);
                let Some(piece) = board.get(position) else { continue };
                if preview_board.get(position) != Some(piece) {
                    continue;
                }
                let before = board
                    .effective(position)
                    .expect("stationary piece must remain effective before action");
                let after = preview_board
                    .effective(position)
                    .expect("stationary piece must remain effective after action");
                if tactical_ability_changed(before, after) {
                    changed = true;
                    break 'changes;
                }
            }
        }
    }
    restore_position_changes(preview_board, changes);
    changed
}

fn tactical_ability_changed(before: Piece, after: Piece) -> bool {
    for player in [Player::Red, Player::Black] {
        if before.can_controlled_by(player) != after.can_controlled_by(player) {
            return true;
        }
    }
    for ability in [
        Ability::CAPTURE,
        Ability::CAPTURE_ON_PUSH_BLOCKED,
        Ability::CAPTURE_ON_CAPTURED,
        Ability::CAPTURED_ON_CAPTURE,
        Ability::ANY_DISTANCE,
    ] {
        if before.ability.has(ability) != after.ability.has(ability) {
            return true;
        }
    }
    false
}

fn apply_position_changes(board: &mut Board, changes: &[PositionChange]) {
    for change in changes {
        board[change.at] = change.new;
    }
}

fn restore_position_changes(board: &mut Board, changes: &[PositionChange]) {
    for change in changes {
        board[change.at] = change.old;
    }
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

fn quick_change_utility(board: &Board, changes: &[PositionChange], perspective: Player) -> i32 {
    let mut next_board = board.clone();
    apply_position_changes(&mut next_board, changes);

    let mut matched_new = [false; 3];
    let mut utility = 0;
    for change in changes {
        let Some(old_piece) = change.old else {
            continue;
        };
        let mut matched = false;
        for (index, new_change) in changes.iter().enumerate() {
            if matched_new[index] {
                continue;
            }
            let Some(new_piece) = new_change.new else {
                continue;
            };
            if new_piece.id() != old_piece.id() {
                continue;
            }
            matched_new[index] = true;
            matched = true;
            break;
        }
        if matched {
            continue;
        }
        let Some(piece) = board.effective(change.at) else {
            continue;
        };
        let units = tactical_piece_units(piece);
        if piece.player == perspective {
            utility -= units;
        } else {
            utility += units;
        }
    }

    for (index, change) in changes.iter().enumerate() {
        if matched_new[index] {
            continue;
        }
        if change.new.is_none() {
            continue;
        }
        let Some(piece) = next_board.effective(change.at) else {
            continue;
        };
        let units = tactical_piece_units(piece);
        if piece.player == perspective {
            utility += units;
        } else {
            utility -= units;
        }
    }
    utility
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

    let player = game.player();
    let mut threat_actions = Vec::new();
    let threats = immediate_action_map(game.board(), &mut threat_actions);
    let ordinals = movement_probe_ordinals(game.board(), player, actions, actions.len(), None);
    let shortlist_width = ordinals.len().min(quick_shortlist_width(width, probe_limit));
    let shortlist = quick_action_shortlist(
        QuickSelectionContext {
            board: game.board(),
            root_player,
            player,
            recapture_target: None,
            threats: &threats,
            maximizing,
        },
        actions,
        &ordinals,
        shortlist_width,
    );
    let mut children = Vec::with_capacity(shortlist.len().min(width));
    let mut probed = 0;
    for quick_child in shortlist {
        let action = quick_child.action;
        let reaction = game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min movement was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(game, root_player).utility;
        game.undo(reaction);
        probed += 1;
        let child = SearchChild { action, ordering_utility, ordinal: quick_child.ordinal };
        if is_preferred_terminal_bound(ordering_utility, maximizing) {
            return Ok(ChildSelection { children: vec![child], probed });
        }
        insert_child(&mut children, child, width, maximizing);
    }

    Ok(ChildSelection { children, probed })
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
    movement: MovementSearchContext,
    depth_remaining: u8,
}

#[derive(Copy, Clone)]
struct MovementSearchContext {
    root_player: Player,
    evaluator: MinEvaluator,
    config: MinMovementSearchConfig,
}

#[derive(Copy, Clone)]
struct SearchWindow {
    alpha: i32,
    beta: i32,
}

impl SearchWindow {
    fn full() -> Self {
        Self { alpha: -i32::from(MIN_TERMINAL_UTILITY), beta: i32::from(MIN_TERMINAL_UTILITY) }
    }
}

struct RootCandidate {
    action: Action,
    game_result: GameResult,
    ordering_utility: i32,
    utility: i32,
    exact: bool,
    ordinal: usize,
}

#[derive(Default)]
struct ChildSelection {
    children: Vec<SearchChild>,
    probed: usize,
}

#[derive(Copy, Clone)]
struct QuickSelectionContext<'a> {
    board: &'a Board,
    root_player: Player,
    player: Player,
    recapture_target: Option<(u8, u8)>,
    threats: &'a ImmediateActionMap,
    maximizing: bool,
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
    exact: bool,
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

#[derive(Default)]
struct ImmediateActionMap {
    red: ImmediateActions,
    black: ImmediateActions,
}

impl ImmediateActionMap {
    fn side(&self, player: Player) -> &ImmediateActions {
        match player {
            Player::Red => &self.red,
            Player::Black => &self.black,
        }
    }
}

#[derive(Default)]
struct ImmediateActions {
    capture_targets: Vec<(u8, u8)>,
    profitable_capture_targets: Vec<(u8, u8)>,
    winning: bool,
}

#[derive(Copy, Clone)]
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

    use super::MIN_TERMINAL_UTILITY;
    use super::MovementSearchContext;
    use super::QuickSelectionContext;
    use super::ResolvedActionKind;
    use super::SearchWindow;
    use super::TacticalSearchLimits;
    use super::has_forcing_noncapture_impact;
    use super::immediate_action_map;
    use super::immediate_actions;
    use super::preliminary_root_budget;
    use super::preview_action;
    use super::quick_action_shortlist;
    use super::quick_board_utility;
    use super::search_position;
    use super::search_tactical_responses;
    use super::select_children;
    use super::should_continue_tactical_search;
    use super::tactical_action_priority_from_preview;
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

    fn movement_search_context(root_player: Player) -> MovementSearchContext {
        let config = MinConfig::best();
        let evaluator = MinEvaluator::new(&config).expect("best evaluator");
        MovementSearchContext { root_player, evaluator, config: config.movement_search }
    }

    #[test]
    fn root_search_reserves_one_third_for_verification() {
        assert_eq!(preliminary_root_budget(20_000), 13_334);
        assert_eq!(preliminary_root_budget(3), 2);
        assert_eq!(preliminary_root_budget(1), 1);
    }

    #[test]
    fn root_alpha_cuts_off_minimizing_search() {
        let game = movement_game(&[((0, 0), Piece::RED_GENERAL), ((4, 4), Piece::BLACK_GENERAL)]);
        let context = movement_search_context(Player::Black);
        let mut search_game = game;
        let mut action_buffer = Vec::new();
        let result = search_position(
            &mut search_game,
            context,
            1,
            64,
            SearchWindow { alpha: 9_500, beta: i32::from(MIN_TERMINAL_UTILITY) },
            &mut action_buffer,
        )
        .expect("bounded minimizing search");

        assert!(!result.exact);
        assert!(result.utility <= 9_500);
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
    fn quick_shortlist_width_keeps_a_half_width_reserve() {
        assert_eq!(super::quick_shortlist_width(1, 100), 2);
        assert_eq!(super::quick_shortlist_width(8, 100), 12);
        assert_eq!(super::quick_shortlist_width(12, 10), 10);
    }

    #[test]
    fn quick_shortlist_prefers_higher_value_capture() {
        let mut board = Board::new(7, 7);
        board[(6, 6)] = Some(Piece::RED_GENERAL);
        board[(3, 3)] = Some(Piece::RED_ROOK);
        board[(6, 0)] = Some(Piece::BLACK_GENERAL);
        board[(3, 0)] = Some(Piece::BLACK_ROOK);
        board[(6, 3)] = Some(Piece::BLACK_PAWN);
        let low_value = Action::Capture(Move { from: (3, 3), to: (6, 3) });
        let high_value = Action::Capture(Move { from: (3, 3), to: (3, 0) });
        board.try_capture((3, 3), (6, 3)).unwrap();
        board.try_capture((3, 3), (3, 0)).unwrap();

        let actions = [low_value, high_value];
        let mut action_buffer = Vec::new();
        let threats = immediate_action_map(&board, &mut action_buffer);
        let shortlist = quick_action_shortlist(
            QuickSelectionContext {
                board: &board,
                root_player: Player::Red,
                player: Player::Red,
                recapture_target: None,
                threats: &threats,
                maximizing: true,
            },
            &actions,
            &[0, 1],
            1,
        );

        assert_eq!(shortlist.len(), 1);
        assert_eq!(shortlist[0].action, high_value);
    }

    #[test]
    fn quick_shortlist_rejects_quiet_move_into_capture() {
        let mut board = Board::new(7, 7);
        let mut black_attacker = Piece::BLACK_ROOK;
        black_attacker.ability &= !Ability::CAPTURED;
        black_attacker.ability &= !Ability::CAPTURE_ON_CAPTURED;
        board[(6, 6)] = Some(Piece::RED_GENERAL);
        board[(1, 3)] = Some(Piece::RED_ROOK);
        board[(6, 0)] = Some(Piece::BLACK_GENERAL);
        board[(4, 0)] = Some(black_attacker);
        let hanging = Action::Move(Move { from: (1, 3), to: (4, 3) });
        let safe = Action::Move(Move { from: (1, 3), to: (1, 4) });
        board.try_move((1, 3), (4, 3)).unwrap();
        board.try_move((1, 3), (1, 4)).unwrap();

        let actions = [hanging, safe];
        let mut action_buffer = Vec::new();
        let threats = immediate_action_map(&board, &mut action_buffer);
        let shortlist = quick_action_shortlist(
            QuickSelectionContext {
                board: &board,
                root_player: Player::Red,
                player: Player::Red,
                recapture_target: None,
                threats: &threats,
                maximizing: true,
            },
            &actions,
            &[0, 1],
            1,
        );

        assert_eq!(shortlist.len(), 1);
        assert_eq!(shortlist[0].action, safe);
    }

    #[test]
    fn immediate_pressure_ignores_losing_sacrifice() {
        let mut board = Board::new(7, 7);
        let mut attacker = Piece::RED_ROOK;
        attacker.ability &= !Ability::CAPTURE;
        attacker.ability |= Ability::CAPTURED_ON_CAPTURE
            | Ability::PUSH_ENEMY
            | Ability::PULL_ENEMY
            | Ability::CAPTURE_ON_PUSH_BLOCKED
            | Ability::DIRECTION_DIAGONAL;
        let mut target = Piece::BLACK_PAWN;
        target.ability = Ability::CONTROLLED_BY_BLACK | Ability::CAPTURED;
        board[(6, 6)] = Some(Piece::RED_GENERAL);
        board[(0, 3)] = Some(attacker);
        board[(6, 0)] = Some(Piece::BLACK_GENERAL);
        board[(3, 3)] = Some(target);
        let changes = board.try_capture((0, 3), (3, 3)).expect("sacrifice must be legal");
        let exchange = super::quick_change_utility(&board, changes.as_slice(), Player::Red);
        assert!(
            exchange < 0,
            "exchange={exchange} attacker={} target={} changes={changes:?}",
            super::tactical_piece_units(attacker),
            super::tactical_piece_units(target),
        );

        let mut actions = Vec::new();
        let immediate = immediate_actions(&board, Player::Red, &mut actions);

        assert!(immediate.capture_targets.contains(&(3, 3)));
        assert!(!immediate.profitable_capture_targets.contains(&(3, 3)));
    }

    #[test]
    fn quick_position_score_tracks_formation_control() {
        let mut board = Board::new(5, 5);
        board[(1, 1)] = Some(Piece::RED_STRATAGEM);
        board[(2, 2)] = Some(Piece::BLACK_ROOK);
        let before = quick_board_utility(&board, Player::Red);
        let changes = board.try_move((1, 1), (0, 0)).expect("stratagem move must be legal");
        super::apply_position_changes(&mut board, changes.as_slice());
        let after = quick_board_utility(&board, Player::Red);

        assert!(before > after, "before={before} after={after}");
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
            ResolvedActionKind::QuietMove,
            false,
            &mut actions,
        ));
    }

    #[test]
    fn quiet_leaf_extends_for_available_capture() {
        let game = movement_game(&[
            ((4, 4), Piece::RED_GENERAL),
            ((0, 2), Piece::RED_ROOK),
            ((4, 0), Piece::BLACK_GENERAL),
            ((2, 2), Piece::BLACK_ROOK),
        ]);
        let mut actions = Vec::new();

        assert!(should_continue_tactical_search(
            &game,
            ResolvedActionKind::QuietMove,
            false,
            &mut actions,
        ));
    }

    #[test]
    fn push_displacing_long_range_piece_is_forcing() {
        let mut board = Board::new(5, 5);
        board[(1, 1)] = Some(Piece::RED_GENERAL);
        board[(2, 2)] = Some(Piece::BLACK_ROOK);
        board[(4, 4)] = Some(Piece::BLACK_GENERAL);
        let action = Action::Push(Move { from: (1, 1), to: (2, 2) });
        let preview = preview_action(&board, action);

        assert!(has_forcing_noncapture_impact(
            &board,
            action,
            preview.changes,
            ResolvedActionKind::Push,
        ));
    }

    #[test]
    fn quiet_move_changing_local_control_is_forcing() {
        let mut board = Board::new(5, 5);
        board[(1, 1)] = Some(Piece::RED_STRATAGEM);
        board[(2, 2)] = Some(Piece::BLACK_ROOK);
        let action = Action::Move(Move { from: (1, 1), to: (0, 0) });
        let preview = preview_action(&board, action);

        assert!(
            board
                .effective((2, 2))
                .expect("black rook must be effective")
                .can_controlled_by(Player::Red)
        );
        assert!(has_forcing_noncapture_impact(
            &board,
            action,
            preview.changes,
            ResolvedActionKind::QuietMove,
        ));
    }
    #[test]
    fn quiet_escape_from_immediate_win_gets_tactical_priority() {
        let mut board = Board::new(5, 5);
        board[(2, 2)] = Some(Piece::RED_GENERAL);
        board[(0, 2)] = Some(Piece::BLACK_ROOK);
        board[(4, 4)] = Some(Piece::BLACK_GENERAL);
        let action = Action::Move(Move { from: (2, 2), to: (1, 1) });
        let preview = preview_action(&board, action);
        let mut actions = Vec::new();
        let threats = immediate_actions(&board, Player::Black, &mut actions);

        assert!(threats.winning);
        let mut preview_board = board.clone();
        assert_eq!(
            tactical_action_priority_from_preview(
                &board,
                &mut preview_board,
                Player::Red,
                action,
                preview,
                None,
                &threats,
            ),
            2,
        );
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
        let context = movement_search_context(Player::Red);
        let recapture = Action::Capture(Move { from: (0, 4), to: (3, 4) });
        let mut expected_game = game.clone();
        expected_game.action(recapture).expect("recapture must be legal");
        let expected = context.evaluator.evaluate(&expected_game, Player::Red).utility;

        let mut search_game = game;
        let mut actions = Vec::new();
        let result = search_tactical_responses(
            &mut search_game,
            context,
            TacticalSearchLimits { width: 1, depth_remaining: 1 },
            1,
            SearchWindow::full(),
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
        let context = movement_search_context(Player::Red);
        let mut actions = Vec::new();

        let mut shallow_game = game.clone();
        let shallow = search_tactical_responses(
            &mut shallow_game,
            context,
            TacticalSearchLimits { width: 1, depth_remaining: 1 },
            15,
            SearchWindow::full(),
            Some((2, 2)),
            &mut actions,
        )
        .expect("one-ply tactical search");

        let mut counter_game = game.clone();
        let counter = search_tactical_responses(
            &mut counter_game,
            context,
            TacticalSearchLimits { width: 1, depth_remaining: 2 },
            15,
            SearchWindow::full(),
            Some((2, 2)),
            &mut actions,
        )
        .expect("two-ply tactical search");

        let mut recapture_game = game;
        let recapture = search_tactical_responses(
            &mut recapture_game,
            context,
            TacticalSearchLimits { width: 1, depth_remaining: 3 },
            15,
            SearchWindow::full(),
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
