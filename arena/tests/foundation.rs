use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::analyze_prepared;
use formation_chess_agent::prepare_turn;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::Matchup;
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
