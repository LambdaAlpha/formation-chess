//! Reproducible match scheduling and data collection for Formation Chess agents.
//!
//! The crate defines stable participant identities, seeded agent factories,
//! deterministic fixed or color-paired schedules, and bounded single-game
//! execution. Persistent recording, replay, and statistics are added later.

mod agent_factory;
mod runner;
mod schedule;

pub use agent_factory::AgentDescriptor;
pub use agent_factory::AgentFactory;
pub use agent_factory::RandomAgentFactory;
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
