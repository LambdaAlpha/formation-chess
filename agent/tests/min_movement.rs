use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::Agent;
use formation_chess_agent::AgentInput;
use formation_chess_agent::MIN_TERMINAL_UTILITY;
use formation_chess_agent::MinAgent;
use formation_chess_agent::MinConfig;
use formation_chess_agent::MinEvaluator;
use formation_chess_agent::ScoredAction;
use formation_chess_agent::legal_movement_actions;
use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

fn collected_movement_actions(game: &Game) -> Vec<Action> {
    let mut actions = Vec::new();
    legal_movement_actions(game, &mut actions);
    actions
}

fn search_config(depth: u8, nodes: u32, width: u8) -> MinConfig {
    let mut config = MinConfig::best();
    config.movement_search.max_depth = NonZeroU8::new(depth).expect("test depth must be non-zero");
    config.movement_search.max_nodes =
        NonZeroU32::new(nodes).expect("test node budget must be non-zero");
    config.movement_search.opponent_width =
        NonZeroU8::new(width).expect("test opponent width must be non-zero");
    config.movement_search.response_width =
        NonZeroU8::new(width).expect("test response width must be non-zero");
    config
}

fn movement_game(player: Player, pieces: &[((u8, u8), Piece)]) -> Game {
    movement_game_on(5, 5, player, pieces)
}

fn movement_game_on(width: u8, height: u8, player: Player, pieces: &[((u8, u8), Piece)]) -> Game {
    let mut board = Board::new(width, height);
    for &(position, piece) in pieces {
        board[position] = Some(piece);
    }
    Game::new(GameConfig {
        player,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

fn analyze(game: &Game, config: MinConfig, legal_actions: &[Action]) -> Vec<ScoredAction> {
    let mut agent = MinAgent::new(config).expect("valid Min agent");
    agent
        .analyze(game, AgentInput::Movement { legal_actions }, NonZeroU8::MAX)
        .expect("movement analysis")
}

fn exhaustive_value(
    game: &Game, root_player: Player, evaluator: MinEvaluator, depth_remaining: u8,
) -> i32 {
    if depth_remaining == 0 || game.result() != GameResult::Unfinished {
        return evaluator.evaluate(game, root_player).utility;
    }

    let values = collected_movement_actions(game).into_iter().map(|action| {
        let mut child = game.clone();
        child.action(action).expect("enumerated movement must be legal");
        exhaustive_value(&child, root_player, evaluator, depth_remaining - 1)
    });
    if game.player() == root_player {
        values.max().expect("movement node must have a child")
    } else {
        values.min().expect("movement node must have a child")
    }
}

fn exhaustive_roots(game: &Game, config: &MinConfig) -> Vec<ScoredAction> {
    let evaluator = MinEvaluator::new(config).expect("valid evaluator");
    let root_player = game.player();
    let depth = config.movement_search.max_depth.get();
    let mut roots = collected_movement_actions(game)
        .into_iter()
        .enumerate()
        .map(|(ordinal, action)| {
            let mut child = game.clone();
            child.action(action).expect("enumerated root movement must be legal");
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
        .map(|(_, action, _, utility)| ScoredAction {
            action,
            score: utility as f32 / f32::from(MIN_TERMINAL_UTILITY),
        })
        .collect()
}

fn candidate(candidates: &[ScoredAction], action: Action) -> (usize, ScoredAction) {
    candidates
        .iter()
        .copied()
        .enumerate()
        .find(|(_, candidate)| candidate.action == action)
        .expect("expected movement candidate")
}

#[test]
fn one_ply_movement_scores_static_leaves_and_scans_every_unique_root() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((4, 4), Piece::RED_ROOK),
        ((0, 0), Piece::BLACK_GENERAL),
        ((4, 0), Piece::BLACK_ROOK),
    ]);
    let mut legal_actions = collected_movement_actions(&game);
    let config = search_config(1, 1, 64);
    let expected = exhaustive_roots(&game, &config);
    legal_actions.push(legal_actions[0]);

    let candidates = analyze(&game, config, &legal_actions);

    assert_eq!(candidates, expected);
}

#[test]
fn two_and_three_ply_movement_match_exhaustive_minimax_with_full_budget() {
    let game = movement_game_on(3, 3, Player::Red, &[
        ((0, 2), Piece::RED_GENERAL),
        ((2, 2), Piece::RED_ROOK),
        ((0, 0), Piece::BLACK_GENERAL),
        ((2, 0), Piece::BLACK_ROOK),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let mut analyses = Vec::new();

    for depth in [2, 3] {
        let config = search_config(depth, 100_000, 64);
        let expected = exhaustive_roots(&game, &config);
        let candidates = analyze(&game, config, &legal_actions);

        assert_eq!(candidates, expected);
        analyses.push(candidates);
    }

    assert_ne!(analyses[0], analyses[1]);
}

#[test]
fn opponent_reply_turns_an_exposed_general_into_an_exact_loss() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((4, 4), Piece::RED_ROOK),
        ((4, 0), Piece::BLACK_GENERAL),
        ((0, 0), Piece::BLACK_ROOK),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let exposed = Action::Move(Move { from: (0, 4), to: (0, 2) });

    assert!(legal_actions.contains(&exposed));

    let one_ply = analyze(&game, search_config(1, 100_000, 64), &legal_actions);
    let two_ply = analyze(&game, search_config(2, 100_000, 64), &legal_actions);
    let (_, one_ply_candidate) = candidate(&one_ply, exposed);
    let (_, two_ply_candidate) = candidate(&two_ply, exposed);

    assert!(one_ply_candidate.score > -1.0);
    assert_eq!(two_ply_candidate.score, -1.0);
}

#[test]
fn movement_search_is_deterministic_and_scans_all_roots_with_a_tiny_budget() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((4, 4), Piece::RED_ROOK),
        ((0, 0), Piece::BLACK_GENERAL),
        ((4, 0), Piece::BLACK_ROOK),
    ]);
    let mut legal_actions = collected_movement_actions(&game);
    let unique_count = legal_actions.len();
    legal_actions.push(legal_actions[0]);
    let config = search_config(3, 1, 64);

    let first = analyze(&game, config.clone(), &legal_actions);
    let second = analyze(&game, config, &legal_actions);

    assert_eq!(first, second);
    assert_eq!(first.len(), unique_count);
    for candidate in first {
        assert!(legal_actions.contains(&candidate.action));
    }
}

