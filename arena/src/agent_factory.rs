use std::collections::BTreeMap;

use formation_chess_agent::Agent;
use formation_chess_agent::RandomAgent;
use serde_json::Value;

/// Stable metadata describing an agent implementation and configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentDescriptor {
    pub kind: String,
    pub display_name: String,
    pub implementation_version: String,
    pub parameters: BTreeMap<String, Value>,
}

/// Creates a fresh seeded agent instance for each arena seat and game.
pub trait AgentFactory: Send + Sync {
    fn descriptor(&self) -> AgentDescriptor;

    fn create(&self, seed: u64) -> Box<dyn Agent>;
}

/// Factory for the bundled deterministic random baseline.
#[derive(Debug, Copy, Clone, Default)]
pub struct RandomAgentFactory;

impl AgentFactory for RandomAgentFactory {
    fn descriptor(&self) -> AgentDescriptor {
        AgentDescriptor {
            kind: "random".to_owned(),
            display_name: "Random".to_owned(),
            implementation_version: formation_chess_agent::VERSION.to_owned(),
            parameters: BTreeMap::new(),
        }
    }

    fn create(&self, seed: u64) -> Box<dyn Agent> {
        Box::new(RandomAgent::with_seed(seed))
    }
}
