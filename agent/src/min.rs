use std::fmt::Display;
use std::num::NonZeroU8;
use std::num::NonZeroU16;
use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

/// Schema version of the serialized Min configuration.
pub const MIN_CONFIG_SCHEMA_VERSION: u16 = 1;
/// Version of the canonical text hashed for configuration identity.
pub const MIN_CONFIG_HASH_FORMAT_VERSION: u16 = 1;
/// Hash algorithm used by [`MinConfig::sha256`].
pub const MIN_CONFIG_HASH_ALGORITHM: &str = "sha256";
/// Identifier of the only bundled Min configuration.
pub const MIN_BEST_CONFIG_ID: &str = "best";
/// Version of the current bundled `best` configuration.
pub const MIN_BEST_CONFIG_VERSION: u16 = 1;
/// Static evaluation model understood by this agent version.
pub const MIN_EVALUATION_MODEL_VERSION: u16 = 1;
/// Hard maximum number of simulated actions from the root.
pub const MIN_MAX_SEARCH_DEPTH: u8 = 3;
/// Hard maximum for any selective-search width.
pub const MIN_MAX_SEARCH_WIDTH: u8 = 64;
/// Hard maximum number of simulated nodes per analysis.
pub const MIN_MAX_NODE_BUDGET: u32 = 100_000;
/// Exact terminal utility before conversion to the public floating-point score.
pub const MIN_TERMINAL_UTILITY: u16 = 10_000;

/// Complete versioned configuration for the fast depth-limited Min agent.
///
/// The built-in [`Self::best`] value is the only bundled profile. Callers may
/// deserialize or modify a value for tuning, but every value must pass
/// [`Self::validate`] before an agent uses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinConfig {
    pub schema_version: u16,
    pub config_id: String,
    pub config_version: NonZeroU16,
    pub placement_search: MinPlacementSearchConfig,
    pub movement_search: MinMovementSearchConfig,
    pub evaluation: MinEvaluationConfig,
}

/// Selective search limits for the placement phase.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinPlacementSearchConfig {
    pub max_depth: NonZeroU8,
    pub max_nodes: NonZeroU32,
    pub root_width: NonZeroU8,
    pub opponent_width: NonZeroU8,
    pub response_width: NonZeroU8,
}

/// Selective search limits for the movement phase.
///
/// Movement always scans every legal root action. Width limits apply only to
/// non-terminal opponent replies and third-ply responses.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinMovementSearchConfig {
    pub max_depth: NonZeroU8,
    pub max_nodes: NonZeroU32,
    pub opponent_width: NonZeroU8,
    pub response_width: NonZeroU8,
}

/// Static evaluation settings shared by placement and movement search.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinEvaluationConfig {
    pub model_version: NonZeroU16,
    pub non_terminal_utility_limit: NonZeroU16,
    pub placement_weights: MinFeatureWeights,
    pub movement_weights: MinFeatureWeights,
}

/// Relative weights for oriented feature groups.
///
/// Each future feature group is normalized so that a larger value is better
/// for the evaluated player. These weights express relative influence only;
/// terminal results and searched tactical outcomes are handled outside this
/// soft aggregation and cannot be compensated by these values.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinFeatureWeights {
    pub vital_safety: u16,
    pub effective_abilities: u16,
    pub formation_effects: u16,
    pub control: u16,
    pub mobility: u16,
    pub action_effects: u16,
    pub white_resources: u16,
    pub material: u16,
    pub tempo: u16,
    pub interactions: u16,
}

impl MinFeatureWeights {
    /// Sum used to normalize the weighted feature aggregate.
    pub fn total(self) -> u32 {
        u32::from(self.vital_safety)
            + u32::from(self.effective_abilities)
            + u32::from(self.formation_effects)
            + u32::from(self.control)
            + u32::from(self.mobility)
            + u32::from(self.action_effects)
            + u32::from(self.white_resources)
            + u32::from(self.material)
            + u32::from(self.tempo)
            + u32::from(self.interactions)
    }
}

impl MinConfig {
    /// Current source-defined `best` configuration.
    ///
    /// Arena datasets must record the versioned ID, full serialized value, and
    /// SHA-256 rather than storing only the moving `best` alias.
    pub fn best() -> Self {
        Self {
            schema_version: MIN_CONFIG_SCHEMA_VERSION,
            config_id: MIN_BEST_CONFIG_ID.to_owned(),
            config_version: non_zero_u16(MIN_BEST_CONFIG_VERSION),
            placement_search: MinPlacementSearchConfig {
                max_depth: non_zero_u8(MIN_MAX_SEARCH_DEPTH),
                max_nodes: non_zero_u32(6_000),
                root_width: non_zero_u8(32),
                opponent_width: non_zero_u8(8),
                response_width: non_zero_u8(4),
            },
            movement_search: MinMovementSearchConfig {
                max_depth: non_zero_u8(MIN_MAX_SEARCH_DEPTH),
                max_nodes: non_zero_u32(20_000),
                opponent_width: non_zero_u8(12),
                response_width: non_zero_u8(8),
            },
            evaluation: MinEvaluationConfig {
                model_version: non_zero_u16(MIN_EVALUATION_MODEL_VERSION),
                non_terminal_utility_limit: non_zero_u16(9_500),
                placement_weights: MinFeatureWeights {
                    vital_safety: 240,
                    effective_abilities: 180,
                    formation_effects: 240,
                    control: 140,
                    mobility: 60,
                    action_effects: 40,
                    white_resources: 20,
                    material: 20,
                    tempo: 20,
                    interactions: 160,
                },
                movement_weights: MinFeatureWeights {
                    vital_safety: 300,
                    effective_abilities: 120,
                    formation_effects: 120,
                    control: 120,
                    mobility: 80,
                    action_effects: 220,
                    white_resources: 40,
                    material: 40,
                    tempo: 40,
                    interactions: 180,
                },
            },
        }
    }

