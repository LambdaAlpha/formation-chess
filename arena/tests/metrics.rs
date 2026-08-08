use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::AgentError;
use formation_chess_agent::legal_movement_actions;
use formation_chess_arena::ActionKind;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::CountRatio;
use formation_chess_arena::DistributionMetrics;
use formation_chess_arena::ExecutedAction;
use formation_chess_arena::GameMetrics;
use formation_chess_arena::GameRecord;
use formation_chess_arena::GameRun;
use formation_chess_arena::GameTermination;
use formation_chess_arena::Matchup;
use formation_chess_arena::MetricsError;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::ReplayError;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_arena::TerminationKind;
use formation_chess_arena::record::GameResultRecord;
use formation_chess_arena::record::PhaseRecord;
use formation_chess_arena::record::PlayerRecord;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
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

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn game_plan(mode: ScheduleMode) -> formation_chess_arena::GamePlan {
    let matchup =
        Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup");
    Schedule::new(matchup, mode, 83).next().expect("one game plan")
}

fn placement_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![Piece::BLACK_ROOK],
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
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

fn pull_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    board[(2, 2)] = Some(Piece::RED_WIND);
    board[(2, 3)] = Some(Piece::RED_PAWN);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
    .expect("valid pull game")
}

fn game_record(
    initial_game: Game, actions: Vec<Action>, termination: GameTermination, mode: ScheduleMode,
) -> GameRecord {
    let mut final_game = initial_game.clone();
    let mut executed_actions = Vec::with_capacity(actions.len());
    for action in actions {
        let player = final_game.player();
        let phase = final_game.phase();
        let legal_action_count = match phase {
            Phase::Place => None,
            Phase::Move => Some(collected_movement_actions(&final_game).len()),
        };
        let reaction = final_game.action(action).expect("legal test action");
        executed_actions.push(ExecutedAction {
            player,
            phase,
            action,
            score: 0.5,
            candidate_rank: NonZeroU8::MIN,
            reaction,
            legal_action_count,
        });
    }

    let factory = RandomAgentFactory;
    let descriptor = factory.descriptor();
    let run = GameRun {
        plan: game_plan(mode),
        red_agent: descriptor.clone(),
        black_agent: descriptor,
        initial_game,
        final_game,
        actions: executed_actions,
        termination,
    };
    GameRecord::from_game_run(&run).expect("valid game record")
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("distribution value");
    assert!((actual - expected).abs() < 1e-12, "expected {expected}, got {actual}");
}

#[test]
fn distribution_uses_type_7_interpolation_and_empty_options() {
    assert_eq!(DistributionMetrics::from_counts([]), DistributionMetrics::default());

    let distribution = DistributionMetrics::from_counts([1, 2, 3, 4]);
    assert_eq!(distribution.count, 4);
    assert_eq!(distribution.min, Some(1));
    assert_eq!(distribution.max, Some(4));
    assert_close(distribution.mean, 2.5);
    assert_close(distribution.median, 2.5);
    assert_close(distribution.p25, 1.75);
    assert_close(distribution.p75, 3.25);
    assert_close(distribution.p90, 3.7);
    assert_close(distribution.p95, 3.85);
}

