use std::fs;
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use formation_chess_agent::AgentError;
use formation_chess_agent::legal_movement_actions;
use formation_chess_arena::ANALYSIS_SCHEMA_VERSION;
use formation_chess_arena::AgentDescriptor;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::AnalysisError;
use formation_chess_arena::ArenaManifest;
use formation_chess_arena::DatasetAnalyzer;
use formation_chess_arena::DatasetSummary;
use formation_chess_arena::ExecutedAction;
use formation_chess_arena::GAME_METRICS_FILE_NAME;
use formation_chess_arena::GameRecord;
use formation_chess_arena::GameRun;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::GameTermination;
use formation_chess_arena::JsonlDatasetWriter;
use formation_chess_arena::MatchRunner;
use formation_chess_arena::Matchup;
use formation_chess_arena::MetricsError;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::ReplayError;
use formation_chess_arena::SUMMARY_FILE_NAME;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
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

const PARTICIPANT_A: &str = "agent,\"a";
const PARTICIPANT_B: &str = "agent_b";

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "formation-chess-arena-analysis-{label}-{}-{unique}",
            std::process::id()
        ));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).expect("remove analysis test directory");
        }
    }
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn matchup() -> Matchup {
    Matchup::new(participant(PARTICIPANT_A), participant(PARTICIPANT_B)).expect("distinct matchup")
}

fn placement_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    board[(4, 2)] = Some(Piece::WHITE);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![Piece::BLACK_ROOK],
        white: Piece::WHITE,
        white_pool: 2,
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

fn game_run(
    plan: formation_chess_arena::GamePlan, initial_game: Game, actions: Vec<Action>,
    termination: GameTermination, descriptor: &AgentDescriptor,
) -> GameRun {
    let mut final_game = initial_game.clone();
    let mut executed_actions = Vec::with_capacity(actions.len());
    for action in actions {
        let player = final_game.player();
        let phase = final_game.phase();
        let legal_action_count = match phase {
            Phase::Place => None,
            Phase::Move => Some(legal_movement_actions(&final_game).len()),
        };
        let reaction = final_game.action(action).expect("legal analysis test action");
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
    GameRun {
        plan,
        red_agent: descriptor.clone(),
        black_agent: descriptor.clone(),
        initial_game,
        final_game,
        actions: executed_actions,
        termination,
    }
}

fn create_dataset(label: &str) -> TestDirectory {
    let directory = TestDirectory::new(label);
    let factory = RandomAgentFactory;
    let matchup = matchup();
    let runner =
        MatchRunner::new(matchup.clone(), &factory, &factory, GameRunConfig::new(nonzero(3)));
    let schedule = Schedule::new(matchup, ScheduleMode::Paired { pairs: nonzero(2) }, 97);
    let manifest = ArenaManifest::new(&schedule, &runner).expect("valid analysis manifest");
    let descriptor = factory.descriptor();
    let mut writer =
        JsonlDatasetWriter::create(directory.path(), manifest).expect("create analysis dataset");

    for plan in schedule {
        let run = match plan.game_id {
            0 => game_run(
                plan,
                placement_game(),
                vec![
                    Action::Place(Place { piece: Piece::RED_ROOK, to: (1, 3) }),
                    Action::Place(Place { piece: Piece::BLACK_ROOK, to: (1, 1) }),
                    Action::Move(Move { from: (1, 3), to: (1, 2) }),
                    Action::Capture(Move { from: (1, 1), to: (1, 2) }),
                    Action::Resign(Player::Red),
                ],
                GameTermination::Completed { result: GameResult::BlackWin },
                &descriptor,
            ),
            1 => game_run(
                plan,
                movement_game(),
                vec![Action::Pass(Player::Red), Action::Resign(Player::Black)],
                GameTermination::Completed { result: GameResult::RedWin },
                &descriptor,
            ),
            2 => game_run(
                plan,
                placement_game(),
                Vec::new(),
                GameTermination::AgentFailure {
                    player: Player::Red,
                    phase: Phase::Place,
                    error: AgentError::Decision("failure,\"quoted\"\nline".to_owned()),
                },
                &descriptor,
            ),
            3 => game_run(
                plan,
                movement_game(),
                vec![
                    Action::Pass(Player::Red),
                    Action::Pass(Player::Black),
                    Action::Pass(Player::Red),
                ],
                GameTermination::MovementActionLimit { limit: nonzero(3) },
                &descriptor,
            ),
            game_id => panic!("unexpected game id {game_id}"),
        };
        writer.write_game(&run).expect("write analysis game");
    }
    writer.finish().expect("finish analysis dataset");
    directory
}

fn analyze_dataset(label: &str) -> (TestDirectory, DatasetSummary, String) {
    let directory = create_dataset(label);
    let report = DatasetAnalyzer::analyze(directory.path()).expect("analyze dataset");
    assert_eq!(report.games_analyzed, 4, "analysis must consume every scheduled game");
    let summary: DatasetSummary =
        serde_json::from_slice(&fs::read(&report.summary_path).expect("read summary output"))
            .expect("parse summary output");
    let csv = fs::read_to_string(&report.game_metrics_path).expect("read CSV output");
    (directory, summary, csv)
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("aggregate floating value");
    assert!((actual - expected).abs() < 1e-12, "expected {expected}, got {actual}");
}

fn parse_csv(input: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut characters = input.chars().peekable();
    let mut quoted = false;
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                field.push('"');
                characters.next();
            },
            '"' => quoted = !quoted,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            },
            '\r' if !quoted => {},
            _ => field.push(character),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

