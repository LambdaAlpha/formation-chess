//! Runs the txt-based specification test suites through the shared harness
//! in `common.rs`. One test function per data file.

mod common;

#[test]
fn test_move() {
    common::run_tests(
        include_str!("move.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/move.txt"),
    );
}

#[test]
fn test_capture() {
    common::run_tests(
        include_str!("capture.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/capture.txt"),
    );
}

#[test]
fn test_push() {
    common::run_tests(
        include_str!("push.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/push.txt"),
    );
}

#[test]
fn test_placement() {
    common::run_tests(
        include_str!("placement.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/placement.txt"),
    );
}

#[test]
fn test_formation() {
    common::run_tests(
        include_str!("formation.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formation.txt"),
    );
}

#[test]
fn test_game_end() {
    common::run_tests(
        include_str!("game_end.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/game_end.txt"),
    );
}

#[test]
fn test_pass_resign() {
    common::run_tests(
        include_str!("pass_resign.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pass_resign.txt"),
    );
}

#[test]
fn test_draw() {
    common::run_tests(
        include_str!("draw.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/draw.txt"),
    );
}

#[test]
fn test_white() {
    common::run_tests(
        include_str!("white.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/white.txt"),
    );
}
