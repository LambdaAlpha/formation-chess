use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use formation_chess_agent::Agent;
use formation_chess_agent::AgentError;
use formation_chess_agent::AgentInput;
use formation_chess_agent::ScoredAction;
use formation_chess_arena::AgentDescriptor;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::ArenaManifest;
use formation_chess_arena::DatasetError;
use formation_chess_arena::GAMES_FILE_NAME;
use formation_chess_arena::GameRecord;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::JsonlDatasetReader;
use formation_chess_arena::JsonlDatasetWriter;
use formation_chess_arena::MANIFEST_FILE_NAME;
use formation_chess_arena::MatchRunner;
use formation_chess_arena::Matchup;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RECORD_SCHEMA_VERSION;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::STATE_HASH_ALGORITHM;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_arena::record::AgentErrorRecord;
use formation_chess_arena::record::PhaseRecord;
use formation_chess_arena::record::TerminationRecord;
use formation_chess_arena::record::state_sha256;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("formation-chess-arena-{label}-{}-{unique}", std::process::id()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).expect("remove test dataset directory");
        }
    }
}

struct FailingFactory {
    message: &'static str,
}

struct FailingAgent {
    message: &'static str,
}

impl Agent for FailingAgent {
    fn name(&self) -> &str {
        "failing"
    }

    fn analyze(
        &mut self, _game: &Game, _input: AgentInput<'_>, _top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        Err(AgentError::Decision(self.message.to_owned()))
    }
}

impl AgentFactory for FailingFactory {
    fn descriptor(&self) -> AgentDescriptor {
        AgentDescriptor {
            kind: "failing".to_owned(),
            display_name: "Failing".to_owned(),
            implementation_version: "test".to_owned(),
            parameters: BTreeMap::new(),
        }
    }

    fn create(&self, _seed: u64) -> Box<dyn Agent> {
        Box::new(FailingAgent { message: self.message })
    }
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn matchup() -> Matchup {
    Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup")
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

fn read_dataset(path: &Path) -> (Vec<u8>, Vec<u8>) {
    let manifest = fs::read(path.join("manifest.json")).expect("read manifest");
    let games = fs::read(path.join("games.jsonl")).expect("read games");
    (manifest, games)
}

#[test]
fn game_record_replays_actions_and_splits_counts_by_side() {
    let matchup = matchup();
    let mut schedule =
        Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 37);
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(33)));
    let manifest = ArenaManifest::new(&schedule, &runner).expect("matching manifest");
    let plan = schedule.next().expect("first fixed game");
    let run = runner.run(plan, Game::default()).expect("valid game plan");

    let record = GameRecord::from_game_run(&run).expect("consistent game run");

    assert_eq!(manifest.schema_version, RECORD_SCHEMA_VERSION);
    assert_eq!(manifest.state_hash_algorithm, STATE_HASH_ALGORITHM);
    assert_eq!(manifest.total_games(), 1);
    let manifest_json = serde_json::to_string(&manifest).expect("serialize manifest");
    let decoded_manifest: ArenaManifest =
        serde_json::from_str(&manifest_json).expect("deserialize manifest");
    assert_eq!(decoded_manifest, manifest);
    assert_eq!(record.schema_version, RECORD_SCHEMA_VERSION);
    assert_eq!(record.initial_state_sha256, state_sha256(&record.initial_state));
    assert_eq!(record.final_state_sha256, state_sha256(&record.final_state));
    assert_eq!(record.actions.len(), 33);
    assert_eq!(record.action_counts.red.placement_phase_actions, 16);
    assert_eq!(record.action_counts.black.placement_phase_actions, 16);
    assert_eq!(record.action_counts.red.movement_phase_actions, 1);
    assert_eq!(record.action_counts.black.movement_phase_actions, 0);
    assert!(record.actions[.. 32].iter().all(|action| {
        action.phase == PhaseRecord::Placement && action.legal_action_count.is_none()
    }));
    assert_eq!(record.actions[32].phase, PhaseRecord::Movement);
    assert!(record.actions[32].legal_action_count.is_some());
    assert_eq!(record.actions[32].state_after_sha256, record.final_state_sha256);
    assert!(record.actions.iter().all(|action| !action.notation.is_empty()));

    let encoded = serde_json::to_string(&record).expect("serialize game record");
    let decoded: GameRecord = serde_json::from_str(&encoded).expect("deserialize game record");
    assert_eq!(decoded, record);
}

