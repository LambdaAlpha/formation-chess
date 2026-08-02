use std::num::NonZeroU32;

use formation_chess_agent::AgentError;
use formation_chess_agent::legal_movement_actions;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::ExecutedAction;
use formation_chess_arena::GameRecord;
use formation_chess_arena::GameRun;
use formation_chess_arena::GameTermination;
use formation_chess_arena::Matchup;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::ReplayError;
use formation_chess_arena::ReplayVerifier;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_arena::record::ActionData;
use formation_chess_arena::record::AgentErrorRecord;
use formation_chess_arena::record::ColorRecord;
use formation_chess_arena::record::GameResultRecord;
use formation_chess_arena::record::PhaseRecord;
use formation_chess_arena::record::PieceRecord;
use formation_chess_arena::record::PlayerRecord;
use formation_chess_arena::record::PositionRecord;
use formation_chess_arena::record::TerminationRecord;
use formation_chess_arena::record::state_sha256;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn game_plan() -> formation_chess_arena::GamePlan {
    let matchup =
        Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup");
    Schedule::new(matchup, ScheduleMode::Fixed { games: nonzero(1) }, 83)
        .next()
        .expect("one game plan")
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

fn one_action_record(action: Action, termination: GameTermination) -> GameRecord {
    let initial_game = movement_game();
    let player = initial_game.player();
    let phase = initial_game.phase();
    let legal_action_count = Some(legal_movement_actions(&initial_game).len());
    let mut final_game = initial_game.clone();
    let reaction = final_game.action(action).expect("legal test action");
    let factory = RandomAgentFactory;
    let descriptor = factory.descriptor();
    let run = GameRun {
        plan: game_plan(),
        red_agent: descriptor.clone(),
        black_agent: descriptor,
        initial_game,
        final_game,
        actions: vec![ExecutedAction {
            player,
            phase,
            action,
            score: 0.5,
            reaction,
            legal_action_count,
        }],
        termination,
    };
    GameRecord::from_game_run(&run).expect("valid game record")
}

fn movement_limit_record() -> GameRecord {
    one_action_record(Action::Pass(Player::Red), GameTermination::MovementActionLimit {
        limit: nonzero(1),
    })
}

fn completed_record() -> GameRecord {
    one_action_record(Action::Resign(Player::Red), GameTermination::Completed {
        result: GameResult::BlackWin,
    })
}

fn agent_failure_record() -> GameRecord {
    let game = movement_game();
    let factory = RandomAgentFactory;
    let descriptor = factory.descriptor();
    let run = GameRun {
        plan: game_plan(),
        red_agent: descriptor.clone(),
        black_agent: descriptor,
        initial_game: game.clone(),
        final_game: game,
        actions: Vec::new(),
        termination: GameTermination::AgentFailure {
            player: Player::Red,
            phase: Phase::Move,
            error: AgentError::Decision("test failure".to_owned()),
        },
    };
    GameRecord::from_game_run(&run).expect("valid failure record")
}

fn assert_action_error(record: &GameRecord, expected: &str) {
    let Err(ReplayError::Action { game_id: 0, action_index: 0, message }) =
        ReplayVerifier::verify(record)
    else {
        panic!("expected action replay error containing {expected:?}");
    };
    assert!(message.contains(expected), "unexpected action error: {message}");
}

fn assert_game_error(record: &GameRecord, expected: &str) {
    let Err(ReplayError::Game { game_id: 0, message }) = ReplayVerifier::verify(record) else {
        panic!("expected game replay error containing {expected:?}");
    };
    assert!(message.contains(expected), "unexpected game error: {message}");
}

#[test]
fn verifier_accepts_generated_termination_variants() {
    for record in [movement_limit_record(), completed_record(), agent_failure_record()] {
        ReplayVerifier::verify(&record).expect("generated record must replay");
    }
}

#[test]
fn verifier_rejects_action_context_and_invalid_action_data() {
    let valid = movement_limit_record();

    let mut record = valid.clone();
    record.actions[0].action_index = 1;
    assert_action_error(&record, "stored action index is 1");

    let mut record = valid.clone();
    record.actions[0].player = PlayerRecord::Black;
    assert_action_error(&record, "player differs");

    let mut record = valid.clone();
    record.actions[0].phase = PhaseRecord::Placement;
    assert_action_error(&record, "phase differs");

    let mut record = valid.clone();
    record.actions[0].score = f32::INFINITY;
    assert_action_error(&record, "score is not finite");

    let mut record = valid.clone();
    record.actions[0].legal_action_count =
        record.actions[0].legal_action_count.map(|count| count + 1);
    assert_action_error(&record, "legal-action count");

    let mut record = valid.clone();
    record.actions[0].action = ActionData::Place {
        piece: PieceRecord { name: '?', color: ColorRecord::Red },
        to: PositionRecord { x: 0, y: 0 },
    };
    assert_action_error(&record, "unknown piece");

    let mut record = valid;
    record.actions[0].action = ActionData::Move {
        from: PositionRecord { x: u8::MAX, y: u8::MAX },
        to: PositionRecord { x: 0, y: 0 },
    };
    assert_action_error(&record, "action is not legal");
}

#[test]
fn verifier_rejects_notation_reaction_and_action_hash_tampering() {
    let valid = movement_limit_record();

    let mut record = valid.clone();
    record.actions[0].notation.push_str(" tampered");
    assert_action_error(&record, "action notation differs");

    let mut record = valid.clone();
    record.actions[0].reaction.game_result = GameResultRecord::Draw;
    assert_action_error(&record, "reaction differs");

    let mut record = valid.clone();
    record.actions[0].reaction_notation.push_str(" tampered");
    assert_action_error(&record, "reaction notation differs");

    let mut record = valid;
    record.actions[0].state_after_sha256 = "0".repeat(64);
    assert_action_error(&record, "post-action state hash differs");
}

#[test]
fn verifier_rejects_game_state_counts_and_termination_tampering() {
    let valid = movement_limit_record();

    let mut record = valid.clone();
    record.schema_version += 1;
    assert_game_error(&record, "unsupported schema version");

    let mut record = valid.clone();
    record.initial_state_sha256 = "0".repeat(64);
    assert_game_error(&record, "initial state hash is invalid");

    let mut record = valid.clone();
    record.initial_state = "invalid state".to_owned();
    record.initial_state_sha256 = state_sha256(&record.initial_state);
    assert_game_error(&record, "initial state cannot be parsed");

    let mut record = valid.clone();
    record.final_state_sha256 = "0".repeat(64);
    assert_game_error(&record, "final state hash is invalid");

    let mut record = valid.clone();
    record.final_state.clone_from(&record.initial_state);
    record.final_state_sha256 = state_sha256(&record.final_state);
    assert_game_error(&record, "final state differs");

    let mut record = valid.clone();
    record.final_game_result = GameResultRecord::Draw;
    assert_game_error(&record, "final game result differs");

    let mut record = valid.clone();
    record.action_counts.red.total_actions += 1;
    assert_game_error(&record, "action counts differ");

    let mut record = valid;
    record.termination = TerminationRecord::MovementActionLimit { limit: 2 };
    assert_game_error(&record, "movement-limit termination differs");

    let mut record = completed_record();
    record.termination = TerminationRecord::Completed { result: GameResultRecord::RedWin };
    assert_game_error(&record, "completed termination differs");

    let mut record = agent_failure_record();
    record.termination = TerminationRecord::AgentFailure {
        player: PlayerRecord::Black,
        phase: PhaseRecord::Movement,
        error: AgentErrorRecord::Decision("test failure".to_owned()),
    };
    assert_game_error(&record, "agent-failure termination differs");
}