    /// Stable versioned label such as `best-v1`.
    pub fn versioned_id(&self) -> String {
        format!("{}-v{}", self.config_id, self.config_version)
    }

    /// Validate hard Min limits and the supported configuration contract.
    pub fn validate(&self) -> Result<(), MinConfigError> {
        if self.schema_version != MIN_CONFIG_SCHEMA_VERSION {
            return Err(MinConfigError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                supported: MIN_CONFIG_SCHEMA_VERSION,
            });
        }
        if !is_valid_config_id(&self.config_id) {
            return Err(MinConfigError::InvalidConfigId(self.config_id.clone()));
        }

        validate_depth("placement_search.max_depth", self.placement_search.max_depth)?;
        validate_node_budget("placement_search.max_nodes", self.placement_search.max_nodes)?;
        validate_width("placement_search.root_width", self.placement_search.root_width)?;
        validate_width("placement_search.opponent_width", self.placement_search.opponent_width)?;
        validate_width("placement_search.response_width", self.placement_search.response_width)?;

        validate_depth("movement_search.max_depth", self.movement_search.max_depth)?;
        validate_node_budget("movement_search.max_nodes", self.movement_search.max_nodes)?;
        validate_width("movement_search.opponent_width", self.movement_search.opponent_width)?;
        validate_width("movement_search.response_width", self.movement_search.response_width)?;

        let model_version = self.evaluation.model_version.get();
        if model_version != MIN_EVALUATION_MODEL_VERSION {
            return Err(MinConfigError::UnsupportedEvaluationModelVersion {
                actual: model_version,
                supported: MIN_EVALUATION_MODEL_VERSION,
            });
        }
        let non_terminal_limit = self.evaluation.non_terminal_utility_limit.get();
        if non_terminal_limit >= MIN_TERMINAL_UTILITY {
            return Err(MinConfigError::NonTerminalUtilityLimit {
                actual: non_terminal_limit,
                terminal: MIN_TERMINAL_UTILITY,
            });
        }
        validate_weights("placement", self.evaluation.placement_weights)?;
        validate_weights("movement", self.evaluation.movement_weights)?;
        Ok(())
    }

    /// Stable canonical text covered by [`Self::sha256`].
    pub fn canonical_text(&self) -> Result<String, MinConfigError> {
        self.validate()?;
        Ok(format!(
            concat!(
                "formation-chess-min-config\n",
                "hash_format_version={}\n",
                "schema_version={}\n",
                "config_id={}\n",
                "config_version={}\n",
                "placement_search.max_depth={}\n",
                "placement_search.max_nodes={}\n",
                "placement_search.root_width={}\n",
                "placement_search.opponent_width={}\n",
                "placement_search.response_width={}\n",
                "movement_search.max_depth={}\n",
                "movement_search.max_nodes={}\n",
                "movement_search.opponent_width={}\n",
                "movement_search.response_width={}\n",
                "evaluation.model_version={}\n",
                "evaluation.non_terminal_utility_limit={}\n",
                "evaluation.placement_weights.vital_safety={}\n",
                "evaluation.placement_weights.effective_abilities={}\n",
                "evaluation.placement_weights.formation_effects={}\n",
                "evaluation.placement_weights.control={}\n",
                "evaluation.placement_weights.mobility={}\n",
                "evaluation.placement_weights.action_effects={}\n",
                "evaluation.placement_weights.white_resources={}\n",
                "evaluation.placement_weights.material={}\n",
                "evaluation.placement_weights.tempo={}\n",
                "evaluation.placement_weights.interactions={}\n",
                "evaluation.movement_weights.vital_safety={}\n",
                "evaluation.movement_weights.effective_abilities={}\n",
                "evaluation.movement_weights.formation_effects={}\n",
                "evaluation.movement_weights.control={}\n",
                "evaluation.movement_weights.mobility={}\n",
                "evaluation.movement_weights.action_effects={}\n",
                "evaluation.movement_weights.white_resources={}\n",
                "evaluation.movement_weights.material={}\n",
                "evaluation.movement_weights.tempo={}\n",
                "evaluation.movement_weights.interactions={}\n",
            ),
            MIN_CONFIG_HASH_FORMAT_VERSION,
            self.schema_version,
            self.config_id,
            self.config_version,
            self.placement_search.max_depth,
            self.placement_search.max_nodes,
            self.placement_search.root_width,
            self.placement_search.opponent_width,
            self.placement_search.response_width,
            self.movement_search.max_depth,
            self.movement_search.max_nodes,
            self.movement_search.opponent_width,
            self.movement_search.response_width,
            self.evaluation.model_version,
            self.evaluation.non_terminal_utility_limit,
            self.evaluation.placement_weights.vital_safety,
            self.evaluation.placement_weights.effective_abilities,
            self.evaluation.placement_weights.formation_effects,
            self.evaluation.placement_weights.control,
            self.evaluation.placement_weights.mobility,
            self.evaluation.placement_weights.action_effects,
            self.evaluation.placement_weights.white_resources,
            self.evaluation.placement_weights.material,
            self.evaluation.placement_weights.tempo,
            self.evaluation.placement_weights.interactions,
            self.evaluation.movement_weights.vital_safety,
            self.evaluation.movement_weights.effective_abilities,
            self.evaluation.movement_weights.formation_effects,
            self.evaluation.movement_weights.control,
            self.evaluation.movement_weights.mobility,
            self.evaluation.movement_weights.action_effects,
            self.evaluation.movement_weights.white_resources,
            self.evaluation.movement_weights.material,
            self.evaluation.movement_weights.tempo,
            self.evaluation.movement_weights.interactions,
        ))
    }

    /// SHA-256 of [`Self::canonical_text`].
    pub fn sha256(&self) -> Result<String, MinConfigError> {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let digest = Sha256::digest(self.canonical_text()?.as_bytes());
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            output.push(HEX[usize::from(byte >> 4)] as char);
            output.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        Ok(output)
    }
}