#[test]
fn writer_is_deterministic_and_json_escapes_agent_errors() {
    const ERROR_MESSAGE: &str = "invalid \"action\"\nsecond line";
    let matchup = matchup();
    let mut schedule =
        Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 43);
    let participant_a = FailingFactory { message: ERROR_MESSAGE };
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(10)));
    let manifest = ArenaManifest::new(&schedule, &runner).expect("matching manifest");
    let plan = schedule.next().expect("first fixed game");
    let run = runner.run(plan, movement_game()).expect("valid game plan");
    let first_directory = TestDirectory::new("deterministic-a");
    let second_directory = TestDirectory::new("deterministic-b");

    let mut first = JsonlDatasetWriter::create(first_directory.path(), manifest.clone())
        .expect("create first dataset");
    first.write_game(&run).expect("write first game");
    first.finish().expect("finish first dataset");
    let mut second = JsonlDatasetWriter::create(second_directory.path(), manifest)
        .expect("create second dataset");
    second.write_game(&run).expect("write second game");
    second.finish().expect("finish second dataset");

    let first_files = read_dataset(first_directory.path());
    let second_files = read_dataset(second_directory.path());
    assert_eq!(first_files, second_files);
    let games_text = String::from_utf8(first_files.1).expect("UTF-8 games file");
    assert!(games_text.contains(r#"invalid \"action\"\nsecond line"#));
    assert_eq!(games_text.lines().count(), 1);
    let record: GameRecord = serde_json::from_str(games_text.trim_end()).expect("game JSON line");
    assert_eq!(record.termination, TerminationRecord::AgentFailure {
        player: formation_chess_arena::record::PlayerRecord::Red,
        phase: PhaseRecord::Movement,
        error: AgentErrorRecord::Decision(ERROR_MESSAGE.to_owned()),
    });
}

#[test]
fn writer_rejects_existing_outputs_and_out_of_order_games() {
    let matchup = matchup();
    let mut schedule =
        Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(2) }, 47);
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));
    let manifest = ArenaManifest::new(&schedule, &runner).expect("matching manifest");
    let existing_directory = TestDirectory::new("existing");
    fs::create_dir_all(existing_directory.path()).expect("create existing directory");

    let Err(existing_error) =
        JsonlDatasetWriter::create(existing_directory.path(), manifest.clone())
    else {
        panic!("existing output must be rejected");
    };
    assert!(matches!(existing_error, DatasetError::OutputExists(_)));

    let output_directory = TestDirectory::new("out-of-order");
    let mut writer =
        JsonlDatasetWriter::create(output_directory.path(), manifest).expect("create dataset");
    let second_plan = schedule.nth(1).expect("second fixed game");
    let second_run = runner.run(second_plan, Game::default()).expect("valid second game");
    let order_error = writer.write_game(&second_run).expect_err("out-of-order game must fail");
    assert!(matches!(order_error, DatasetError::InvalidDataset(_)));
    let finish_error = writer.finish().expect_err("incomplete dataset must fail");
    assert!(matches!(finish_error, DatasetError::IncompleteDataset { expected: 2, written: 0 }));
}

#[test]
fn record_conversion_rejects_tampered_counts_and_state() {
    let matchup = matchup();
    let mut schedule =
        Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 53);
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(33)));
    let plan = schedule.next().expect("first fixed game");
    let run = runner.run(plan, Game::default()).expect("valid game plan");

    let mut count_tampered = run.clone();
    count_tampered.actions[32].legal_action_count = Some(0);
    let count_error =
        GameRecord::from_game_run(&count_tampered).expect_err("tampered count must fail replay");
    assert!(matches!(count_error, formation_chess_arena::RecordError::InvalidGameRun(_)));

    let mut state_tampered = run;
    state_tampered.final_game = Game::default();
    let state_error =
        GameRecord::from_game_run(&state_tampered).expect_err("tampered state must fail replay");
    assert!(matches!(state_error, formation_chess_arena::RecordError::InvalidGameRun(_)));
}

fn create_reader_dataset(label: &str, games: u32) -> (TestDirectory, ArenaManifest) {
    let directory = TestDirectory::new(label);
    let matchup =
        Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup");
    let schedule =
        Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(games) }, 71);
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));
    let manifest = ArenaManifest::new(&schedule, &runner).expect("matching manifest");
    let mut writer = JsonlDatasetWriter::create(directory.path(), manifest.clone())
        .expect("create test dataset");
    for plan in schedule {
        let run = runner.run(plan, movement_game()).expect("run scheduled game");
        writer.write_game(&run).expect("write scheduled game");
    }
    writer.finish().expect("finish test dataset");
    (directory, manifest)
}

fn read_game_lines(directory: &Path) -> Vec<String> {
    fs::read_to_string(directory.join(GAMES_FILE_NAME))
        .expect("read games file")
        .lines()
        .map(str::to_owned)
        .collect()
}

fn write_game_lines(directory: &Path, lines: &[String]) {
    let mut text = lines.join("\n");
    if !lines.is_empty() {
        text.push('\n');
    }
    fs::write(directory.join(GAMES_FILE_NAME), text).expect("write games file");
}

