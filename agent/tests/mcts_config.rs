use std::num::NonZeroU16;
use std::num::NonZeroU32;

use formation_chess_agent::MCTS_BASELINE_CONFIG_ID;
use formation_chess_agent::MCTS_BASELINE_CONFIG_VERSION;
use formation_chess_agent::MCTS_BASELINE_EXPLORATION;
use formation_chess_agent::MCTS_BASELINE_ITERATIONS;
use formation_chess_agent::MCTS_CONFIG_HASH_FORMAT_VERSION;
use formation_chess_agent::MCTS_CONFIG_SCHEMA_VERSION;
use formation_chess_agent::MCTS_MAX_ITERATIONS;
use formation_chess_agent::MCTS_MAX_SIMULATION_ACTIONS;
use formation_chess_agent::MctsConfig;
use formation_chess_agent::MctsConfigError;

#[test]
fn baseline_config_is_valid_and_versioned() {
    let config = MctsConfig::baseline();

    assert_eq!(config.validate(), Ok(()));
    assert_eq!(config, MctsConfig::default());
    assert_eq!(config.schema_version, MCTS_CONFIG_SCHEMA_VERSION);
    assert_eq!(config.config_id, MCTS_BASELINE_CONFIG_ID);
    assert_eq!(config.config_version.get(), MCTS_BASELINE_CONFIG_VERSION);
    assert_eq!(config.versioned_id(), "baseline-v1");
    assert_eq!(config.iterations.get(), MCTS_BASELINE_ITERATIONS);
    assert_eq!(config.max_simulation_actions.get(), MCTS_MAX_SIMULATION_ACTIONS);
    assert_eq!(config.exploration, MCTS_BASELINE_EXPLORATION);
}

#[test]
fn baseline_config_canonical_text_is_stable() {
    let config = MctsConfig::baseline();
    let expected = concat!(
        "formation-chess-mcts-config\n",
        "hash_format_version=1\n",
        "schema_version=1\n",
        "config_id=baseline\n",
        "config_version=1\n",
        "iterations=128\n",
        "max_simulation_actions=128\n",
        "exploration_bits=3f333333\n",
    );

    assert_eq!(MCTS_CONFIG_HASH_FORMAT_VERSION, 1);
    assert_eq!(config.canonical_text().expect("valid canonical text"), expected);
    assert_eq!(
        config.sha256().expect("valid hash"),
        "226b866e3c5b34478f75b4e92a03c1107295d2e40a2a87ecc3b1effbb943a41a"
    );
}

#[test]
fn config_rejects_invalid_values() {
    let mut config = MctsConfig::baseline();
    config.schema_version += 1;
    assert_eq!(
        config.validate(),
        Err(MctsConfigError::UnsupportedSchemaVersion {
            actual: MCTS_CONFIG_SCHEMA_VERSION + 1,
            supported: MCTS_CONFIG_SCHEMA_VERSION,
        })
    );

    let mut config = MctsConfig::baseline();
    config.config_id = "MCTS".to_owned();
    assert_eq!(config.validate(), Err(MctsConfigError::InvalidConfigId("MCTS".to_owned())));

    let mut config = MctsConfig::baseline();
    config.iterations = NonZeroU32::new(MCTS_MAX_ITERATIONS + 1).expect("nonzero iterations");
    assert_eq!(
        config.validate(),
        Err(MctsConfigError::ValueAboveMaximum {
            field: "iterations",
            actual: u64::from(MCTS_MAX_ITERATIONS + 1),
            maximum: u64::from(MCTS_MAX_ITERATIONS),
        })
    );

    let mut config = MctsConfig::baseline();
    config.max_simulation_actions =
        NonZeroU16::new(MCTS_MAX_SIMULATION_ACTIONS + 1).expect("nonzero action limit");
    assert_eq!(
        config.validate(),
        Err(MctsConfigError::ValueAboveMaximum {
            field: "max_simulation_actions",
            actual: u64::from(MCTS_MAX_SIMULATION_ACTIONS + 1),
            maximum: u64::from(MCTS_MAX_SIMULATION_ACTIONS),
        })
    );

    let mut config = MctsConfig::baseline();
    config.exploration = f32::NAN;
    assert_eq!(config.validate(), Err(MctsConfigError::InvalidExploration(f32::NAN.to_bits())));
}
