//! Runs the text-protocol specification suites through the shared harness.
//! Each test function owns exactly one behavior-focused data file.

mod support;

#[test]
fn move_spec() {
    support::spec::run_tests(include_str!("move.txt"));
}

#[test]
fn capture_spec() {
    support::spec::run_tests(include_str!("capture.txt"));
}

#[test]
fn push_spec() {
    support::spec::run_tests(include_str!("push.txt"));
}

#[test]
fn placement_spec() {
    support::spec::run_tests(include_str!("placement.txt"));
}

#[test]
fn formation_spec() {
    support::spec::run_tests(include_str!("formation.txt"));
}

#[test]
fn game_end_spec() {
    support::spec::run_tests(include_str!("game_end.txt"));
}

#[test]
fn resign_spec() {
    support::spec::run_tests(include_str!("resign.txt"));
}

#[test]
fn draw_spec() {
    support::spec::run_tests(include_str!("draw.txt"));
}

#[test]
fn pull_spec() {
    support::spec::run_tests(include_str!("pull.txt"));
}
