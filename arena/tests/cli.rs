use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use formation_chess_arena::DatasetSummary;
use formation_chess_arena::GAME_METRICS_FILE_NAME;
use formation_chess_arena::JsonlDatasetReader;
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

#[test]
fn cli_runs_verifies_and_analyzes_a_random_dataset() {
    let directory = TestDirectory::new("end-to-end");
    let dataset_root = directory.path().join("dataset");

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
        .arg("random_b")
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
    assert_eq!(reader.manifest().participant_a.id, "random_a");
    assert_eq!(reader.manifest().participant_b.id, "random_b");
    assert_eq!(reader.manifest().participant_a.agent.kind, "random");
    assert_eq!(reader.manifest().participant_b.agent.kind, "random");
    let record = reader.next().expect("one game record").expect("valid game record");
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