#[test]
fn reader_streams_records_and_exposes_manifest() {
    let (directory, manifest) = create_reader_dataset("stream", 2);
    let mut reader = JsonlDatasetReader::open(directory.path()).expect("open dataset");

    assert_eq!(reader.root(), directory.path());
    assert_eq!(reader.manifest(), &manifest);
    let records = reader.by_ref().collect::<Result<Vec<_>, _>>().expect("read complete dataset");
    assert_eq!(records.iter().map(|record| record.game_id).collect::<Vec<_>>(), [0, 1]);
    assert_eq!(reader.read_games(), 2);
    reader.finish().expect("finish consumed reader");
}

#[test]
fn reader_rejects_manifest_and_game_schema_versions() {
    let (manifest_directory, mut manifest) = create_reader_dataset("manifest-schema", 1);
    manifest.schema_version += 1;
    let manifest_json = serde_json::to_vec_pretty(&manifest).expect("serialize manifest");
    fs::write(manifest_directory.path().join(MANIFEST_FILE_NAME), manifest_json)
        .expect("replace manifest");
    let Err(manifest_error) = JsonlDatasetReader::open(manifest_directory.path()) else {
        panic!("unsupported manifest schema must fail");
    };
    assert!(
        matches!(manifest_error, DatasetError::InvalidDataset(message) if message.contains("unsupported schema version"))
    );

    let (game_directory, _) = create_reader_dataset("game-schema", 1);
    let mut lines = read_game_lines(game_directory.path());
    let mut record: GameRecord = serde_json::from_str(&lines[0]).expect("parse game record");
    record.schema_version += 1;
    lines[0] = serde_json::to_string(&record).expect("serialize game record");
    write_game_lines(game_directory.path(), &lines);
    let mut reader = JsonlDatasetReader::open(game_directory.path()).expect("open dataset");
    let game_error = reader.next().expect("game result").expect_err("unsupported game schema");
    assert!(
        matches!(game_error, DatasetError::InvalidDataset(message) if message.contains("games.jsonl line 1") && message.contains("unsupported schema version"))
    );
}

#[test]
fn reader_reports_json_line_and_non_contiguous_game_id() {
    let (json_directory, _) = create_reader_dataset("json-line", 2);
    let mut lines = read_game_lines(json_directory.path());
    lines[1] = "{not valid json}".to_owned();
    write_game_lines(json_directory.path(), &lines);
    let mut reader = JsonlDatasetReader::open(json_directory.path()).expect("open dataset");
    reader.next().expect("first game result").expect("valid first game");
    let json_error = reader.next().expect("second game result").expect_err("invalid JSON");
    assert!(matches!(json_error, DatasetError::JsonLine { line_number: 2, .. }));

    let (id_directory, _) = create_reader_dataset("game-id", 2);
    let mut lines = read_game_lines(id_directory.path());
    let mut record: GameRecord = serde_json::from_str(&lines[1]).expect("parse second game");
    record.game_id = 4;
    lines[1] = serde_json::to_string(&record).expect("serialize second game");
    write_game_lines(id_directory.path(), &lines);
    let mut reader = JsonlDatasetReader::open(id_directory.path()).expect("open dataset");
    reader.next().expect("first game result").expect("valid first game");
    let id_error = reader.next().expect("second game result").expect_err("invalid game id");
    assert!(
        matches!(id_error, DatasetError::InvalidDataset(message) if message.contains("games.jsonl line 2") && message.contains("expected game id 1, got 4"))
    );
}

#[test]
fn reader_rejects_missing_and_extra_game_records() {
    let (missing_directory, _) = create_reader_dataset("missing", 2);
    let mut lines = read_game_lines(missing_directory.path());
    lines.pop();
    write_game_lines(missing_directory.path(), &lines);
    let reader = JsonlDatasetReader::open(missing_directory.path()).expect("open dataset");
    let missing_error = reader.finish().expect_err("missing game must fail");
    assert!(matches!(missing_error, DatasetError::IncompleteDataset { expected: 2, written: 1 }));

    let (extra_directory, _) = create_reader_dataset("extra", 1);
    let mut lines = read_game_lines(extra_directory.path());
    lines.push(lines[0].clone());
    write_game_lines(extra_directory.path(), &lines);
    let mut reader = JsonlDatasetReader::open(extra_directory.path()).expect("open dataset");
    reader.next().expect("first game result").expect("valid first game");
    let extra_error = reader.next().expect("extra game result").expect_err("extra game must fail");
    assert!(
        matches!(extra_error, DatasetError::InvalidDataset(message) if message.contains("games.jsonl line 2") && message.contains("manifest declares 1 games"))
    );
}
