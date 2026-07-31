#![allow(dead_code)]

use std::collections::HashSet;
use std::str::FromStr;

use formation_chess_core::action::Reaction;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;

struct TestCase {
    title: String,
    game: Game,
    actions: Vec<String>,
    expected_state: Game,
    expected_result: String,
}

pub fn run_tests(data: &str) {
    let cases = parse_test_file(data);
    assert!(!cases.is_empty(), "test fixture must contain at least one case");
    let mut titles = HashSet::new();
    for case in &cases {
        assert!(titles.insert(case.title.as_str()), "duplicate test title: {}", case.title);
    }
    let mut passed = 0;
    let mut failed = 0;
    for case in &cases {
        match run_case(case) {
            Ok(()) => {
                passed += 1;
                println!("  PASS: {}", case.title);
            },
            Err(e) => {
                failed += 1;
                println!("  FAIL: {} — {e}", case.title);
            },
        }
    }
    println!("  {passed} passed, {failed} failed");
    if failed > 0 {
        panic!("{failed} test(s) failed");
    }
}

fn run_case(case: &TestCase) -> Result<(), String> {
    let mut game = case.game.clone();
    let mut pre_board = game.board().clone();
    let mut pre_phase = game.phase();
    let mut last_result = None;

    for action_str in &case.actions {
        let action = NotationResolver::new(game.board(), game.phase())
            .parse_action(action_str)
            .map_err(|e| format!("parse action: {e}"))?;
        pre_board = game.board().clone();
        pre_phase = game.phase();
        last_result = Some(game.action(action));
    }

    let actual =
        last_result.unwrap_or(Ok(Reaction { changes: Vec::new(), game_result: game.result() }));

    let expected = NotationResolver::new(&pre_board, pre_phase)
        .parse_reaction(&case.expected_result)
        .map_err(|e| format!("parse result: {e}"))?;

    // when the expected result is an error, the failing action must leave the
    // game state unchanged (compared against the state after any preceding
    // successful actions)
    if expected.is_err() {
        assert_game_eq(&game, &case.expected_state)?;
    }

    compare_results(&actual, &expected)?;

    // Round-trip: formatting the actual result and re-parsing it must
    // resolve to the same changes.
    let formatted = NotationResolver::new(&pre_board, pre_phase).fmt_reaction(actual.clone());
    let reparsed = NotationResolver::new(&pre_board, pre_phase)
        .parse_reaction(&formatted)
        .map_err(|e| format!("reparse formatted result `{formatted}`: {e}"))?;
    compare_results(&actual, &reparsed)?;

    if let Ok(result) = &actual {
        assert_game_eq(&game, &case.expected_state)?;

        // The receiver resolves piece-based changes into position-based
        // changes and applies them; this must reproduce the final board.
        let changes = Board::normalize_changes(&result.changes);
        let mut board = pre_board;
        board.apply(&changes);
        let applied = format!("{:#}", board);
        let expected_board = format!("{:#}", case.expected_state.board());
        if applied != expected_board {
            return Err(format!(
                "applied changes mismatch:\n# expected:\n{expected_board}\n# applied:\n{applied}"
            ));
        }
    }

    Ok(())
}

fn compare_results(
    actual: &Result<Reaction, String>, expected: &Result<Reaction, String>,
) -> Result<(), String> {
    match (actual, expected) {
        (Err(a), Err(e)) => {
            if a != e {
                return Err(format!("error mismatch:\n  expected: {e}\n  actual:   {a}"));
            }
        },
        (Ok(a), Ok(e)) => {
            let a_changes = Board::normalize_changes(&a.changes);
            let e_changes = Board::normalize_changes(&e.changes);
            if a_changes != e_changes {
                return Err(format!(
                    "changes mismatch:\n  expected: {e_changes:?}\n  actual:   {a_changes:?}"
                ));
            }
            if a.game_result != e.game_result {
                return Err(format!(
                    "game_result mismatch: expected {}, got {}",
                    e.game_result, a.game_result
                ));
            }
        },
        (Err(_), Ok(_)) => return Err("expected error but action succeeded".to_string()),
        (Ok(_), Err(_)) => return Err("expected success but action failed".to_string()),
    }
    Ok(())
}

fn assert_game_eq(actual: &Game, expected: &Game) -> Result<(), String> {
    if actual.player() != expected.player() {
        return Err(format!("player: {} != {}", actual.player(), expected.player()));
    }
    if actual.red_pool() != expected.red_pool() {
        return Err(format!(
            "red_pool mismatch: expected [{}], actual [{}]",
            format_pool(expected.red_pool()),
            format_pool(actual.red_pool())
        ));
    }
    if actual.black_pool() != expected.black_pool() {
        return Err(format!(
            "black_pool mismatch: expected [{}], actual [{}]",
            format_pool(expected.black_pool()),
            format_pool(actual.black_pool())
        ));
    }
    if actual.phase() != expected.phase() {
        return Err(format!("phase: {:?} != {:?}", actual.phase(), expected.phase()));
    }
    if actual.white_pool() != expected.white_pool() {
        return Err(format!("white_count: {} != {}", actual.white_pool(), expected.white_pool()));
    }
    if actual.result() != expected.result() {
        return Err(format!("result: {} != {}", actual.result(), expected.result()));
    }

    let actual_board = format!("{:#}", actual.board());
    let expected_board = format!("{:#}", expected.board());
    if actual_board != expected_board {
        return Err(format!(
            "board mismatch:\n# expected:\n{expected_board}\n# actual:\n{actual_board}"
        ));
    }

    Ok(())
}

fn format_pool(pool: &[Piece]) -> String {
    pool.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")
}

fn parse_test_file(data: &str) -> Vec<TestCase> {
    let mut cases = Vec::new();
    for block in data.split("=====").map(str::trim).filter(|s| !s.is_empty()) {
        let sections: Vec<&str> = block.split("-----").map(str::trim).collect();
        assert_eq!(
            sections.len(),
            5,
            "malformed test block with {} sections (expected 5):\n{block}",
            sections.len()
        );
        let title = sections[0].to_string();
        validate_title(&title);
        let game = Game::from_str(sections[1])
            .unwrap_or_else(|e| panic!("`{title}`: initial state parse error: {e}"));
        let actions: Vec<String> =
            sections[2].lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        let expected_state = Game::from_str(sections[3])
            .unwrap_or_else(|e| panic!("`{title}`: expected state parse error: {e}"));
        let result = sections[4];

        cases.push(TestCase {
            title,
            game,
            actions,
            expected_state,
            expected_result: result.to_string(),
        });
    }
    cases
}

fn validate_title(title: &str) {
    const ENGLISH_PIECE_NAMES: &[&str] = &[
        "general", "army", "agent", "spy", "scholar", "pawn", "rook", "horse", "wind", "mountain",
        "fire", "forest", "spear", "shield", "shell", "mine",
    ];
    assert!(!title.is_empty(), "test title must not be empty");
    assert!(!title.contains('\n'), "test title must be a single line: {title:?}");
    let words = title
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty());
    for word in words {
        assert!(
            !ENGLISH_PIECE_NAMES.contains(&word.to_ascii_lowercase().as_str()),
            "test title must describe an ability, formation, or action rather than piece config: {title}"
        );
    }
    for piece in Piece::RED_PLAYER_PIECES {
        assert!(
            !title.contains(piece.name),
            "test title must describe an ability, formation, or action rather than piece config: {title}"
        );
    }
}
