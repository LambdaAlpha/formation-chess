use std::num::NonZeroU8;
use std::num::NonZeroU16;
use std::num::NonZeroU32;

use formation_chess_agent::MIN_BEST_CONFIG_ID;
use formation_chess_agent::MIN_BEST_CONFIG_VERSION;
use formation_chess_agent::MIN_CONFIG_HASH_FORMAT_VERSION;
use formation_chess_agent::MIN_CONFIG_SCHEMA_VERSION;
use formation_chess_agent::MIN_EVALUATION_MODEL_VERSION;
use formation_chess_agent::MIN_MAX_NODE_BUDGET;
use formation_chess_agent::MIN_MAX_SEARCH_DEPTH;
use formation_chess_agent::MIN_MAX_SEARCH_WIDTH;
use formation_chess_agent::MIN_TERMINAL_UTILITY;
use formation_chess_agent::MinConfig;
use formation_chess_agent::MinConfigError;
use formation_chess_agent::MinFeatureWeights;

#[test]
fn best_config_is_valid_and_versioned() {
    let config = MinConfig::best();

    assert_eq!(config.validate(), Ok(()));
    assert_eq!(config, MinConfig::default());
    assert_eq!(config.schema_version, MIN_CONFIG_SCHEMA_VERSION);
    assert_eq!(config.config_id, MIN_BEST_CONFIG_ID);
    assert_eq!(config.config_version.get(), MIN_BEST_CONFIG_VERSION);
    assert_eq!(config.versioned_id(), "best-v1");
    assert_eq!(config.placement_search.max_depth.get(), MIN_MAX_SEARCH_DEPTH);
    assert_eq!(config.movement_search.max_depth.get(), MIN_MAX_SEARCH_DEPTH);
    assert!(config.evaluation.non_terminal_utility_limit.get() < MIN_TERMINAL_UTILITY);
    assert!(config.evaluation.placement_weights.total() > 0);
    assert!(config.evaluation.movement_weights.total() > 0);
}

#[test]
fn best_config_canonical_text_and_hash_are_stable() {
    let config = MinConfig::best();
    let expected = concat!(
        "formation-chess-min-config\n",
        "hash_format_version=1\n",
        "schema_version=1\n",
        "config_id=best\n",
        "config_version=1\n",
        "placement_search.max_depth=2\n",
        "placement_search.max_nodes=6000\n",
        "placement_search.root_width=32\n",
        "placement_search.opponent_width=8\n",
        "placement_search.response_width=4\n",
        "movement_search.max_depth=2\n",
        "movement_search.max_nodes=20000\n",
        "movement_search.opponent_width=12\n",
        "movement_search.response_width=8\n",
        "evaluation.model_version=1\n",
        "evaluation.non_terminal_utility_limit=9500\n",
        "evaluation.placement_weights.vital_safety=240\n",
        "evaluation.placement_weights.effective_abilities=180\n",
        "evaluation.placement_weights.formation_effects=240\n",
        "evaluation.placement_weights.control=140\n",
        "evaluation.placement_weights.mobility=60\n",
        "evaluation.placement_weights.action_effects=40\n",
        "evaluation.placement_weights.white_resources=20\n",
        "evaluation.placement_weights.material=20\n",
        "evaluation.placement_weights.tempo=20\n",
        "evaluation.placement_weights.interactions=160\n",
        "evaluation.movement_weights.vital_safety=300\n",
        "evaluation.movement_weights.effective_abilities=120\n",
        "evaluation.movement_weights.formation_effects=120\n",
        "evaluation.movement_weights.control=120\n",
        "evaluation.movement_weights.mobility=80\n",
        "evaluation.movement_weights.action_effects=220\n",
        "evaluation.movement_weights.white_resources=40\n",
        "evaluation.movement_weights.material=40\n",
        "evaluation.movement_weights.tempo=40\n",
        "evaluation.movement_weights.interactions=180\n",
    );

    assert_eq!(MIN_CONFIG_HASH_FORMAT_VERSION, 1);
    assert_eq!(config.canonical_text().expect("valid canonical text"), expected);
    assert_eq!(
        config.sha256().expect("valid hash"),
        "c9e9eaac1b2edea2b0da5aa3eba983a6b412df1476b7a3eff3abe63ae0c17c2e"
    );
}

