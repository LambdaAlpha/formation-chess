use std::num::NonZeroU8;

use formation_chess_agent::ActionSelector;
use formation_chess_agent::Agent;
use formation_chess_agent::AgentError;
use formation_chess_agent::AgentInput;
use formation_chess_agent::PreparedInput;
use formation_chess_agent::ScoredAction;
use formation_chess_agent::analyze_agent;
use formation_chess_agent::analyze_prepared;
use formation_chess_agent::legal_movement_actions;
use formation_chess_agent::placement_area;
use formation_chess_agent::play_agent_turn;
use formation_chess_agent::prepare_turn;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Place;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

struct TestAgent {
    placement_candidates: Vec<ScoredAction>,
    movement_candidates: Vec<ScoredAction>,
    placement_calls: usize,
    movement_calls: usize,
    observed_game_pool: Vec<Piece>,
    observed_position_count: usize,
    observed_legal_action_count: usize,
    observed_top_k: Option<NonZeroU8>,
}

impl TestAgent {
    fn new(
        placement_candidates: Vec<ScoredAction>, movement_candidates: Vec<ScoredAction>,
    ) -> Self {
        Self {
            placement_candidates,
            movement_candidates,
            placement_calls: 0,
            movement_calls: 0,
            observed_game_pool: Vec::new(),
            observed_position_count: 0,
            observed_legal_action_count: 0,
            observed_top_k: None,
        }
    }
}

impl Agent for TestAgent {
    fn name(&self) -> &str {
        "test"
    }

    fn analyze(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        self.observed_top_k = Some(top_k);
        match input {
            AgentInput::Placement { area } => {
                self.placement_calls += 1;
                self.observed_game_pool = match game.player() {
                    Player::Red => game.red_pool(),
                    Player::Black => game.black_pool(),
                }
                .to_vec();
                self.observed_position_count = area.positions().count();
                Ok(self.placement_candidates.clone())
            },
            AgentInput::Movement { legal_actions } => {
                self.movement_calls += 1;
                self.observed_legal_action_count = legal_actions.len();
                Ok(self.movement_candidates.clone())
            },
        }
    }
}

fn top_k(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("nonzero top_k")
}

fn scored(action: Action, score: f32) -> ScoredAction {
    ScoredAction { action, score }
}

