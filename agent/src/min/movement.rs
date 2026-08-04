use std::cmp::Ordering;
use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

use super::MIN_TERMINAL_UTILITY;
use super::MinEvaluator;
use super::MinMovementSearchConfig;
use crate::AgentError;
use crate::ScoredAction;
use crate::legal_movement_actions;

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
        let mut remaining_nodes = usize::try_from(config.max_nodes.get())
            .expect("validated Min node budget must fit usize");
        let mut branches_left = candidates
            .iter()
            .filter(|candidate| candidate.game_result == GameResult::Unfinished)
            .count();
        for candidate in &mut candidates {
            if candidate.game_result != GameResult::Unfinished {
                continue;
            }

            let branch_budget = fair_share(remaining_nodes, branches_left);
            branches_left -= 1;
            let reaction = search_game.action(candidate.action).map_err(|message| {
                AgentError::Decision(format!(
                    "supplied Min movement action was rejected during search: {message}"
                ))
            })?;
            let search = if branch_budget > 0 {
                search_position(
                    &mut search_game,
                    root_player,
                    evaluator,
                    config,
                    max_depth - 1,
                    branch_budget,
                )
            } else {
                Ok(SearchResult { utility: candidate.ordering_utility, nodes: 0 })
            };
            search_game.undo(reaction);
            let search = search?;
            remaining_nodes -= search.nodes;
            candidate.utility = search.utility;
        }
    }

    candidates.sort_by(|left, right| {
        right
            .utility
            .cmp(&left.utility)
            .then_with(|| right.ordering_utility.cmp(&left.ordering_utility))
            .then_with(|| left.ordinal.cmp(&right.ordinal))
    });
    candidates.truncate(usize::from(top_k.get()));

    Ok(candidates
        .into_iter()
        .map(|candidate| ScoredAction {
            action: candidate.action,
            score: candidate.utility as f32 / f32::from(MIN_TERMINAL_UTILITY),
        })
        .collect())
}

fn search_position(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, config: MinMovementSearchConfig,
    depth_remaining: u8, budget: usize,
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
    let candidate_probe_limit = probe_limit(budget, width, depth_remaining > 1);
    let selection =
        select_children(game, root_player, evaluator, width, maximizing, candidate_probe_limit)?;
    if selection.children.is_empty() {
        return Err(AgentError::Decision("recursive Min movement action list is empty".to_owned()));
    }

    let mut nodes = selection.probed;
    if let Some(utility) = preferred_terminal_bound(&selection.children, maximizing) {
        return Ok(SearchResult { utility, nodes });
    }
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
            AgentError::Decision(format!("generated Min movement was rejected: {message}"))
        })?;
        let child_search = if branch_budget > 0 && reaction.game_result == GameResult::Unfinished {
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

fn select_children(
    game: &mut Game, root_player: Player, evaluator: MinEvaluator, width: usize, maximizing: bool,
    probe_limit: usize,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    let actions = legal_movement_actions(game);
    if actions.is_empty() {
        return Ok(ChildSelection::default());
    }

    let probes = actions.len().min(probe_limit);
    let mut children = Vec::with_capacity(width.min(probes));
    for sample_index in 0 .. probes {
        let ordinal = spread_index(sample_index, probes, actions.len());
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
        .expect("nonempty movement selection must have a preferred utility")
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

fn probe_limit(budget: usize, width: usize, has_deeper_search: bool) -> usize {
    if !has_deeper_search {
        return budget;
    }
    (budget / 2).max(width).min(budget)
}

fn fair_share(remaining: usize, branches_left: usize) -> usize {
    if remaining == 0 { 0 } else { remaining.div_ceil(branches_left) }
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

struct SearchChild {
    action: Action,
    ordering_utility: i32,
    ordinal: usize,
}

struct SearchResult {
    utility: i32,
    nodes: usize,
}
