use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::Agent;
use formation_chess_agent::MinAgent;
use formation_chess_agent::MinConfig;
use formation_chess_agent::MinEvaluator;
use formation_chess_agent::ScoredAction;
use formation_chess_agent::analyze_agent;
use formation_chess_agent::placement_area;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Place;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

fn top_k(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("test top_k must be non-zero")
}

fn search_config(depth: u8, nodes: u32, width: u8) -> MinConfig {
    let mut config = MinConfig::best();
    config.placement_search.max_depth = NonZeroU8::new(depth).expect("test depth must be non-zero");
    config.placement_search.max_nodes =
        NonZeroU32::new(nodes).expect("test node budget must be non-zero");
    config.placement_search.root_width =
        NonZeroU8::new(width).expect("test root width must be non-zero");
    config.placement_search.opponent_width =
        NonZeroU8::new(width).expect("test opponent width must be non-zero");
    config.placement_search.response_width =
        NonZeroU8::new(width).expect("test response width must be non-zero");
    config
}

fn placement_game() -> Game {
    Game::new(GameConfig {
        player: Player::Red,
        board: Board::new(2, 2),
        red_pool: vec![Piece::RED_GENERAL, Piece::RED_ROOK],
        black_pool: vec![Piece::BLACK_GENERAL, Piece::BLACK_ROOK],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid test placement game")
}

fn placements(game: &Game) -> Vec<Action> {
    let area = placement_area(game).expect("placement area");
    let pool = match game.player() {
        Player::Red => game.red_pool(),
        Player::Black => game.black_pool(),
    };
    pool.iter()
        .copied()
        .flat_map(|piece| {
            area.positions()
                .filter(|position| game.board().get(*position).is_none())
                .map(move |to| Action::Place(Place { piece: piece.id(), to }))
        })
        .collect()
}

fn exhaustive_value(
    game: &Game, root_player: Player, evaluator: MinEvaluator, depth_remaining: u8,
) -> i32 {
    if depth_remaining == 0 || game.phase() != Phase::Place {
        return evaluator.evaluate(game, root_player).utility;
    }

    let values = placements(game).into_iter().map(|action| {
        let mut child = game.clone();
        child.action(action).expect("enumerated placement must be legal");
        exhaustive_value(&child, root_player, evaluator, depth_remaining - 1)
    });
    if game.player() == root_player {
        values.max().expect("placement node must have a child")
    } else {
        values.min().expect("placement node must have a child")
    }
}

fn exhaustive_roots(game: &Game, config: &MinConfig) -> Vec<ScoredAction> {
    let evaluator = MinEvaluator::new(config).expect("valid evaluator");
    let root_player = game.player();
    let depth = config.placement_search.max_depth.get();
    let mut roots = placements(game)
        .into_iter()
        .enumerate()
        .map(|(ordinal, action)| {
            let mut child = game.clone();
            child.action(action).expect("enumerated root placement must be legal");
            let static_utility = evaluator.evaluate(&child, root_player).utility;
            let utility = if depth == 1 {
                static_utility
            } else {
                exhaustive_value(&child, root_player, evaluator, depth - 1)
            };
            (ordinal, action, static_utility, utility)
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| {
        right.3.cmp(&left.3).then_with(|| right.2.cmp(&left.2)).then_with(|| left.0.cmp(&right.0))
    });
    roots
        .into_iter()
        .map(|(_, action, _, utility)| ScoredAction { action, score: utility as f32 / 10_000.0 })
        .collect()
}

#[test]
fn one_ply_placement_scores_are_static_leaf_evaluations() {
    let game = placement_game();
    let config = search_config(1, 1_000, 64);
    let expected = exhaustive_roots(&game, &config);
    let mut agent = MinAgent::new(config).expect("valid Min agent");

    let analysis = analyze_agent(&game, &mut agent, top_k(3)).expect("Min placement analysis");

    assert_eq!(analysis.candidates, expected[.. 3]);
}

#[test]
fn two_ply_placement_matches_exhaustive_minimax_when_budget_covers_tree() {
    let game = placement_game();
    let config = search_config(2, 1_000, 64);
    let expected = exhaustive_roots(&game, &config);
    let mut agent = MinAgent::new(config).expect("valid Min agent");

    let analysis = analyze_agent(&game, &mut agent, top_k(4)).expect("Min placement analysis");

    assert_eq!(analysis.candidates, expected);
}

#[test]
fn placement_search_is_deterministic_and_respects_a_tiny_budget() {
    let game = placement_game();
    let config = search_config(2, 1, 64);
    let mut first = MinAgent::new(config.clone()).expect("first Min agent");
    let mut second = MinAgent::new(config).expect("second Min agent");

    let first_analysis = analyze_agent(&game, &mut first, top_k(8)).expect("first analysis");
    let second_analysis = analyze_agent(&game, &mut second, top_k(8)).expect("second analysis");

    assert_eq!(first_analysis.candidates, second_analysis.candidates);
    assert_eq!(first_analysis.candidates.len(), 1);
    game.try_action(first_analysis.candidates[0].action)
        .expect("budget-limited candidate must remain legal");
}

#[test]
fn min_agent_exposes_versioned_name_and_config() {
    let config = search_config(1, 100, 4);
    let mut agent = MinAgent::new(config.clone()).expect("valid Min agent");
    let game = placement_game();

    assert_eq!(agent.name(), "Min best-v1");
    assert_eq!(agent.config(), &config);
    let analysis = analyze_agent(&game, &mut agent, NonZeroU8::MIN).expect("Min analysis");
    assert_eq!(analysis.candidates.len(), 1);
}

#[test]
fn best_agent_analyzes_the_standard_initial_placement() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let mut agent = MinAgent::best();

    let analysis = analyze_agent(&game, &mut agent, top_k(3)).expect("best Min analysis");

    assert_eq!(analysis.candidates.len(), 3);
    for candidate in &analysis.candidates {
        game.try_action(candidate.action).expect("best Min placement candidate must be legal");
    }
}