#[test]
fn analyzer_writes_deterministic_flat_csv_and_refuses_overwrite() {
    let (directory, _, csv) = analyze_dataset("csv-one");
    let rows = parse_csv(&csv);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].len(), 61);
    assert!(
        rows.iter().all(|row| row.len() == rows[0].len()),
        "every CSV record must match the header width"
    );
    assert_eq!(rows[0][0], "game_id");
    assert_eq!(rows[0][60], "state_unique_ratio");
    assert_eq!(rows[1][3], PARTICIPANT_A);
    assert_eq!(rows[1][6], "completed");
    assert_eq!(rows[3][6], "agent_failure");
    assert_eq!(rows[3][8], "red");
    assert_eq!(rows[3][9], "placement");
    assert_eq!(rows[3][10], "");
    assert!(csv.contains("\"agent,\"\"a\""), "CSV must quote commas and double quotes");

    let second_directory = create_dataset("csv-two");
    DatasetAnalyzer::analyze(second_directory.path()).expect("analyze second dataset");
    assert_eq!(
        csv,
        fs::read_to_string(second_directory.path().join(GAME_METRICS_FILE_NAME))
            .expect("read second CSV")
    );
    assert_eq!(
        fs::read_to_string(directory.path().join(SUMMARY_FILE_NAME)).expect("read first summary"),
        fs::read_to_string(second_directory.path().join(SUMMARY_FILE_NAME))
            .expect("read second summary")
    );

    let error = DatasetAnalyzer::analyze(directory.path()).expect_err("outputs must not overwrite");
    assert!(
        matches!(error, AnalysisError::OutputExists(path) if path.ends_with(GAME_METRICS_FILE_NAME))
    );
}

#[test]
fn summary_aggregates_results_terminations_and_participant_seats() {
    let (_, summary, _) = analyze_dataset("outcomes");

    assert_eq!(summary.schema_version, ANALYSIS_SCHEMA_VERSION);
    assert_eq!(summary.games, 4);
    assert_eq!(summary.results.red_wins.count, 1);
    assert_eq!(summary.results.black_wins.count, 1);
    assert_eq!(summary.results.draws.count, 0);
    assert_eq!(summary.results.unfinished.count, 2);
    assert_close(summary.results.unfinished.ratio, 0.5);
    assert_eq!(summary.terminations.completed.count, 2);
    assert_eq!(summary.terminations.movement_action_limits.count, 1);
    assert_eq!(summary.terminations.agent_failures.count, 1);

    assert_eq!(summary.participant_a.participant_id, PARTICIPANT_A);
    assert_eq!(summary.participant_a.overall.games, 4);
    assert_eq!(summary.participant_a.overall.wins.count, 0);
    assert_eq!(summary.participant_a.overall.losses.count, 2);
    assert_eq!(summary.participant_a.overall.unfinished.count, 2);
    assert_eq!(summary.participant_a.as_red.games, 2);
    assert_eq!(summary.participant_a.as_red.losses.count, 1);
    assert_eq!(summary.participant_a.as_black.losses.count, 1);
    assert_eq!(summary.participant_a.agent_failures.count, 1);

    assert_eq!(summary.participant_b.participant_id, PARTICIPANT_B);
    assert_eq!(summary.participant_b.overall.wins.count, 2);
    assert_eq!(summary.participant_b.overall.losses.count, 0);
    assert_eq!(summary.participant_b.as_red.wins.count, 1);
    assert_eq!(summary.participant_b.as_black.wins.count, 1);
}

