use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use formation_chess_agent::Agent;
use formation_chess_agent::AgentError;
use formation_chess_agent::AgentInput;
use formation_chess_agent::ScoredAction;
use formation_chess_arena::AgentDescriptor;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::BatchError;
use formation_chess_arena::BatchHarness;
use formation_chess_arena::DatasetError;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::JsonlDatasetReader;
use formation_chess_arena::MatchRunner;
use formation_chess_arena::Matchup;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::RecordError;
use formation_chess_arena::ReplayVerifier;
use formation_chess_arena::ScenarioError;
use formation_chess_arena::ScenarioFactory;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_arena::record::AgentErrorRecord;
use formation_chess_arena::record::TerminationRecord;
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
            .join(format!("formation-chess-arena-batch-{label}-{}-{unique}", std::process::id()));
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

struct FailingFactory;
struct FailingAgent;

impl Agent for FailingAgent {
    fn name(&self) -> &str {
        "failing"
    }

    fn analyze(
        &mut self, _game: &Game, _input: AgentInput<'_>, _top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        Err(AgentError::Decision("planned failure".to_owned()))
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
        Box::new(FailingAgent)
    }
}

struct FailingScenarioFactory {
    seeds: Arc<Mutex<Vec<u64>>>,
    fail_at: usize,
}

impl ScenarioFactory for FailingScenarioFactory {
    fn create(&self, scenario_seed: u64) -> Result<Game, ScenarioError> {
        let mut seeds = self.seeds.lock().expect("lock recorded scenario seeds");
        let call = seeds.len();
        seeds.push(scenario_seed);
        drop(seeds);
        if call == self.fail_at {
            Err(ScenarioError::new("planned scenario failure"))
        } else {
            Ok(movement_game())
        }
    }
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn flush_interval(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("nonzero flush interval")
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
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
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

#[test]
fn default_batch_writes_and_replays_every_scheduled_game() {
    let matchup = matchup();
    let schedule = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(3) }, 89);
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));
    let output = TestDirectory::new("default");

    let report = BatchHarness::new(schedule, runner)
        .run(output.path(), flush_interval(2))
        .expect("run default batch");
    assert_eq!(report.output_root, output.path());
    assert_eq!(report.games_written, 3);

    let mut reader = JsonlDatasetReader::open(output.path()).expect("open batch dataset");
    let records = reader.by_ref().collect::<Result<Vec<_>, _>>().expect("read batch dataset");
    assert_eq!(records.len(), 3);
    for record in records {
        assert_eq!(record.initial_state, Game::default().to_string());
        ReplayVerifier::verify(&record).expect("batch record must replay");
    }
}

#[test]
fn batch_records_agent_failures_and_continues() {
    let matchup = matchup();
    let schedule = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(3) }, 97);
    let participant_a = FailingFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));
    let output = TestDirectory::new("agent-failure");

    let report = BatchHarness::new(schedule, runner)
        .run(output.path(), flush_interval(2))
        .expect("agent failures are records, not batch errors");
    assert_eq!(report.games_written, 3);

    let reader = JsonlDatasetReader::open(output.path()).expect("open failure dataset");
    let records = reader.collect::<Result<Vec<_>, _>>().expect("read failure dataset");
    assert_eq!(records.len(), 3);
    for record in records {
        assert!(record.actions.is_empty());
        assert!(matches!(
            record.termination,
            TerminationRecord::AgentFailure {
                error: AgentErrorRecord::Decision(ref message),
                ..
            } if message == "planned failure"
        ));
        ReplayVerifier::verify(&record).expect("failure record must replay");
    }
}

#[test]
fn batch_stops_on_scenario_failure_and_leaves_an_incomplete_prefix() {
    let matchup = matchup();
    let schedule = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(4) }, 101);
    let expected_seeds =
        schedule.clone().take(3).map(|plan| plan.scenario_seed).collect::<Vec<_>>();
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));
    let seeds = Arc::new(Mutex::new(Vec::new()));
    let scenario_factory = FailingScenarioFactory { seeds: Arc::clone(&seeds), fail_at: 2 };
    let output = TestDirectory::new("scenario-failure");

    let error = BatchHarness::with_scenario(schedule, runner, scenario_factory)
        .run(output.path(), flush_interval(2))
        .expect_err("scenario failure must stop batch");
    let BatchError::Scenario { game_id: 2, source, .. } = error else {
        panic!("unexpected batch error: {error}");
    };
    assert_eq!(source.message(), "planned scenario failure");
    assert_eq!(*seeds.lock().expect("lock recorded scenario seeds"), expected_seeds);

    let mut reader = JsonlDatasetReader::open(output.path()).expect("open partial dataset");
    reader.next().expect("first game result").expect("first game was flushed");
    reader.next().expect("second game result").expect("second game was flushed");
    let count_error = reader.next().expect("incomplete result").expect_err("dataset is incomplete");
    assert!(matches!(count_error, DatasetError::IncompleteDataset { expected: 4, written: 2 }));
}

#[test]
fn batch_rejects_consumed_or_mismatched_schedules_before_creating_output() {
    let consumed_matchup = matchup();
    let mut consumed_schedule =
        Schedule::new(consumed_matchup.clone(), ScheduleMode::Fixed { games: nonzero(2) }, 103);
    consumed_schedule.next().expect("consume first plan");
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner = MatchRunner::new(
        consumed_matchup,
        &participant_a,
        &participant_b,
        GameRunConfig::new(nonzero(1)),
    );
    let consumed_output = TestDirectory::new("consumed");
    let consumed_error = BatchHarness::new(consumed_schedule, runner)
        .run(consumed_output.path(), flush_interval(1))
        .expect_err("consumed schedule must fail");
    assert!(matches!(consumed_error, BatchError::ScheduleAlreadyStarted { next_game_id: 1 }));
    assert!(!consumed_output.path().exists());

    let schedule_matchup = matchup();
    let runner_matchup = Matchup::new(participant("other_a"), participant("other_b"))
        .expect("distinct runner matchup");
    let schedule = Schedule::new(schedule_matchup, ScheduleMode::Fixed { games: nonzero(1) }, 107);
    let runner = MatchRunner::new(
        runner_matchup,
        &participant_a,
        &participant_b,
        GameRunConfig::new(nonzero(1)),
    );
    let mismatch_output = TestDirectory::new("mismatch");
    let mismatch_error = BatchHarness::new(schedule, runner)
        .run(mismatch_output.path(), flush_interval(1))
        .expect_err("mismatched schedule must fail");
    assert!(matches!(mismatch_error, BatchError::Record(RecordError::MismatchedMatchup)));
    assert!(!mismatch_output.path().exists());
}
