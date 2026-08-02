//! Reproducible match scheduling and data collection for Formation Chess agents.
//!
//! The crate defines stable participant identities, seeded agent factories,
//! deterministic fixed or color-paired schedules, bounded single-game
//! execution, and versioned JSON Lines datasets. Replay and statistics are
//! added later.

/// Package version of the Arena framework.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod agent_factory;
pub mod record;
mod runner;
mod schedule;
mod storage;

pub use agent_factory::AgentDescriptor;
pub use agent_factory::AgentFactory;
pub use agent_factory::RandomAgentFactory;
pub use record::ArenaManifest;
pub use record::GameRecord;
pub use record::RECORD_SCHEMA_VERSION;
pub use record::RecordError;
pub use record::STATE_HASH_ALGORITHM;
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
