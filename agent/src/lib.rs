//! Ranked agent analysis and turn execution for Formation Chess.
//!
//! The crate also defines versioned, validated configuration contracts for the
//! pure UCT MCTS agent and the fast depth-limited Min agent.
//!
//! Every agent implements one Agent::analyze interface. Placement receives
//! the current game plus a compact geometric PlacementArea; movement receives
//! the current game plus an explicit legal-action slice. Analysis returns
//! complete, scored actions ordered from best to worst. Turn execution asks an
//! explicit selector for its candidate count and applies the selected action.

mod agent;
mod error;
mod executor;
mod legal_actions;
mod mcts;
mod min;
mod random;
mod selection;

/// Package version of the agent framework and bundled agents.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use agent::Agent;
pub use agent::AgentInput;
pub use agent::ScoredAction;
pub use error::AgentError;
pub use executor::AgentAnalysis;
pub use executor::AgentTurn;
pub use executor::PreparedInput;
pub use executor::PreparedTurn;
pub use executor::analyze_agent;
pub use executor::analyze_prepared;
pub use executor::play_agent_turn;
pub use executor::prepare_turn;
pub use legal_actions::PlacementArea;
pub use legal_actions::legal_movement_actions;
pub use legal_actions::placement_area;
pub use mcts::MCTS_BASELINE_CONFIG_ID;
pub use mcts::MCTS_BASELINE_CONFIG_VERSION;
pub use mcts::MCTS_BASELINE_EXPLORATION;
pub use mcts::MCTS_BASELINE_ITERATIONS;
pub use mcts::MCTS_CONFIG_HASH_ALGORITHM;
pub use mcts::MCTS_CONFIG_HASH_FORMAT_VERSION;
pub use mcts::MCTS_CONFIG_SCHEMA_VERSION;
pub use mcts::MCTS_MAX_ITERATIONS;
pub use mcts::MCTS_MAX_SIMULATION_ACTIONS;
pub use mcts::MctsAgent;
pub use mcts::MctsConfig;
pub use mcts::MctsConfigError;
pub use mcts::MctsStats;
pub use min::MIN_BEST_CONFIG_ID;
pub use min::MIN_BEST_CONFIG_VERSION;
pub use min::MIN_CONFIG_HASH_ALGORITHM;
pub use min::MIN_CONFIG_HASH_FORMAT_VERSION;
pub use min::MIN_CONFIG_SCHEMA_VERSION;
pub use min::MIN_EVALUATION_MODEL_VERSION;
pub use min::MIN_FEATURE_SCALE;
pub use min::MIN_MAX_NODE_BUDGET;
pub use min::MIN_MAX_SEARCH_DEPTH;
pub use min::MIN_MAX_SEARCH_WIDTH;
pub use min::MIN_TERMINAL_UTILITY;
pub use min::MinAgent;
pub use min::MinConfig;
pub use min::MinConfigError;
pub use min::MinEvaluation;
pub use min::MinEvaluationConfig;
pub use min::MinEvaluator;
pub use min::MinFeatureContributions;
pub use min::MinFeatureVector;
pub use min::MinFeatureWeights;
pub use min::MinMovementSearchConfig;
pub use min::MinPlacementSearchConfig;
pub use random::RandomAgent;
pub use selection::ActionSelectionError;
pub use selection::ActionSelectionPolicy;
pub use selection::ActionSelector;
pub use selection::RankSoftmaxPolicy;
