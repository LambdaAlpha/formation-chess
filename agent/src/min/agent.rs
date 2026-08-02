use std::num::NonZeroU8;

use formation_chess_core::game::Game;

use super::MinConfig;
use super::MinConfigError;
use super::MinEvaluator;
use super::placement::analyze_placements;
use crate::Agent;
use crate::AgentError;
use crate::AgentInput;
use crate::ScoredAction;

/// Fast deterministic Min agent.
///
/// Placement search is implemented. Movement analysis remains a separate
/// reviewable change and currently returns an explicit decision error.
#[derive(Debug, Clone)]
pub struct MinAgent {
    config: MinConfig,
    evaluator: MinEvaluator,
    name: String,
}

impl MinAgent {
    /// Validate and construct a Min agent from one complete configuration.
    pub fn new(config: MinConfig) -> Result<Self, MinConfigError> {
        let evaluator = MinEvaluator::new(&config)?;
        let name = format!("Min {}", config.versioned_id());
        Ok(Self { config, evaluator, name })
    }

    /// Construct the current bundled `best` Min configuration.
    pub fn best() -> Self {
        Self::new(MinConfig::best()).expect("built-in Min best config must remain valid")
    }

    /// Complete immutable configuration used by this agent.
    pub fn config(&self) -> &MinConfig {
        &self.config
    }
}

impl Default for MinAgent {
    fn default() -> Self {
        Self::best()
    }
}

impl Agent for MinAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn analyze(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        match input {
            AgentInput::Placement { area } => {
                analyze_placements(game, area, top_k, self.config.placement_search, self.evaluator)
            },
            AgentInput::Movement { .. } => {
                Err(AgentError::Decision("Min movement search is not implemented".to_owned()))
            },
        }
    }
}
