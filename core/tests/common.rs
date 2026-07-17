use std::str::FromStr;

use formation_chess_core::action::Reaction;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;

struct TestCase {
    title: String,
    game: Game,
    actions: Vec<String>,
    expected_state: Game,
    expected_result: String,
}

pub fn run_tests(data: &str, filepath: &str) {
    if data.contains("__GEN__") {
        let content = std::fs::read_to_string(filepath).unwrap_or_else(|e| {
            panic!("failed to read test file {filepath}: {e}");
        });
        let updated = gen_replacements(&content);
        std::fs::write(filepath, updated.as_bytes())
            .unwrap_or_else(|e| panic!("failed to write test file {filepath}: {e}"));
        // Fail loudly: a leftover __GEN__ marker must never make the suite
        // pass without running any assertions.
        panic!("GEN: wrote {filepath}; review the generated data and re-run the tests");
    }

    let cases = parse_test_file(data);
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
    let mut last_result = None;

    for action_str in &case.actions {
        let action = NotationResolver::new(game.board())
            .parse_action(action_str)
            .map_err(|e| format!("parse action: {e}"))?;
        pre_board = game.board().clone();
        last_result = Some(game.action(action));
    }

    let actual =
        last_result.unwrap_or(Ok(Reaction { changes: Vec::new(), game_result: game.result() }));

    let expected = NotationResolver::new(&pre_board)
        .parse_reaction(&case.expected_result)
        .map_err(|e| format!("parse result: {e}"))?;

    // when the expected result is an error, the failing action must leave the
    // game state unchanged (compared against the state after any preceding
    // successful actions)
    if expected.is_err() {
        assert_game_eq(&game, &case.expected_state)?;
    }

    compare_results(&pre_board, &actual, &expected)?;

    // Round-trip: formatting the actual result and re-parsing it must
    // resolve to the same changes.
    let formatted = NotationResolver::new(&pre_board).fmt_reaction(actual.clone());
    let reparsed = NotationResolver::new(&pre_board)
        .parse_reaction(&formatted)
        .map_err(|e| format!("reparse formatted result `{formatted}`: {e}"))?;
    compare_results(&pre_board, &actual, &reparsed)?;

    if let Ok(result) = &actual {
        assert_game_eq(&game, &case.expected_state)?;

        // The receiver resolves piece-based changes into position-based
        // changes and applies them; this must reproduce the final board.
        let changes = pre_board
            .resolve_changes(&result.changes)
            .map_err(|e| format!("resolve changes: {e}"))?;
        let mut board = pre_board.clone();
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
    board: &Board, actual: &Result<Reaction, String>, expected: &Result<Reaction, String>,
) -> Result<(), String> {
    match (actual, expected) {
        (Err(a), Err(e)) => {
            if a != e {
                return Err(format!("error mismatch:\n  expected: {e}\n  actual:   {a}"));
            }
        },
        (Ok(a), Ok(e)) => {
            let a_changes = board
                .resolve_changes(&a.changes)
                .map_err(|e| format!("resolve actual changes: {e}"))?;
            let e_changes = board
                .resolve_changes(&e.changes)
                .map_err(|e| format!("resolve expected changes: {e}"))?;
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

fn parse_test_file(data: &str) -> Vec<TestCase> {
    let mut cases = Vec::new();
    for block in data.split("=====").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let sections: Vec<&str> = block.split("-----").map(|s| s.trim()).collect();
        assert_eq!(
            sections.len(),
            5,
            "malformed test block with {} sections (expected 5):\n{block}",
            sections.len()
        );
        let title = sections[0].to_string();
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

fn gen_replacements(content: &str) -> String {
    let blocks: Vec<&str> = content.split("=====").collect();
    let mut result = String::new();

    for (i, block) in blocks.iter().enumerate() {
        if i == 0 {
            result.push_str(block);
            continue;
        }
        result.push_str("=====");

        if !block.contains("__GEN__") {
            result.push_str(block);
            continue;
        }

        let sections: Vec<&str> = block.split("-----").map(|s| s.trim()).collect();
        if sections.len() != 5 {
            result.push_str(block);
            continue;
        }

        let state = sections[1];
        let action_strs: Vec<String> =
            sections[2].lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        let notation = generate_notation(state, &action_strs);
        result.push_str(&block.replace("__GEN__", &notation));
    }

    result
}

fn generate_notation(state_str: &str, action_strs: &[String]) -> String {
    let mut game = Game::from_str(state_str).expect("parse game state for GEN");
    let mut pre_board = game.board().clone();
    let mut result = Ok(Reaction { changes: Vec::new(), game_result: game.result() });
    for action_str in action_strs {
        let action = NotationResolver::new(game.board())
            .parse_action(action_str)
            .expect("parse action for GEN");
        pre_board = game.board().clone();
        result = game.action(action);
    }
    NotationResolver::new(&pre_board).fmt_reaction(result)
}
