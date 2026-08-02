//! Reproducible match scheduling and data collection for Formation Chess agents.
//!
//! The current foundation defines stable participant identities, seeded agent
//! factories, and deterministic fixed or color-paired game schedules. Game
//! execution, recording, replay, and statistics are added in later layers.

mod agent_factory;
mod schedule;

pub use agent_factory::AgentDescriptor;
pub use agent_factory::AgentFactory;
pub use agent_factory::RandomAgentFactory;
pub use schedule::GamePlan;
pub use schedule::Matchup;
pub use schedule::ParticipantId;
pub use schedule::SEED_DERIVATION_VERSION;
pub use schedule::Schedule;
pub use schedule::ScheduleError;
pub use schedule::ScheduleMode;
