//! Tests for the concrete examples that appear in docs/rules.md and
//! docs/notation.md. Engine-driven examples live in doc.txt; the tests
//! below cover the examples that need direct API access.

use std::str::FromStr;

use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;

mod common;

#[test]
fn test_doc_examples() {
    common::run_tests(
        include_str!("doc.txt"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doc.txt"),
    );
}

/// The game state example in notation.md must parse, validate, and format
/// back to the exact same text.
#[test]
fn notation_md_game_state_example_round_trips() {
    let snapshot = "行棋方：黑
红方：[炮 马]
黑方：[将 犬 盾]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
";
    let game = Game::from_str(snapshot).expect("notation.md snapshot must be a valid game");
    assert_eq!(game.to_string(), snapshot);
}

/// The cyclic swap example in notation.md: `变化：[一二三四 三四一二]`
/// applied to a board must swap the two pieces.
#[test]
fn notation_md_swap_example_applies() {
    let board: Board = "零[一一 二二 三三 四四 五五]
一[一一 一一 一一 一一 一一]
二[红卒 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 红马 一一 一一]
五[一一 一一 一一 一一 一一]
"
    .parse()
    .expect("parse board");

    let reaction = NotationResolver::new(&board)
        .parse_reaction("变化：[一二三四 三四一二]\n胜负：未分")
        .expect("parse reaction")
        .expect("reaction is a success");

    let changes = board.resolve_changes(&reaction.changes).expect("resolve changes");
    let mut applied = board.clone();
    applied.apply(&changes);
    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(2, 3)].map(|p| p.name), Some('卒'));
}
