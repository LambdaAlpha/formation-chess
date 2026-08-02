//! Reproducible match scheduling and data collection for Formation Chess agents.
//!
//! The crate defines stable participant identities, seeded agent factories,
//! deterministic fixed or color-paired schedules, bounded single-game
//! execution, versioned JSON Lines datasets, and strict replay verification.
//! Descriptive statistics are added later.

/// Package version of the Arena framework.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod agent_factory;
mod batch;
pub mod record;
mod replay;
mod runner;
mod schedule;
mod storage;

pub use agent_factory::AgentDescriptor;
pub use agent_factory::AgentFactory;
pub use agent_factory::RandomAgentFactory;
pub use batch::BatchError;
pub use batch::BatchHarness;
pub use batch::BatchReport;
pub use batch::DefaultScenarioFactory;
pub use batch::ScenarioError;
pub use batch::ScenarioFactory;
pub use record::ArenaManifest;
pub use record::GameRecord;
pub use record::RECORD_SCHEMA_VERSION;
pub use record::RecordError;
pub use record::STATE_HASH_ALGORITHM;
pub use replay::ReplayError;
pub use replay::ReplayVerifier;
pub use runner::ExecutedAction;
pub use runner::GameRun;
pub use runner::GameRunConfig;
pub use runner::GameRunError;
pub use runner::GameTermination;
pub use runner::MatchRunner;
pub use schedule::GamePlan;
pub use schedule::Matchup;
pub use schedule::ParticipantId;
pub use schedule::SEED_DERIVATION_VERSION;
pub use schedule::Schedule;
pub use schedule::ScheduleError;
pub use schedule::ScheduleMode;
pub use storage::DatasetError;
pub use storage::GAMES_FILE_NAME;
pub use storage::JsonlDatasetReader;
pub use storage::JsonlDatasetWriter;
pub use storage::MANIFEST_FILE_NAME;