fn placement_game(width: u8, height: u8) -> Game {
    Game::new(GameConfig {
        player: Player::Red,
        board: Board::new(width, height),
        red_pool: vec![Piece::RED_GENERAL],
        black_pool: vec![Piece::BLACK_GENERAL],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid placement game")
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
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

#[test]
fn standard_red_placement_area_matches_bottom_half() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let area = placement_area(&game).expect("placement area");

    assert_eq!(area.x_range(), 0 .. 9);
    assert_eq!(area.y_range(), 5 .. 10);
    assert_eq!(area.positions().count(), 45);
    assert!(area.contains((0, 5)));
    assert!(area.contains((8, 9)));
    assert!(!area.contains((0, 4)));
    assert!(!area.contains((9, 9)));
}

#[test]
fn black_placement_area_matches_top_half_after_red_places() {
    let mut game = Game::new(GameConfig::default()).expect("standard game");
    game.action(Action::Place(Place { piece: Piece::RED_GENERAL.id(), to: (0, 5) }))
        .expect("red placement");

    let area = placement_area(&game).expect("black placement area");
    assert_eq!(game.player(), Player::Black);
    assert_eq!(area.x_range(), 0 .. 9);
    assert_eq!(area.y_range(), 0 .. 5);
    assert_eq!(area.positions().count(), 45);
}

#[test]
fn odd_height_placement_area_excludes_middle_row() {
    let mut game = placement_game(3, 5);
    let red_area = placement_area(&game).expect("red placement area");
    assert_eq!(red_area.y_range(), 3 .. 5);
    assert!(!red_area.contains((1, 2)));

    game.action(Action::Place(Place { piece: Piece::RED_GENERAL.id(), to: (0, 3) }))
        .expect("red placement");
    let black_area = placement_area(&game).expect("black placement area");
    assert_eq!(black_area.y_range(), 0 .. 2);
    assert!(!black_area.contains((1, 2)));
}

#[test]
fn board_iterator_yields_positions_for_duplicate_pieces() {
    let mut board = Board::new(3, 3);
    board[(2, 0)] = Some(Piece::WHITE);
    board[(0, 2)] = Some(Piece::WHITE);

    assert_eq!(board.iter().collect::<Vec<_>>(), vec![
        ((2, 0), Piece::WHITE),
        ((0, 2), Piece::WHITE)
    ]);
}

#[test]
fn movement_actions_include_pass_but_not_place_or_resign() {
    let game = movement_game();
    let actions = legal_movement_actions(&game);

    assert_eq!(actions.last(), Some(&Action::Pass(Player::Red)));
    assert!(!actions.iter().any(|action| matches!(action, Action::Place(_) | Action::Resign(_))));
}

#[test]
fn every_enumerated_movement_action_is_accepted_by_core() {
    let game = movement_game();
    let actions = legal_movement_actions(&game);

    for action in actions {
        game.try_action(action).expect("enumerated action must be legal");
    }
}

#[test]
fn prepared_placement_turn_exposes_compact_area() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let prepared = prepare_turn(&game).expect("prepared placement turn");

    assert!(std::ptr::eq(prepared.game(), &game));
    assert_eq!(prepared.player(), Player::Red);
    assert_eq!(prepared.phase(), formation_chess_core::game::Phase::Place);
    assert_eq!(prepared.legal_action_count(), None);

    let PreparedInput::Placement { area } = prepared.input() else {
        panic!("expected placement input");
    };
    assert_eq!(area.x_range(), 0 .. 9);
    assert_eq!(area.y_range(), 5 .. 10);
}

#[test]
fn analysis_dispatches_placement_without_action_enumeration() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let before = game.to_string();
    let expected_pool = game.red_pool().to_vec();
    let placement = Action::Place(Place { piece: Piece::RED_GENERAL.id(), to: (0, 5) });
    let mut agent =
        TestAgent::new(vec![scored(placement, 3.0)], vec![scored(Action::Pass(Player::Red), 1.0)]);

    let analysis = analyze_agent(&game, &mut agent, top_k(3)).expect("placement analysis");

    assert_eq!(agent.placement_calls, 1);
    assert_eq!(agent.movement_calls, 0);
    assert_eq!(agent.observed_game_pool, expected_pool);
    assert_eq!(agent.observed_position_count, 45);
    assert_eq!(agent.observed_top_k, Some(top_k(3)));
    assert_eq!(analysis.player, Player::Red);
    assert_eq!(analysis.candidates, vec![scored(placement, 3.0)]);
    assert_eq!(analysis.legal_action_count, None);
    assert_eq!(game.to_string(), before);
}

#[test]
fn analysis_dispatches_prepared_movement_candidates() {
    let game = movement_game();
    let expected_actions = legal_movement_actions(&game);
    let expected_count = expected_actions.len();
    let prepared = prepare_turn(&game).expect("prepared movement turn");
    let PreparedInput::Movement { legal_actions } = prepared.input() else {
        panic!("expected movement input");
    };
    assert_eq!(legal_actions, &expected_actions);
    assert_eq!(prepared.legal_action_count(), Some(expected_count));

    let pass = Action::Pass(Player::Red);
    let mut agent = TestAgent::new(
        vec![scored(Action::Place(Place { piece: Piece::RED_GENERAL.id(), to: (0, 4) }), 1.0)],
        vec![scored(pass, 2.0)],
    );

    let analysis = analyze_prepared(&prepared, &mut agent, top_k(2)).expect("movement analysis");

    assert_eq!(agent.placement_calls, 0);
    assert_eq!(agent.movement_calls, 1);
    assert_eq!(agent.observed_legal_action_count, expected_count);
    assert_eq!(agent.observed_top_k, Some(top_k(2)));
    assert_eq!(analysis.candidates, vec![scored(pass, 2.0)]);
    assert_eq!(analysis.legal_action_count, Some(expected_count));
}