#[test]
fn config_rejects_unsupported_versions_and_invalid_ids() {
    let mut config = MinConfig::best();
    config.schema_version += 1;
    assert_eq!(
        config.validate(),
        Err(MinConfigError::UnsupportedSchemaVersion {
            actual: MIN_CONFIG_SCHEMA_VERSION + 1,
            supported: MIN_CONFIG_SCHEMA_VERSION,
        })
    );

    for id in ["", "Best", "best_config", "-best", "best/config"] {
        let mut config = MinConfig::best();
        config.config_id = id.to_owned();
        assert_eq!(config.validate(), Err(MinConfigError::InvalidConfigId(id.to_owned())));
    }

    let mut config = MinConfig::best();
    config.evaluation.model_version = NonZeroU16::new(MIN_EVALUATION_MODEL_VERSION + 1)
        .expect("unsupported version remains non-zero");
    assert_eq!(
        config.validate(),
        Err(MinConfigError::UnsupportedEvaluationModelVersion {
            actual: MIN_EVALUATION_MODEL_VERSION + 1,
            supported: MIN_EVALUATION_MODEL_VERSION,
        })
    );
}

#[test]
fn config_rejects_search_limits_above_hard_bounds() {
    let mut config = MinConfig::best();
    config.movement_search.max_depth =
        NonZeroU8::new(MIN_MAX_SEARCH_DEPTH + 1).expect("depth remains non-zero");
    assert_eq!(
        config.validate(),
        Err(MinConfigError::ValueAboveMaximum {
            field: "movement_search.max_depth",
            actual: u64::from(MIN_MAX_SEARCH_DEPTH + 1),
            maximum: u64::from(MIN_MAX_SEARCH_DEPTH),
        })
    );

    let mut config = MinConfig::best();
    config.placement_search.root_width =
        NonZeroU8::new(MIN_MAX_SEARCH_WIDTH + 1).expect("width remains non-zero");
    assert_eq!(
        config.validate(),
        Err(MinConfigError::ValueAboveMaximum {
            field: "placement_search.root_width",
            actual: u64::from(MIN_MAX_SEARCH_WIDTH + 1),
            maximum: u64::from(MIN_MAX_SEARCH_WIDTH),
        })
    );

    let mut config = MinConfig::best();
    config.placement_search.max_nodes =
        NonZeroU32::new(MIN_MAX_NODE_BUDGET + 1).expect("budget remains non-zero");
    assert_eq!(
        config.validate(),
        Err(MinConfigError::ValueAboveMaximum {
            field: "placement_search.max_nodes",
            actual: u64::from(MIN_MAX_NODE_BUDGET + 1),
            maximum: u64::from(MIN_MAX_NODE_BUDGET),
        })
    );
}

#[test]
fn config_rejects_invalid_evaluation_bounds() {
    let mut config = MinConfig::best();
    config.evaluation.non_terminal_utility_limit =
        NonZeroU16::new(MIN_TERMINAL_UTILITY).expect("terminal utility is non-zero");
    assert_eq!(
        config.validate(),
        Err(MinConfigError::NonTerminalUtilityLimit {
            actual: MIN_TERMINAL_UTILITY,
            terminal: MIN_TERMINAL_UTILITY,
        })
    );

    let mut config = MinConfig::best();
    config.evaluation.placement_weights = MinFeatureWeights {
        vital_safety: 0,
        effective_abilities: 0,
        formation_effects: 0,
        control: 0,
        mobility: 0,
        action_effects: 0,
        white_resources: 0,
        material: 0,
        tempo: 0,
        interactions: 0,
    };
    assert_eq!(config.validate(), Err(MinConfigError::EmptyFeatureWeights { phase: "placement" }));
}