#[test]
fn zero_action_failure_has_empty_action_metrics_and_preserves_final_material() {
    let game = placement_game();
    let record = game_record(
        game,
        Vec::new(),
        GameTermination::AgentFailure {
            player: Player::Red,
            phase: Phase::Place,
            error: AgentError::Decision("test failure".to_owned()),
        },
        ScheduleMode::Fixed { games: nonzero(1) },
    );
    let metrics = GameMetrics::from_record(&record).expect("valid metrics");

    assert_eq!(metrics.result, GameResultRecord::Unfinished);
    assert_eq!(metrics.termination, TerminationKind::AgentFailure {
        player: PlayerRecord::Red,
        phase: PhaseRecord::Placement,
    });
    assert_eq!(metrics.last_action, None);
    assert_eq!(metrics.actions_by_side.red.total_actions, 0);
    assert_eq!(metrics.actions_by_side.black.total_actions, 0);
    assert_eq!(metrics.action_types.total_actions, 0);
    assert_eq!(metrics.action_types.placements, CountRatio { count: 0, ratio: None });
    assert_eq!(metrics.action_types.resignations, CountRatio { count: 0, ratio: None });
    assert_eq!(metrics.legal_movement_actions, DistributionMetrics::default());
    assert_eq!(metrics.reaction_changes.additions, 0);
    assert_eq!(metrics.reaction_changes.removals, 0);
    assert_eq!(metrics.reaction_changes.replacements, 0);
    assert_eq!(metrics.final_material.board_pieces.total, 2);
    assert_eq!(metrics.final_material.board_pieces.red, 1);
    assert_eq!(metrics.final_material.board_pieces.black, 1);
    assert_eq!(metrics.final_material.red_pool_pieces, 1);
    assert_eq!(metrics.final_material.black_pool_pieces, 1);
    assert_eq!(metrics.state_visits.total_visits, 1);
    assert_eq!(metrics.state_visits.unique_states, 1);
    assert_eq!(metrics.state_visits.repeated_visits, 0);
    assert_eq!(metrics.state_visits.unique_state_ratio, 1.0);
}

#[test]
fn metrics_classify_and_count_pull_actions() {
    let record = game_record(
        pull_game(),
        vec![Action::Pull(Move { from: (2, 2), to: (2, 1) })],
        GameTermination::ActionLimit { limit: nonzero(1) },
        ScheduleMode::Fixed { games: nonzero(1) },
    );
    let metrics = GameMetrics::from_record(&record).expect("valid pull metrics");

    let last_action = metrics.last_action.as_ref().expect("last action");
    assert_eq!(last_action.action_kind, ActionKind::Pull);
    assert_eq!(metrics.action_types.pulls, CountRatio { count: 1, ratio: Some(1.0) });
    assert_eq!(metrics.reaction_changes.additions, 1);
    assert_eq!(metrics.reaction_changes.removals, 1);
    assert_eq!(metrics.reaction_changes.replacements, 1);
}

fn mixed_action_record() -> GameRecord {
    let actions = vec![
        Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (1, 3) }),
        Action::Place(Place { piece: Piece::BLACK_ROOK.id(), to: (1, 1) }),
        Action::Move(Move { from: (1, 3), to: (1, 2) }),
        Action::Capture(Move { from: (1, 1), to: (1, 2) }),
        Action::Resign(0, 4),
    ];
    game_record(
        placement_game(),
        actions,
        GameTermination::Completed { result: GameResult::BlackWin },
        ScheduleMode::Paired { pairs: nonzero(1) },
    )
}

#[test]
fn metrics_retain_dimensions_last_action_and_serde_shape() {
    let record = mixed_action_record();
    let metrics = GameMetrics::from_record(&record).expect("valid metrics");

    assert_eq!(metrics.game_id, 0);
    assert_eq!(metrics.pair_id, Some(0));
    assert_eq!(metrics.game_in_pair, Some(0));
    assert_eq!(metrics.red_participant_id, "agent_a");
    assert_eq!(metrics.black_participant_id, "agent_b");
    assert_eq!(metrics.result, GameResultRecord::BlackWin);
    assert_eq!(metrics.termination, TerminationKind::Completed);
    let last_action = metrics.last_action.as_ref().expect("last action");
    assert_eq!(last_action.action_index, 4);
    assert_eq!(last_action.player, PlayerRecord::Red);
    assert_eq!(last_action.phase, PhaseRecord::Movement);
    assert_eq!(last_action.action_kind, ActionKind::Resign);
    assert_eq!(last_action.notation, record.actions[4].notation);

    let json = serde_json::to_string(&metrics).expect("serialize metrics");
    let decoded: GameMetrics = serde_json::from_str(&json).expect("deserialize metrics");
    assert_eq!(decoded, metrics);
}

