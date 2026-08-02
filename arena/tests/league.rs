use std::num::NonZeroU32;

use formation_chess_agent::ActionSelectionPolicy;
use formation_chess_agent::MinConfig;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::MinAgentFactory;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::RoundRobinLeague;
use formation_chess_arena::RoundRobinParticipant;

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn run_config() -> GameRunConfig {
    GameRunConfig::with_action_selection(NonZeroU32::MIN, ActionSelectionPolicy::Best)
}

#[test]
fn league_manifest_records_every_pair_descriptor_and_seed() {
    let mut custom_config = MinConfig::best();
    custom_config.config_id = "custom".to_owned();
    let custom_factory = MinAgentFactory::new(custom_config).expect("valid custom Min config");
    let participants = vec![
        RoundRobinParticipant::new(participant("random/baseline"), Box::new(RandomAgentFactory)),
        RoundRobinParticipant::new(participant("best"), Box::new(MinAgentFactory::best())),
        RoundRobinParticipant::new(participant("custom"), Box::new(custom_factory)),
    ];
    let league = RoundRobinLeague::new(
        participants,
        NonZeroU32::new(2).expect("nonzero pair count"),
        71,
        run_config(),
    )
    .expect("valid round-robin league");

    assert_eq!(league.matchup_count(), 3);
    assert_eq!(league.total_games(), 12);
    let manifest = league.manifest();
    assert_eq!(manifest.matchup_count, 3);
    assert_eq!(manifest.total_games, 12);
    assert_eq!(manifest.participants[0].agent.kind, "random");
    assert_eq!(manifest.participants[1].agent.display_name, "Min AI best-v1");
    assert_eq!(manifest.participants[2].agent.parameters["config"]["config_id"], "custom");
    assert_eq!(manifest.matchups[0].participant_a, "random/baseline");
    assert_eq!(manifest.matchups[0].participant_b, "best");
    assert_eq!(manifest.matchups[1].participant_a, "random/baseline");
    assert_eq!(manifest.matchups[1].participant_b, "custom");
    assert_eq!(manifest.matchups[2].participant_a, "best");
    assert_eq!(manifest.matchups[2].participant_b, "custom");
    assert_eq!(manifest.matchups[0].dataset_directory, "matchup-000000");
    assert_eq!(manifest.matchups[1].dataset_directory, "matchup-000001");
    assert_eq!(manifest.matchups[2].dataset_directory, "matchup-000002");
    assert_ne!(manifest.matchups[0].root_seed, manifest.matchups[1].root_seed);
    assert_ne!(manifest.matchups[1].root_seed, manifest.matchups[2].root_seed);
    assert_eq!(manifest, league.manifest());
}

#[test]
fn league_rejects_too_few_and_duplicate_participants() {
    let too_few = RoundRobinLeague::new(
        vec![RoundRobinParticipant::new(participant("only"), Box::new(RandomAgentFactory))],
        NonZeroU32::MIN,
        1,
        run_config(),
    );
    assert!(too_few.is_err(), "one participant cannot form a matchup");

    let duplicate = RoundRobinLeague::new(
        vec![
            RoundRobinParticipant::new(participant("same"), Box::new(RandomAgentFactory)),
            RoundRobinParticipant::new(participant("same"), Box::new(MinAgentFactory::best())),
        ],
        NonZeroU32::MIN,
        1,
        run_config(),
    );
    assert!(duplicate.is_err(), "duplicate participant IDs must be rejected");
}