#[test]
fn summary_aggregates_action_distributions_and_type_ratios() {
    let (_, summary, _) = analyze_dataset("actions");
    let actions = summary.actions;

    assert_eq!(actions.total_actions, 10);
    assert_eq!(actions.all_per_game.total_actions.count, 4);
    assert_eq!(actions.all_per_game.total_actions.min, Some(0));
    assert_eq!(actions.all_per_game.total_actions.max, Some(5));
    assert_close(actions.all_per_game.total_actions.mean, 2.5);
    assert_close(actions.all_per_game.total_actions.median, 2.5);
    assert_close(actions.all_per_game.total_actions.p25, 1.5);
    assert_close(actions.all_per_game.total_actions.p75, 3.5);
    assert_close(actions.all_per_game.placement_actions.mean, 0.5);
    assert_close(actions.all_per_game.movement_actions.mean, 2.0);
    assert_close(actions.red_per_game.total_actions.mean, 1.5);
    assert_close(actions.black_per_game.total_actions.mean, 1.0);

    assert_eq!(actions.action_types.placements.count, 2);
    assert_eq!(actions.action_types.moves.count, 1);
    assert_eq!(actions.action_types.captures.count, 1);
    assert_eq!(actions.action_types.passes.count, 4);
    assert_eq!(actions.action_types.resignations.count, 2);
    assert_close(actions.action_types.placements.ratio, 0.2);
    assert_close(actions.action_types.passes.ratio, 0.4);
    assert_eq!(actions.legal_movement_actions.count, 8);
}

#[test]
fn summary_aggregates_reactions_material_and_repeated_states() {
    let (_, summary, _) = analyze_dataset("positions");

    assert_eq!(summary.reactions.totals.additions, 3);
    assert_eq!(summary.reactions.totals.removals, 2);
    assert_eq!(summary.reactions.totals.replacements, 1);
    assert_close(summary.reactions.additions_per_game.mean, 0.75);
    assert_eq!(summary.final_material.board_total.min, Some(2));
    assert_eq!(summary.final_material.board_total.max, Some(4));
    assert_close(summary.final_material.board_total.mean, 2.75);
    assert_close(summary.final_material.board_red.mean, 1.0);
    assert_close(summary.final_material.board_black.mean, 1.25);
    assert_close(summary.final_material.board_white.mean, 0.5);
    assert_close(summary.final_material.red_pool.mean, 0.25);
    assert_close(summary.final_material.black_pool.mean, 0.25);
    assert_close(summary.final_material.white_pool.mean, 1.25);

    assert_eq!(summary.states.totals.total_visits, 14);
    assert_eq!(summary.states.totals.unique_states, 12);
    assert_eq!(summary.states.totals.repeated_visits, 2);
    assert_close(summary.states.totals.unique_state_ratio, 6.0 / 7.0);
    assert_close(summary.states.total_visits_per_game.mean, 3.5);
    assert_close(summary.states.unique_states_per_game.mean, 3.0);
    assert_close(summary.states.repeated_visits_per_game.mean, 0.5);
    assert_eq!(summary.states.unique_state_ratio_per_game.min, Some(0.5));
    assert_eq!(summary.states.unique_state_ratio_per_game.max, Some(1.0));
    assert_close(summary.states.unique_state_ratio_per_game.mean, 0.875);
}

#[test]
fn analyzer_rejects_tampered_games_without_leaving_outputs() {
    let directory = create_dataset("tampered");
    let games_path = directory.path().join(formation_chess_arena::GAMES_FILE_NAME);
    let mut lines = fs::read_to_string(&games_path)
        .expect("read games")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut record: GameRecord = serde_json::from_str(&lines[0]).expect("parse first game");
    record.actions[0].state_after_sha256 = "0".repeat(64);
    lines[0] = serde_json::to_string(&record).expect("serialize tampered game");
    fs::write(&games_path, format!("{}\n", lines.join("\n"))).expect("write tampered games");

    let error = DatasetAnalyzer::analyze(directory.path()).expect_err("tampered game must fail");
    assert!(matches!(
        error,
        AnalysisError::Metrics(MetricsError::Replay(ReplayError::Action {
            game_id: 0,
            action_index: 0,
            ..
        }))
    ));
    assert!(!directory.path().join(GAME_METRICS_FILE_NAME).exists());
    assert!(!directory.path().join(SUMMARY_FILE_NAME).exists());
    assert!(
        fs::read_dir(directory.path()).expect("list dataset").all(|entry| !entry
            .expect("dataset entry")
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")),
        "failed analysis must remove temporary outputs"
    );
}

#[test]
fn analyzer_rejects_participant_seats_that_differ_from_manifest() {
    let directory = create_dataset("seats");
    let games_path = directory.path().join(formation_chess_arena::GAMES_FILE_NAME);
    let mut lines = fs::read_to_string(&games_path)
        .expect("read games")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut record: GameRecord = serde_json::from_str(&lines[0]).expect("parse first game");
    record.red.participant_id = PARTICIPANT_B.to_owned();
    lines[0] = serde_json::to_string(&record).expect("serialize invalid seats");
    fs::write(&games_path, format!("{}\n", lines.join("\n"))).expect("write invalid seats");

    let error = DatasetAnalyzer::analyze(directory.path()).expect_err("invalid seats must fail");
    assert!(matches!(error, AnalysisError::InvalidRecord { game_id: 0, .. }));
    assert!(!directory.path().join(GAME_METRICS_FILE_NAME).exists());
    assert!(!directory.path().join(SUMMARY_FILE_NAME).exists());
}