#[test]
fn turn_requests_top_one_and_executes_the_first_candidate() {
    let mut game = movement_game();
    let pass = Action::Pass(Player::Red);
    let mut agent = TestAgent::new(Vec::new(), vec![scored(pass, 7.0)]);
    let mut selector = ActionSelector::default();

    let turn = play_agent_turn(&mut game, &mut agent, &mut selector).expect("agent movement");

    assert_eq!(agent.observed_top_k, Some(NonZeroU8::MIN));
    assert_eq!(turn.player, Player::Red);
    assert_eq!(turn.action, pass);
    assert_eq!(turn.score, 7.0);
    assert_eq!(turn.candidate_rank, NonZeroU8::MIN);
    assert_eq!(game.player(), Player::Black);
}

#[test]
fn analysis_rejects_empty_candidate_list() {
    let game = movement_game();
    let mut agent = TestAgent::new(Vec::new(), Vec::new());

    let error = analyze_agent(&game, &mut agent, top_k(2)).expect_err("empty analysis");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_more_candidates_than_top_k() {
    let game = movement_game();
    let legal_actions = legal_movement_actions(&game);
    let mut agent = TestAgent::new(Vec::new(), vec![
        scored(legal_actions[0], 2.0),
        scored(legal_actions[1], 1.0),
    ]);

    let error = analyze_agent(&game, &mut agent, NonZeroU8::MIN).expect_err("too many candidates");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_non_finite_scores() {
    let game = movement_game();
    let mut agent = TestAgent::new(Vec::new(), vec![scored(Action::Pass(Player::Red), f32::NAN)]);

    let error = analyze_agent(&game, &mut agent, NonZeroU8::MIN).expect_err("non-finite score");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_increasing_scores() {
    let game = movement_game();
    let legal_actions = legal_movement_actions(&game);
    let mut agent = TestAgent::new(Vec::new(), vec![
        scored(legal_actions[0], 1.0),
        scored(legal_actions[1], 2.0),
    ]);

    let error = analyze_agent(&game, &mut agent, top_k(2)).expect_err("unsorted scores");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_duplicate_actions() {
    let game = movement_game();
    let pass = Action::Pass(Player::Red);
    let mut agent = TestAgent::new(Vec::new(), vec![scored(pass, 2.0), scored(pass, 1.0)]);

    let error = analyze_agent(&game, &mut agent, top_k(2)).expect_err("duplicate action");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_movement_action_outside_supplied_list() {
    let game = movement_game();
    let mut agent = TestAgent::new(Vec::new(), vec![scored(Action::Resign(Player::Red), 1.0)]);

    let error = analyze_agent(&game, &mut agent, NonZeroU8::MIN)
        .expect_err("resign is not a movement candidate");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
}

#[test]
fn analysis_rejects_invalid_placement() {
    let game = Game::new(GameConfig::default()).expect("standard game");
    let before = game.to_string();
    let invalid = Action::Place(Place { piece: Piece::BLACK_GENERAL.id(), to: (0, 5) });
    let mut agent = TestAgent::new(vec![scored(invalid, 1.0)], Vec::new());

    let error =
        analyze_agent(&game, &mut agent, NonZeroU8::MIN).expect_err("wrong-color placement");

    assert!(matches!(error, AgentError::InvalidAnalysis(_)));
    assert_eq!(game.to_string(), before);
    assert_eq!(game.player(), Player::Red);
}

#[test]
fn analysis_rejects_finished_game_without_calling_agent() {
    let mut game = movement_game();
    game.action(Action::Resign(Player::Red)).expect("finish game");
    let mut agent = TestAgent::new(
        vec![scored(Action::Place(Place { piece: Piece::BLACK_GENERAL.id(), to: (0, 0) }), 1.0)],
        vec![scored(Action::Pass(Player::Black), 1.0)],
    );

    let prepare_error = prepare_turn(&game).expect_err("finished game cannot be prepared");
    assert!(matches!(prepare_error, AgentError::GameState(_)));

    let error = analyze_agent(&game, &mut agent, NonZeroU8::MIN).expect_err("finished game");

    assert!(matches!(error, AgentError::GameState(_)));
    assert_eq!(agent.placement_calls, 0);
    assert_eq!(agent.movement_calls, 0);
}
