//! Ranked agent analysis and turn execution for Formation Chess.
//!
//! Every agent implements one Agent::analyze interface. Placement receives
//! the current game plus a compact geometric PlacementArea; movement receives
//! the current game plus an explicit legal-action slice. Analysis returns
//! complete, scored actions ordered from best to worst. Turn execution requests
//! one candidate and applies the first action through the core engine.

mod agent;
mod error;
mod executor;
mod legal_actions;
mod random;

pub use agent::Agent;
pub use agent::AgentInput;
pub use agent::ScoredAction;
pub use error::AgentError;
pub use executor::AgentAnalysis;
pub use executor::AgentTurn;
pub use executor::analyze_agent;
pub use executor::play_agent_turn;
pub use legal_actions::PlacementArea;
pub use legal_actions::legal_movement_actions;
pub use legal_actions::placement_area;
pub use random::RandomAgent;