impl Default for MinConfig {
    fn default() -> Self {
        Self::best()
    }
}

/// A Min configuration that cannot be used safely or reproduced correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinConfigError {
    UnsupportedSchemaVersion { actual: u16, supported: u16 },
    InvalidConfigId(String),
    UnsupportedEvaluationModelVersion { actual: u16, supported: u16 },
    ValueAboveMaximum { field: &'static str, actual: u64, maximum: u64 },
    NonTerminalUtilityLimit { actual: u16, terminal: u16 },
    EmptyFeatureWeights { phase: &'static str },
}

impl Display for MinConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { actual, supported } => write!(
                formatter,
                "unsupported Min config schema version {actual}; supported version is {supported}"
            ),
            Self::InvalidConfigId(id) => {
                write!(formatter, "invalid Min config ID {id:?}; expected [a-z][a-z0-9-]{{0,31}}")
            },
            Self::UnsupportedEvaluationModelVersion { actual, supported } => write!(
                formatter,
                "unsupported Min evaluation model version {actual}; supported version is {supported}"
            ),
            Self::ValueAboveMaximum { field, actual, maximum } => {
                write!(formatter, "Min config field {field} is {actual}; maximum is {maximum}")
            },
            Self::NonTerminalUtilityLimit { actual, terminal } => write!(
                formatter,
                "non-terminal utility limit {actual} must be below terminal utility {terminal}"
            ),
            Self::EmptyFeatureWeights { phase } => {
                write!(formatter, "{phase} feature weights must contain a non-zero value")
            },
        }
    }
}

impl std::error::Error for MinConfigError {}

fn validate_depth(field: &'static str, value: NonZeroU8) -> Result<(), MinConfigError> {
    validate_maximum(field, u64::from(value.get()), u64::from(MIN_MAX_SEARCH_DEPTH))
}

fn validate_width(field: &'static str, value: NonZeroU8) -> Result<(), MinConfigError> {
    validate_maximum(field, u64::from(value.get()), u64::from(MIN_MAX_SEARCH_WIDTH))
}

fn validate_node_budget(field: &'static str, value: NonZeroU32) -> Result<(), MinConfigError> {
    validate_maximum(field, u64::from(value.get()), u64::from(MIN_MAX_NODE_BUDGET))
}

fn validate_maximum(field: &'static str, actual: u64, maximum: u64) -> Result<(), MinConfigError> {
    if actual > maximum {
        Err(MinConfigError::ValueAboveMaximum { field, actual, maximum })
    } else {
        Ok(())
    }
}

fn validate_weights(phase: &'static str, weights: MinFeatureWeights) -> Result<(), MinConfigError> {
    if weights.total() == 0 { Err(MinConfigError::EmptyFeatureWeights { phase }) } else { Ok(()) }
}

fn is_valid_config_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1 ..= 32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1 ..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn non_zero_u8(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("built-in Min u8 values must be non-zero")
}

fn non_zero_u16(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).expect("built-in Min u16 values must be non-zero")
}

fn non_zero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("built-in Min u32 values must be non-zero")
}
