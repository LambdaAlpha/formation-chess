use std::collections::BTreeMap;

use formation_chess_agent::Agent;
use formation_chess_agent::MIN_CONFIG_HASH_ALGORITHM;
use formation_chess_agent::MIN_CONFIG_HASH_FORMAT_VERSION;
use formation_chess_agent::MIN_CONFIG_SCHEMA_VERSION;
use formation_chess_agent::MIN_EVALUATION_MODEL_VERSION;
use formation_chess_agent::MinAgent;
use formation_chess_agent::MinConfig;
use formation_chess_agent::MinConfigError;
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

/// Factory for one validated, fully described Min configuration.
#[derive(Debug, Clone)]
pub struct MinAgentFactory {
    config: MinConfig,
}

impl MinAgentFactory {
    /// Validate and retain one complete Min configuration.
    pub fn new(config: MinConfig) -> Result<Self, MinConfigError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Factory for the current source-defined `best` configuration.
    pub fn best() -> Self {
        Self::new(MinConfig::best()).expect("built-in Min best config must remain valid")
    }

    /// Complete immutable configuration created for every game.
    pub fn config(&self) -> &MinConfig {
        &self.config
    }
}

impl Default for MinAgentFactory {
    fn default() -> Self {
        Self::best()
    }
}

impl AgentFactory for MinAgentFactory {
    fn descriptor(&self) -> AgentDescriptor {
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "config".to_owned(),
            serde_json::to_value(&self.config).expect("validated Min config must serialize"),
        );
        parameters.insert(
            "config_sha256".to_owned(),
            Value::String(self.config.sha256().expect("validated Min config must hash")),
        );
        parameters.insert(
            "config_hash_algorithm".to_owned(),
            Value::String(MIN_CONFIG_HASH_ALGORITHM.to_owned()),
        );
        parameters.insert(
            "config_hash_format_version".to_owned(),
            Value::from(MIN_CONFIG_HASH_FORMAT_VERSION),
        );
        parameters
            .insert("config_schema_version".to_owned(), Value::from(MIN_CONFIG_SCHEMA_VERSION));
        parameters.insert(
            "evaluation_model_version".to_owned(),
            Value::from(MIN_EVALUATION_MODEL_VERSION),
        );

        AgentDescriptor {
            kind: "min".to_owned(),
            display_name: format!("Min AI {}", self.config.versioned_id()),
            implementation_version: formation_chess_agent::VERSION.to_owned(),
            parameters,
        }
    }

    fn create(&self, _seed: u64) -> Box<dyn Agent> {
        Box::new(
            MinAgent::new(self.config.clone()).expect("MinAgentFactory config must remain valid"),
        )
    }
}
