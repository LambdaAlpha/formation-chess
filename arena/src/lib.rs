//! Reproducible match scheduling and data collection for Formation Chess agents.
//!
//! The crate defines stable participant identities, seeded agent factories,
//! deterministic fixed or color-paired schedules, bounded single-game
//! execution, versioned JSON Lines datasets, and strict replay verification.
//! Replay-verified per-game and aggregate descriptive metrics are available for analysis.

/// Package version of the Arena framework.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod agent_factory;
mod analysis;
mod batch;
mod league;
mod metrics;
pub mod record;
mod replay;
mod runner;
mod schedule;
mod storage;

pub use agent_factory::AgentDescriptor;
pub use agent_factory::AgentFactory;
pub use agent_factory::MctsAgentFactory;
pub use agent_factory::MinAgentFactory;
pub use agent_factory::RandomAgentFactory;
pub use analysis::ANALYSIS_SCHEMA_VERSION;
pub use analysis::ActionSummary;
pub use analysis::AnalysisError;
pub use analysis::AnalysisReport;
pub use analysis::DatasetAnalyzer;
pub use analysis::DatasetSummary;
pub use analysis::FinalMaterialSummary;
pub use analysis::FloatDistributionMetrics;
pub use analysis::GAME_METRICS_FILE_NAME;
pub use analysis::ParticipantOutcomeSummary;
pub use analysis::ParticipantSummary;
pub use analysis::PhaseActionDistributionSummary;
pub use analysis::ReactionSummary;
pub use analysis::ResultSummary;
pub use analysis::SUMMARY_FILE_NAME;
pub use analysis::StateSummary;
pub use analysis::StateVisitTotals;
pub use analysis::TerminationSummary;
pub use batch::BatchError;
pub use batch::BatchHarness;
pub use batch::BatchReport;
pub use batch::DefaultScenarioFactory;
pub use batch::ScenarioError;
pub use batch::ScenarioFactory;
pub use league::LEAGUE_MANIFEST_FILE_NAME;
pub use league::LEAGUE_SCHEMA_VERSION;
pub use league::LEAGUE_SEED_DERIVATION_VERSION;
pub use league::LeagueError;
pub use league::LeagueManifest;
pub use league::LeagueMatchupRecord;
pub use league::LeagueParticipantRecord;
pub use league::LeagueReport;
pub use league::RoundRobinLeague;
pub use league::RoundRobinParticipant;
pub use metrics::ActionKind;
pub use metrics::ActionTypeMetrics;
pub use metrics::ActionsBySideMetrics;
pub use metrics::CountRatio;
pub use metrics::DistributionMetrics;
pub use metrics::FinalMaterialMetrics;
pub use metrics::GameMetrics;
pub use metrics::LastActionMetrics;
pub use metrics::MetricsError;
pub use metrics::PieceColorCounts;
pub use metrics::ReactionChangeMetrics;
pub use metrics::SideActionMetrics;
pub use metrics::StateVisitMetrics;
pub use metrics::TerminationKind;
pub use record::ActionSelectionPolicyRecord;
pub use record::AgentDescriptorRecord;
pub use record::ArenaManifest;
pub use record::GameRecord;
pub use record::GameRunConfigRecord;
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
pub use runner::MAX_GAME_ACTIONS;
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