#[test]
fn exact_win_outranks_exact_draw() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((4, 0), Piece::RED_ROOK),
        ((0, 0), Piece::BLACK_GENERAL),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let win = Action::Capture(Move { from: (4, 0), to: (0, 0) });
    let draw = Action::Draw(Move { from: (0, 4), to: (0, 0) });

    assert!(legal_actions.contains(&win));
    assert!(legal_actions.contains(&draw));

    let candidates = analyze(&game, MinConfig::best(), &legal_actions);
    let (win_index, win_candidate) = candidate(&candidates, win);
    let (draw_index, draw_candidate) = candidate(&candidates, draw);

    assert_eq!(win_candidate.score, 1.0);
    assert_eq!(draw_candidate.score, 0.0);
    assert!(win_index < draw_index);
}

#[test]
fn favorable_position_avoids_an_available_draw() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((4, 4), Piece::RED_ROOK),
        ((0, 0), Piece::BLACK_GENERAL),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let draw = Action::Draw(Move { from: (0, 4), to: (0, 0) });
    let candidates = analyze(&game, MinConfig::best(), &legal_actions);
    let (_, draw_candidate) = candidate(&candidates, draw);

    assert_eq!(draw_candidate.score, 0.0);
    assert_ne!(candidates[0].action, draw);
    assert!(candidates[0].score > 0.0);
}

#[test]
fn unfavorable_position_takes_an_available_draw() {
    let game = movement_game(Player::Red, &[
        ((0, 4), Piece::RED_GENERAL),
        ((0, 0), Piece::BLACK_GENERAL),
        ((4, 0), Piece::BLACK_ROOK),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let draw = Action::Draw(Move { from: (0, 4), to: (0, 0) });
    let candidates = analyze(&game, MinConfig::best(), &legal_actions);

    assert_eq!(candidates[0], ScoredAction { action: draw, score: 0.0 });
}

#[test]
fn immediate_loss_receives_exact_negative_score() {
    let mut red_general = Piece::RED_GENERAL;
    red_general.ability.add(Ability::CAPTURE);
    red_general.ability.add(Ability::CAPTURED_ON_CAPTURE);
    let game = movement_game(Player::Red, &[
        ((0, 4), red_general),
        ((0, 2), Piece::BLACK_ROOK),
        ((4, 0), Piece::BLACK_GENERAL),
    ]);
    let legal_actions = collected_movement_actions(&game);
    let loss = Action::Capture(Move { from: (0, 4), to: (0, 2) });

    assert!(legal_actions.contains(&loss));

    let candidates = analyze(&game, MinConfig::best(), &legal_actions);
    let (_, loss_candidate) = candidate(&candidates, loss);

    assert_eq!(loss_candidate.score, -1.0);
    assert_ne!(candidates[0].action, loss);
}
