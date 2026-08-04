use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::MCTS_CONFIG_HASH_ALGORITHM;
use formation_chess_agent::MCTS_CONFIG_HASH_FORMAT_VERSION;
use formation_chess_agent::MCTS_CONFIG_SCHEMA_VERSION;
use formation_chess_agent::MIN_CONFIG_HASH_ALGORITHM;
use formation_chess_agent::MIN_CONFIG_HASH_FORMAT_VERSION;
use formation_chess_agent::MIN_CONFIG_SCHEMA_VERSION;
use formation_chess_agent::MIN_EVALUATION_MODEL_VERSION;
use formation_chess_agent::analyze_prepared;
use formation_chess_agent::prepare_turn;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::Matchup;
use formation_chess_arena::MctsAgentFactory;
use formation_chess_arena::MinAgentFactory;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::SEED_DERIVATION_VERSION;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_core::game::Game;

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero schedule count")
}

#[test]
fn random_factory_describes_and_reproduces_seeded_agents() {
    let factory = RandomAgentFactory;
    let descriptor = factory.descriptor();

    assert_eq!(descriptor.kind, "random");
    assert_eq!(descriptor.display_name, "Random");
    assert_eq!(descriptor.implementation_version, formation_chess_agent::VERSION);
    assert!(descriptor.parameters.is_empty());

    let game = Game::default();
    let prepared = prepare_turn(&game).expect("prepared placement turn");
    let mut first = factory.create(42);
    let mut second = factory.create(42);
    let first_analysis = analyze_prepared(&prepared, first.as_mut(), NonZeroU8::new(4).unwrap())
        .expect("first seeded analysis");
    let second_analysis = analyze_prepared(&prepared, second.as_mut(), NonZeroU8::new(4).unwrap())
        .expect("second seeded analysis");

    assert_eq!(first_analysis.candidates, second_analysis.candidates);
}

#[test]
fn mcts_factory_records_the_complete_validated_configuration() {
    let factory = MctsAgentFactory::baseline();
    let descriptor = factory.descriptor();

    assert_eq!(descriptor.kind, "mcts");
    assert_eq!(descriptor.display_name, "MCTS baseline-v1");
    assert_eq!(descriptor.implementation_version, formation_chess_agent::VERSION);
    assert_eq!(descriptor.parameters.len(), 5);
    assert_eq!(
        descriptor.parameters["config"],
        serde_json::to_value(factory.config()).expect("serialize MCTS config")
    );
    assert_eq!(
        descriptor.parameters["config_sha256"],
        factory.config().sha256().expect("hash MCTS config")
    );
    assert_eq!(descriptor.parameters["config_hash_algorithm"], MCTS_CONFIG_HASH_ALGORITHM);
    assert_eq!(
        descriptor.parameters["config_hash_format_version"],
        MCTS_CONFIG_HASH_FORMAT_VERSION
    );
    assert_eq!(descriptor.parameters["config_schema_version"], MCTS_CONFIG_SCHEMA_VERSION);

    let agent = factory.create(99);
    assert_eq!(agent.name(), "MCTS baseline-v1");
}

#[test]
fn min_factory_records_the_complete_validated_configuration() {
    let factory = MinAgentFactory::best();
    let descriptor = factory.descriptor();

    assert_eq!(descriptor.kind, "min");
    assert_eq!(descriptor.display_name, "Min AI best-v1");
    assert_eq!(descriptor.implementation_version, formation_chess_agent::VERSION);
    assert_eq!(descriptor.parameters.len(), 6);
    assert_eq!(
        descriptor.parameters["config"],
        serde_json::to_value(factory.config()).expect("serialize Min config")
    );
    assert_eq!(
        descriptor.parameters["config_sha256"],
        factory.config().sha256().expect("hash Min config")
    );
    assert_eq!(descriptor.parameters["config_hash_algorithm"], MIN_CONFIG_HASH_ALGORITHM);
    assert_eq!(descriptor.parameters["config_hash_format_version"], MIN_CONFIG_HASH_FORMAT_VERSION);
    assert_eq!(descriptor.parameters["config_schema_version"], MIN_CONFIG_SCHEMA_VERSION);
    assert_eq!(descriptor.parameters["evaluation_model_version"], MIN_EVALUATION_MODEL_VERSION);

    let agent = factory.create(99);
    assert_eq!(agent.name(), "Min best-v1");
}

#[test]
fn participant_and_matchup_ids_are_unambiguous() {
    ParticipantId::new("").expect_err("empty participant id must be rejected");
    ParticipantId::new("agent a").expect_err("whitespace in participant id must be rejected");

    let agent_a = participant("agent_a");
    let duplicate = Matchup::new(agent_a.clone(), agent_a);
    duplicate.expect_err("matchup participants must be distinct");
}

#[test]
fn fixed_schedule_keeps_seats_and_derives_each_game_independently() {
    let matchup =
        Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup");
    let mut schedule = Schedule::new(matchup, ScheduleMode::Fixed { games: nonzero(3) }, 7);

    assert_eq!(schedule.total_games(), 3);
    assert_eq!(schedule.size_hint(), (3, Some(3)));
    let plans = schedule.by_ref().collect::<Vec<_>>();
    assert_eq!(schedule.size_hint(), (0, Some(0)));

    for (game_id, plan) in plans.iter().enumerate() {
        assert_eq!(plan.game_id, game_id as u64);
        assert_eq!(plan.pair_id, None);
        assert_eq!(plan.game_in_pair, None);
        assert_eq!(plan.red.as_str(), "agent_a");
        assert_eq!(plan.black.as_str(), "agent_b");
    }
    assert_ne!(plans[0].scenario_seed, plans[1].scenario_seed);
    assert_ne!(plans[0].red_agent_seed, plans[1].red_agent_seed);
    assert_ne!(plans[0].black_agent_seed, plans[1].black_agent_seed);
}

#[test]
fn paired_schedule_swaps_seats_and_preserves_participant_seeds() {
    let matchup =
        Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup");
    let plans =
        Schedule::new(matchup, ScheduleMode::Paired { pairs: nonzero(2) }, 11).collect::<Vec<_>>();

    assert_eq!(SEED_DERIVATION_VERSION, 1);
    assert_eq!(plans.len(), 4);
    assert_eq!(plans[0].pair_id, Some(0));
    assert_eq!(plans[1].pair_id, Some(0));
    assert_eq!(plans[0].game_in_pair, Some(0));
    assert_eq!(plans[1].game_in_pair, Some(1));
    assert_eq!(plans[0].red.as_str(), "agent_a");
    assert_eq!(plans[0].black.as_str(), "agent_b");
    assert_eq!(plans[1].red.as_str(), "agent_b");
    assert_eq!(plans[1].black.as_str(), "agent_a");
    assert_eq!(plans[0].scenario_seed, 0x43b6_71e5_1afb_d886);
    assert_eq!(plans[0].red_agent_seed, 0xc0ac_21cb_8a12_ff1f);
    assert_eq!(plans[0].black_agent_seed, 0xaafb_2d7f_e878_09dc);
    assert_eq!(plans[0].scenario_seed, plans[1].scenario_seed);
    assert_eq!(plans[0].red_agent_seed, plans[1].black_agent_seed);
    assert_eq!(plans[0].black_agent_seed, plans[1].red_agent_seed);
    assert_ne!(plans[0].scenario_seed, plans[2].scenario_seed);
}
