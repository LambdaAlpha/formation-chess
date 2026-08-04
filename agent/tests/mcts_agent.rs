use std::num::NonZeroU8;
use std::num::NonZeroU16;
use std::num::NonZeroU32;

use formation_chess_agent::Agent;
use formation_chess_agent::AgentInput;
use formation_chess_agent::MctsAgent;
use formation_chess_agent::MctsConfig;
use formation_chess_agent::analyze_agent;
use formation_chess_agent::placement_area;
use formation_chess_core::game::Game;

fn test_config(iterations: u32, action_limit: u16) -> MctsConfig {
    let mut config = MctsConfig::baseline();
    "test".clone_into(&mut config.config_id);
    config.iterations = NonZeroU32::new(iterations).expect("nonzero iterations");
    config.max_simulation_actions =
        NonZeroU16::new(action_limit).expect("nonzero simulation action limit");
    config
}

#[test]
fn seeded_placement_analysis_is_reproducible_and_valid() {
    let game = Game::default();
    let area = placement_area(&game).expect("placement area");
    let config = test_config(16, 8);
    let mut first = MctsAgent::with_seed(config.clone(), 42).expect("first agent");
    let mut second = MctsAgent::with_seed(config, 42).expect("second agent");
    let top_k = NonZeroU8::new(3).expect("nonzero top k");

    let first_candidates =
        first.analyze(&game, AgentInput::Placement { area }, top_k).expect("first analysis");
    let second_candidates =
        second.analyze(&game, AgentInput::Placement { area }, top_k).expect("second analysis");

    assert_eq!(first_candidates, second_candidates);
    assert_eq!(first_candidates.len(), 3);
    for candidate in first_candidates {
        game.try_action(candidate.action).expect("MCTS candidate must be legal");
    }

    let stats = first.last_stats().expect("analysis stats");
    assert_eq!(stats.iterations, 16);
    assert_eq!(stats.root_actions, 720);
    assert_eq!(stats.expanded_root_actions, 16);
    assert_eq!(stats.nodes, 17);
    assert_eq!(stats.terminal_rollouts, 0);
    assert_eq!(stats.cutoff_rollouts, 16);
    assert_eq!(stats.simulated_actions, 16 * 8);
}

#[test]
fn framework_accepts_ranked_mcts_candidates() {
    let game = Game::default();
    let config = test_config(8, 4);
    let mut agent = MctsAgent::with_seed(config, 7).expect("agent");

    let analysis = analyze_agent(&game, &mut agent, NonZeroU8::new(2).expect("nonzero top k"))
        .expect("validated MCTS analysis");

    assert_eq!(analysis.candidates.len(), 2);
    assert!(analysis.candidates[0].score >= analysis.candidates[1].score);
    assert_eq!(agent.name(), "MCTS test-v1");
}
