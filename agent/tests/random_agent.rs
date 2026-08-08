use std::num::NonZeroU8;

use formation_chess_agent::ActionSelector;
use formation_chess_agent::Agent;
use formation_chess_agent::AgentError;
use formation_chess_agent::AgentInput;
use formation_chess_agent::RandomAgent;
use formation_chess_agent::analyze_agent;
use formation_chess_agent::legal_movement_actions;
use formation_chess_agent::placement_area;
use formation_chess_agent::play_agent_turn;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

fn collected_movement_actions(game: &Game) -> Vec<Action> {
    let mut actions = Vec::new();
    legal_movement_actions(game, &mut actions);
    actions
}

fn top_k(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("nonzero top_k")
}

fn movement_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

fn constrained_placement_game(area_is_full: bool) -> Game {
    let height = if area_is_full { 2 } else { 4 };
    let mut board = Board::new(1, height);
    board[(0, 0)] = Some(Piece::BLACK_PAWN);
    board[(0, height / 2)] = Some(Piece::RED_PAWN);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_GENERAL],
        black_pool: vec![Piece::BLACK_GENERAL],
        result: GameResult::Unfinished,
    })
    .expect("valid constrained placement game")
}

#[test]
fn random_agent_has_stable_name() {
    assert_eq!(RandomAgent::with_seed(1).name(), "Random");
}

#[test]
fn same_seed_reproduces_placement_and_movement_analyses() {
    let placement_game = Game::new(GameConfig::default()).expect("standard game");
    let area = placement_area(&placement_game).expect("placement area");
    let movement_game = movement_game();
    let legal_actions = collected_movement_actions(&movement_game);
    let mut first = RandomAgent::with_seed(42);
    let mut second = RandomAgent::with_seed(42);

    assert_eq!(
        first
            .analyze(&placement_game, AgentInput::Placement { area }, top_k(4))
            .expect("first placement analysis"),
        second
            .analyze(&placement_game, AgentInput::Placement { area }, top_k(4))
            .expect("second placement analysis")
    );
    assert_eq!(
        first
            .analyze(
                &movement_game,
                AgentInput::Movement { legal_actions: &legal_actions },
                top_k(4),
            )
            .expect("first movement analysis"),
        second
            .analyze(
                &movement_game,
                AgentInput::Movement { legal_actions: &legal_actions },
                top_k(4),
            )
            .expect("second movement analysis")
    );
}

#[test]
fn random_agent_avoids_occupied_placement_points() {
    let game = constrained_placement_game(false);
    let area = placement_area(&game).expect("placement area");
    let mut agent = RandomAgent::with_seed(7);

    let candidates = agent
        .analyze(&game, AgentInput::Placement { area }, NonZeroU8::MIN)
        .expect("only legal placement");

    assert_eq!(candidates.len(), 1);
    let Action::Place(placement) = candidates[0].action else {
        panic!("placement analysis returned a movement action");
    };
    assert_eq!(placement.piece, Piece::RED_GENERAL.id());
    assert_eq!(placement.to, (0, 3));
    assert_eq!(candidates[0].score, 0.0);
}

#[test]
fn random_agent_returns_unique_legal_placement_top_k() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let mut agent = RandomAgent::with_seed(19);

    let analysis = analyze_agent(&game, &mut agent, top_k(8)).expect("placement analysis");

    assert_eq!(analysis.candidates.len(), 8);
    for (index, candidate) in analysis.candidates.iter().enumerate() {
        assert_eq!(candidate.score, 0.0);
        assert!(
            !analysis.candidates[.. index]
                .iter()
                .any(|previous| previous.action == candidate.action)
        );
        game.try_action(candidate.action).expect("random placement must be legal");
    }
}

#[test]
fn random_agent_reports_full_placement_area() {
    let game = constrained_placement_game(true);
    let area = placement_area(&game).expect("placement area");
    let mut agent = RandomAgent::with_seed(7);

    let error = agent
        .analyze(&game, AgentInput::Placement { area }, NonZeroU8::MIN)
        .expect_err("area is full");

    assert_eq!(error, AgentError::Decision("placement area has no empty point".to_owned()));
}

#[test]
fn random_agent_reports_empty_movement_list() {
    let game = movement_game();
    let mut agent = RandomAgent::with_seed(7);

    let error = agent
        .analyze(&game, AgentInput::Movement { legal_actions: &[] }, NonZeroU8::MIN)
        .expect_err("empty action list");

    assert_eq!(error, AgentError::Decision("movement action list is empty".to_owned()));
}

#[test]
fn random_agent_returns_unique_legal_movement_top_k() {
    let game = movement_game();
    let legal_actions = collected_movement_actions(&game);
    let mut agent = RandomAgent::with_seed(23);

    let analysis = analyze_agent(&game, &mut agent, top_k(5)).expect("movement analysis");

    assert_eq!(analysis.candidates.len(), 5.min(legal_actions.len()));
    for (index, candidate) in analysis.candidates.iter().enumerate() {
        assert_eq!(candidate.score, 0.0);
        assert!(legal_actions.contains(&candidate.action));
        assert!(
            !analysis.candidates[.. index]
                .iter()
                .any(|previous| previous.action == candidate.action)
        );
    }
}

#[test]
fn random_agent_completes_standard_placement_phase() {
    let mut game = Game::new(GameConfig::default()).expect("standard game");
    let mut agent = RandomAgent::with_seed(20260801);
    let mut selector = ActionSelector::default();

    for _ in 0 .. 32 {
        let player = game.player();
        let pool = match player {
            Player::Red => game.red_pool(),
            Player::Black => game.black_pool(),
        }
        .to_vec();
        let area = placement_area(&game).expect("placement area");
        let empty_positions = area
            .positions()
            .filter(|position| game.board().get(*position).is_none())
            .collect::<Vec<_>>();

        let turn =
            play_agent_turn(&mut game, &mut agent, &mut selector).expect("random placement turn");
        let Action::Place(placement) = turn.action else {
            panic!("placement phase returned a movement action");
        };

        assert_eq!(turn.player, player);
        assert!(pool.iter().any(|piece| piece.id() == placement.piece));
        assert!(empty_positions.contains(&placement.to));
        assert_eq!(turn.score, 0.0);
        assert_eq!(turn.legal_action_count, None);
    }

    assert_eq!(game.phase(), Phase::Move);
    assert!(game.red_pool().is_empty());
    assert!(game.black_pool().is_empty());
    assert_eq!(game.board().iter().count(), 32);
}

#[test]
fn random_movement_turn_uses_supplied_candidates() {
    let mut game = movement_game();
    let candidates = collected_movement_actions(&game);
    let mut agent = RandomAgent::with_seed(11);
    let mut selector = ActionSelector::default();

    let turn = play_agent_turn(&mut game, &mut agent, &mut selector).expect("random movement turn");

    assert!(candidates.contains(&turn.action));
    assert_eq!(turn.score, 0.0);
    assert_eq!(turn.legal_action_count, Some(candidates.len()));
}