#[test]
fn metrics_split_side_phases_and_all_action_type_ratios() {
    let record = mixed_action_record();
    let metrics = GameMetrics::from_record(&record).expect("valid metrics");

    assert_eq!(metrics.actions_by_side.red.total_actions, 3);
    assert_eq!(metrics.actions_by_side.red.placement_actions, 1);
    assert_eq!(metrics.actions_by_side.red.movement_actions, 2);
    assert_eq!(metrics.actions_by_side.black.total_actions, 2);
    assert_eq!(metrics.actions_by_side.black.placement_actions, 1);
    assert_eq!(metrics.actions_by_side.black.movement_actions, 1);

    assert_eq!(metrics.action_types.total_actions, 5);
    assert_eq!(metrics.action_types.placements, CountRatio { count: 2, ratio: Some(0.4) });
    assert_eq!(metrics.action_types.moves, CountRatio { count: 1, ratio: Some(0.2) });
    assert_eq!(metrics.action_types.captures, CountRatio { count: 1, ratio: Some(0.2) });
    assert_eq!(metrics.action_types.pushes, CountRatio { count: 0, ratio: Some(0.0) });
    assert_eq!(metrics.action_types.pulls, CountRatio { count: 0, ratio: Some(0.0) });
    assert_eq!(metrics.action_types.draws, CountRatio { count: 0, ratio: Some(0.0) });
    assert_eq!(metrics.action_types.passes, CountRatio { count: 0, ratio: Some(0.0) });
    assert_eq!(metrics.action_types.resignations, CountRatio { count: 1, ratio: Some(0.2) });

    let expected_legal = DistributionMetrics::from_counts(
        record.actions.iter().filter_map(|action| action.legal_action_count),
    );
    assert_eq!(metrics.legal_movement_actions, expected_legal);
    assert_eq!(metrics.legal_movement_actions.count, 3);
}

#[test]
fn metrics_count_reactions_final_material_and_state_visits() {
    let record = mixed_action_record();
    let metrics = GameMetrics::from_record(&record).expect("valid metrics");

    assert_eq!(metrics.reaction_changes.additions, 3);
    assert_eq!(metrics.reaction_changes.removals, 2);
    assert_eq!(metrics.reaction_changes.replacements, 1);

    assert_eq!(metrics.final_material.board_pieces.total, 3);
    assert_eq!(metrics.final_material.board_pieces.red, 1);
    assert_eq!(metrics.final_material.board_pieces.black, 2);
    assert_eq!(metrics.final_material.red_pool_pieces, 0);
    assert_eq!(metrics.final_material.black_pool_pieces, 0);
    assert_eq!(metrics.state_visits.total_visits, 6);
    assert_eq!(metrics.state_visits.unique_states, 6);
    assert_eq!(metrics.state_visits.repeated_visits, 0);
    assert_eq!(metrics.state_visits.unique_state_ratio, 1.0);
}
#[test]
fn repeated_state_visits_count_returns_to_an_earlier_state() {
    let record = game_record(
        movement_game(),
        vec![Action::Pass(Player::Red), Action::Pass(Player::Black)],
        GameTermination::ActionLimit { limit: nonzero(2) },
        ScheduleMode::Fixed { games: nonzero(1) },
    );
    let metrics = GameMetrics::from_record(&record).expect("valid metrics");

    assert_eq!(metrics.termination, TerminationKind::ActionLimit { limit: 2 });
    assert_eq!(metrics.action_types.passes, CountRatio { count: 2, ratio: Some(1.0) });
    assert_eq!(metrics.state_visits.total_visits, 3);
    assert_eq!(metrics.state_visits.unique_states, 2);
    assert_eq!(metrics.state_visits.repeated_visits, 1);
    assert!((metrics.state_visits.unique_state_ratio - 2.0 / 3.0).abs() < 1e-12);
}

#[test]
fn metrics_reject_tampered_records_before_returning_values() {
    let mut record = game_record(
        movement_game(),
        vec![Action::Pass(Player::Red)],
        GameTermination::ActionLimit { limit: nonzero(1) },
        ScheduleMode::Fixed { games: nonzero(1) },
    );
    record.actions[0].state_after_sha256 = "0".repeat(64);

    let error = GameMetrics::from_record(&record).expect_err("tampered record must fail");
    assert!(matches!(
        error,
        MetricsError::Replay(ReplayError::Action { game_id: 0, action_index: 0, .. })
    ));
}
