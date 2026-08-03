use std::cmp::Ordering;
use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Place;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

use super::MIN_TERMINAL_UTILITY;
use super::MinEvaluator;
use super::MinPlacementSearchConfig;
use crate::AgentError;
use crate::PlacementArea;
use crate::ScoredAction;
use crate::placement_area;

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
    let max_depth = config.max_depth.get();
    let max_nodes =
        usize::try_from(config.max_nodes.get()).expect("validated Min node budget must fit usize");
    let root_width = usize::from(config.root_width.get());
    let root_probe_limit = probe_limit(max_nodes, root_width, max_depth > 1);
    let root_selection =
        select_children(game, area, root_player, evaluator, root_width, true, root_probe_limit)?;
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
        let search = if branch_budget > 0 && child.game.phase() == Phase::Place {
            search_position(
                &child.game,
                root_player,
                evaluator,
                config,
                max_depth - 1,
                branch_budget,
            )?
        } else {
            SearchResult { utility: child.ordering_utility, nodes: 0 }
        };
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
    game: &Game, root_player: Player, evaluator: MinEvaluator, config: MinPlacementSearchConfig,
    depth_remaining: u8, budget: usize,
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
        let child_search = if branch_budget > 0 && child.game.phase() == Phase::Place {
            search_position(
                &child.game,
                root_player,
                evaluator,
                config,
                depth_remaining - 1,
                branch_budget,
            )?
        } else {
            SearchResult { utility: child.ordering_utility, nodes: 0 }
        };
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

fn select_children(
    game: &Game, area: PlacementArea, root_player: Player, evaluator: MinEvaluator, width: usize,
    maximizing: bool, probe_limit: usize,
) -> Result<ChildSelection, AgentError> {
    if width == 0 || probe_limit == 0 {
        return Ok(ChildSelection::default());
    }

    let pieces = unique_pool(game);
    let positions = area
        .positions()
        .filter(|position| game.board().get(*position).is_none())
        .collect::<Vec<_>>();
    if pieces.is_empty() || positions.is_empty() {
        return Ok(ChildSelection::default());
    }

    let total = pieces.len().checked_mul(positions.len()).ok_or_else(|| {
        AgentError::Decision("placement candidate count overflowed usize".to_owned())
    })?;
    let probes = total.min(probe_limit);
    let mut children = Vec::with_capacity(width.min(probes));
    for sample_index in 0 .. probes {
        let ordinal = spread_index(sample_index, probes, total);
        let piece = pieces[ordinal / positions.len()];
        let to = positions[ordinal % positions.len()];
        let action = Action::Place(Place { piece: piece.id(), to });
        let mut child_game = game.clone();
        child_game.action(action).map_err(|message| {
            AgentError::Decision(format!("generated Min placement was rejected: {message}"))
        })?;
        let ordering_utility = evaluator.evaluate(&child_game, root_player).utility;
        insert_child(
            &mut children,
            SearchChild { action, game: child_game, ordering_utility, ordinal },
            width,
            maximizing,
        );
    }

    Ok(ChildSelection { children, probed: probes })
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

struct SearchChild {
    action: Action,
    game: Game,
    ordering_utility: i32,
    ordinal: usize,
}

struct RootResult {
    action: Action,
    ordering_utility: i32,
    utility: i32,
    ordinal: usize,
}

struct SearchResult {
    utility: i32,
    nodes: usize,
}
