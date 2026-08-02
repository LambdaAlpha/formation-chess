use std::fs;
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use formation_chess_agent::MinConfig;
use formation_chess_arena::ActionSelectionPolicyRecord;
use formation_chess_arena::DatasetSummary;
use formation_chess_arena::GAME_METRICS_FILE_NAME;
use formation_chess_arena::JsonlDatasetReader;
use formation_chess_arena::LEAGUE_MANIFEST_FILE_NAME;
use formation_chess_arena::LeagueManifest;
use formation_chess_arena::MANIFEST_FILE_NAME;
use formation_chess_arena::ReplayVerifier;
use formation_chess_arena::SUMMARY_FILE_NAME;
use formation_chess_arena::record::ScheduleRecord;

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("formation-chess-arena-cli-{label}-{}-{unique}", std::process::id()));
        fs::create_dir(&path).expect("create CLI test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).expect("remove CLI test directory");
        }
    }
}

fn arena_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_formation-chess-arena"))
}

fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_shallow_min_config(directory: &TestDirectory, config_id: &str) -> PathBuf {
    let mut config = MinConfig::best();
    config_id.clone_into(&mut config.config_id);
    config.placement_search.max_depth = NonZeroU8::MIN;
    config.placement_search.max_nodes = NonZeroU32::new(64).expect("nonzero node budget");
    config.placement_search.root_width = NonZeroU8::MIN;
    config.placement_search.opponent_width = NonZeroU8::MIN;
    config.placement_search.response_width = NonZeroU8::MIN;
    config.movement_search.max_depth = NonZeroU8::MIN;
    config.movement_search.max_nodes = NonZeroU32::new(64).expect("nonzero node budget");
    config.movement_search.opponent_width = NonZeroU8::MIN;
    config.movement_search.response_width = NonZeroU8::MIN;
    config.validate().expect("valid shallow Min config");

    let path = directory.path().join(format!("{config_id}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&config).expect("serialize shallow Min config"))
        .expect("write shallow Min config");
    path
}

#[test]
fn cli_runs_verifies_and_analyzes_a_configured_dataset() {
    let directory = TestDirectory::new("end-to-end");
    let dataset_root = directory.path().join("dataset");
    let min_config_path = write_shallow_min_config(&directory, "cli");

    let run_output = arena_command()
        .arg("run")
        .arg("--output")
        .arg(&dataset_root)
        .arg("--seed")
        .arg("73")
        .arg("--fixed")
        .arg("1")
        .arg("--movement-limit")
        .arg("1")
        .arg("--participant-a")
        .arg("random_a")
        .arg("--participant-b")
        .arg("min_b")
        .arg("--agent-a")
        .arg("random")
        .arg("--agent-b")
        .arg(format!("min:{}", min_config_path.display()))
        .output()
        .expect("run arena CLI");
    assert_success(&run_output, "run");
    assert!(
        String::from_utf8_lossy(&run_output.stdout).contains("wrote 1 games"),
        "run output must report the persisted game count"
    );
    assert!(dataset_root.join(MANIFEST_FILE_NAME).is_file(), "manifest must be written");

    let mut reader = JsonlDatasetReader::open(&dataset_root).expect("open CLI dataset");
    assert_eq!(reader.manifest().root_seed, 73);
    assert_eq!(reader.manifest().schedule, ScheduleRecord::Fixed { games: 1 });
    assert_eq!(reader.manifest().game_run_config.max_movement_actions, 1);
    assert_eq!(
        reader.manifest().game_run_config.action_selection,
        ActionSelectionPolicyRecord::RankSoftmax { top_k: 4, temperature: 0.5 }
    );
    assert_eq!(reader.manifest().participant_a.id, "random_a");
    assert_eq!(reader.manifest().participant_b.id, "min_b");
    assert_eq!(reader.manifest().participant_a.agent.kind, "random");
    assert_eq!(reader.manifest().participant_b.agent.kind, "min");
    assert_eq!(reader.manifest().participant_b.agent.display_name, "Min AI cli-v1");
    assert_eq!(reader.manifest().participant_b.agent.parameters["config"]["config_id"], "cli");
    assert!(
        reader.manifest().participant_b.agent.parameters["config_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64),
        "Min descriptor must retain the complete config identity"
    );
    let record = reader.next().expect("one game record").expect("valid game record");
    assert!(record.actions.iter().all(|action| (1 ..= 4).contains(&action.candidate_rank)));
    ReplayVerifier::verify(&record).expect("CLI game must replay");
    assert!(reader.next().is_none(), "dataset must contain exactly one game");

    let verify_output =
        arena_command().arg("verify").arg(&dataset_root).output().expect("verify arena dataset");
    assert_success(&verify_output, "verify");
    assert!(
        String::from_utf8_lossy(&verify_output.stdout).contains("verified 1 games"),
        "verify output must report the replayed game count"
    );
    assert!(
        !dataset_root.join(GAME_METRICS_FILE_NAME).exists(),
        "verify must not create analysis output"
    );

    let stats_output =
        arena_command().arg("stats").arg(&dataset_root).output().expect("analyze arena dataset");
    assert_success(&stats_output, "stats");
    assert!(
        String::from_utf8_lossy(&stats_output.stdout).contains("analyzed 1 games"),
        "stats output must report the analyzed game count"
    );
    assert!(dataset_root.join(GAME_METRICS_FILE_NAME).is_file(), "stats must create per-game CSV");
    let summary_path = dataset_root.join(SUMMARY_FILE_NAME);
    let summary = serde_json::from_slice::<DatasetSummary>(
        &fs::read(&summary_path).expect("read CLI summary"),
    )
    .expect("parse CLI summary");
    assert_eq!(summary.games, 1);
}
#[test]
fn cli_runs_random_and_min_configs_as_a_round_robin_league() {
    let directory = TestDirectory::new("league");
    let league_root = directory.path().join("league");
    let alpha_config_path = write_shallow_min_config(&directory, "alpha");
    let beta_config_path = write_shallow_min_config(&directory, "beta");

    let output = arena_command()
        .arg("league")
        .arg("--output")
        .arg(&league_root)
        .arg("--seed")
        .arg("101")
        .arg("--paired")
        .arg("1")
        .arg("--movement-limit")
        .arg("1")
        .arg("--participant")
        .arg("random=random")
        .arg("--participant")
        .arg(format!("alpha=min:{}", alpha_config_path.display()))
        .arg("--participant")
        .arg(format!("beta=min:{}", beta_config_path.display()))
        .output()
        .expect("run Arena league CLI");
    assert_success(&output, "league");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("wrote 3 matchups and 6 games"),
        "league output must report matchup and game counts"
    );

    let manifest = serde_json::from_slice::<LeagueManifest>(
        &fs::read(league_root.join(LEAGUE_MANIFEST_FILE_NAME)).expect("read league manifest"),
    )
    .expect("parse league manifest");
    assert_eq!(manifest.root_seed, 101);
    assert_eq!(manifest.pairs_per_matchup, 1);
    assert_eq!(manifest.matchup_count, 3);
    assert_eq!(manifest.total_games, 6);
    assert_eq!(manifest.participants.len(), 3);
    assert_eq!(manifest.participants[0].agent.kind, "random");
    assert_eq!(manifest.participants[1].agent.kind, "min");
    assert_eq!(manifest.participants[1].agent.parameters["config"]["config_id"], "alpha");
    assert_eq!(manifest.participants[2].agent.parameters["config"]["config_id"], "beta");
    assert_eq!(manifest.matchups.len(), 3);
    assert_ne!(manifest.matchups[0].root_seed, manifest.matchups[1].root_seed);

    for matchup in &manifest.matchups {
        let dataset_root = league_root.join(&matchup.dataset_directory);
        let mut reader = JsonlDatasetReader::open(&dataset_root).expect("open league dataset");
        assert_eq!(reader.manifest().root_seed, matchup.root_seed);
        assert_eq!(reader.manifest().participant_a.id, matchup.participant_a);
        assert_eq!(reader.manifest().participant_b.id, matchup.participant_b);
        for record in &mut reader {
            ReplayVerifier::verify(&record.expect("valid league game record"))
                .expect("league game must replay");
        }
        assert_eq!(reader.read_games(), 2);
    }
}

#[test]
fn cli_rejects_fixed_and_paired_counts_together_before_writing() {
    let directory = TestDirectory::new("invalid-schedule");
    let dataset_root = directory.path().join("dataset");

    let output = arena_command()
        .arg("run")
        .arg("--output")
        .arg(&dataset_root)
        .arg("--seed")
        .arg("1")
        .arg("--fixed")
        .arg("1")
        .arg("--paired")
        .arg("1")
        .arg("--movement-limit")
        .arg("1")
        .arg("--participant-a")
        .arg("random_a")
        .arg("--participant-b")
        .arg("random_b")
        .output()
        .expect("run invalid arena CLI command");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("exactly one of `--fixed` or `--paired`"),
        "usage error must explain the mutually exclusive schedule options"
    );
    assert!(!dataset_root.exists(), "invalid arguments must not create output");
}
