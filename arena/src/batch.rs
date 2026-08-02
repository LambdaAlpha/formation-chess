use std::error::Error;
use std::fmt::Display;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;

use formation_chess_core::game::Game;

use crate::ArenaManifest;
use crate::DatasetError;
use crate::GameRunError;
use crate::JsonlDatasetWriter;
use crate::MatchRunner;
use crate::RecordError;
use crate::Schedule;

/// Deterministically creates an initial game from a scheduled scenario seed.
pub trait ScenarioFactory: Send + Sync {
    fn create(&self, scenario_seed: u64) -> Result<Game, ScenarioError>;
}

/// Uses the standard Formation Chess initial game for every scenario seed.
#[derive(Debug, Copy, Clone, Default)]
pub struct DefaultScenarioFactory;

impl ScenarioFactory for DefaultScenarioFactory {
    fn create(&self, _scenario_seed: u64) -> Result<Game, ScenarioError> {
        Ok(Game::default())
    }
}

/// Scenario generation failure that prevents a scheduled game from starting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioError {
    message: String,
}

impl ScenarioError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ScenarioError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScenarioError {}

impl From<String> for ScenarioError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for ScenarioError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// Sequentially executes a fresh schedule and writes one complete JSONL dataset.
///
/// Agent failures are persisted as ordinary game terminations and do not stop
/// later games. Scenario, matchup, record, and storage failures stop the batch
/// immediately. The harness does not run games in parallel and does not resume
/// an existing or partially consumed schedule.
pub struct BatchHarness<'factory, Scenario = DefaultScenarioFactory> {
    schedule: Schedule,
    runner: MatchRunner<'factory>,
    scenario_factory: Scenario,
}

impl<'factory> BatchHarness<'factory, DefaultScenarioFactory> {
    pub fn new(schedule: Schedule, runner: MatchRunner<'factory>) -> Self {
        Self { schedule, runner, scenario_factory: DefaultScenarioFactory }
    }
}

impl<'factory, Scenario> BatchHarness<'factory, Scenario>
where Scenario: ScenarioFactory
{
    pub fn with_scenario(
        schedule: Schedule, runner: MatchRunner<'factory>, scenario_factory: Scenario,
    ) -> Self {
        Self { schedule, runner, scenario_factory }
    }

    /// Run the batch and flush persisted games after every configured interval.
    ///
    /// A smaller interval retains a more recent prefix if the process is
    /// interrupted, at the cost of more frequent I/O. `finish` always flushes
    /// the final interval on successful completion.
    pub fn run(
        self, output_root: impl AsRef<Path>, flush_every_games: NonZeroU64,
    ) -> Result<BatchReport, BatchError> {
        validate_fresh_schedule(&self.schedule)?;
        let manifest = ArenaManifest::new(&self.schedule, &self.runner)?;
        let mut writer = JsonlDatasetWriter::create(output_root, manifest)?;
        let output_root = writer.root().to_path_buf();

        for plan in self.schedule {
            let game_id = plan.game_id;
            let scenario_seed = plan.scenario_seed;
            let game = self
                .scenario_factory
                .create(scenario_seed)
                .map_err(|source| BatchError::Scenario { game_id, scenario_seed, source })?;
            let run = self
                .runner
                .run(plan, game)
                .map_err(|source| BatchError::GameRun { game_id, source })?;
            writer.write_game(&run)?;
            if writer.written_games() % flush_every_games.get() == 0 {
                writer.flush()?;
            }
        }

        let games_written = writer.written_games();
        writer.finish()?;
        Ok(BatchReport { output_root, games_written })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchReport {
    pub output_root: PathBuf,
    pub games_written: u64,
}

#[derive(Debug)]
pub enum BatchError {
    ScheduleAlreadyStarted { next_game_id: u64 },
    Record(RecordError),
    Scenario { game_id: u64, scenario_seed: u64, source: ScenarioError },
    GameRun { game_id: u64, source: GameRunError },
    Dataset(DatasetError),
}

impl Display for BatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScheduleAlreadyStarted { next_game_id } => write!(
                formatter,
                "batch schedule must start at game 0, next game id is {next_game_id}"
            ),
            Self::Record(error) => Display::fmt(error, formatter),
            Self::Scenario { game_id, scenario_seed, source } => {
                write!(formatter, "game {game_id} scenario seed {scenario_seed} failed: {source}")
            },
            Self::GameRun { game_id, source } => {
                write!(formatter, "game {game_id} execution failed: {source}")
            },
            Self::Dataset(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for BatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Record(error) => Some(error),
            Self::Scenario { source, .. } => Some(source),
            Self::GameRun { source, .. } => Some(source),
            Self::Dataset(error) => Some(error),
            Self::ScheduleAlreadyStarted { .. } => None,
        }
    }
}

impl From<RecordError> for BatchError {
    fn from(error: RecordError) -> Self {
        Self::Record(error)
    }
}

impl From<DatasetError> for BatchError {
    fn from(error: DatasetError) -> Self {
        Self::Dataset(error)
    }
}

fn validate_fresh_schedule(schedule: &Schedule) -> Result<(), BatchError> {
    let mut remaining = schedule.clone();
    let next_game_id = remaining.next().map_or(schedule.total_games(), |plan| plan.game_id);
    if next_game_id == 0 {
        Ok(())
    } else {
        Err(BatchError::ScheduleAlreadyStarted { next_game_id })
    }
}
