//! Tests for the concrete examples that appear in docs/rules.md,
//! docs/notation.md, and README.md. Engine-driven examples live in
//! doc.txt; the tests below cover the examples that need direct API
//! access.

use std::str::FromStr;

use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;

mod common;

/// The game state snapshot shared by notation.md (game state example) and
/// README.md (custom positions example, examples/readme_custom.rs).
const GAME_STATE_SNAPSHOT: &str = "行棋方：黑
红方：[弹 马]
黑方：[将 士 盾]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
";

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
    let game =
        Game::from_str(GAME_STATE_SNAPSHOT).expect("notation.md snapshot must be a valid game");
    assert_eq!(game.to_string(), GAME_STATE_SNAPSHOT);
}

/// The quick start in README.md (examples/readme.rs): the standard setup
/// accepts the first two placement moves and prints the snapshot shown in
/// the README.
#[test]
fn readme_quick_start_example() {
    let mut game = Game::new(GameConfig::default()).expect("standard setup must be valid");
    for text in ["红将五十", "黑将五一"] {
        let action = NotationResolver::new(game.board(), game.phase())
            .parse_action(text)
            .unwrap_or_else(|e| panic!("parse {text}: {e}"));
        let reaction = game.action(action).unwrap_or_else(|e| panic!("action {text}: {e}"));
        assert_eq!(reaction.game_result.to_string(), "未分");
    }
    let expected = "行棋方：红
红方：[军 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹 雷]
黑方：[军 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹 雷]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路 六路 七路 八路 九路]
一[一一 一一 一一 一一 黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 一一 一一 一一 一一 一一]
六[一一 一一 一一 一一 一一 一一 一一 一一 一一]
七[一一 一一 一一 一一 一一 一一 一一 一一 一一]
八[一一 一一 一一 一一 一一 一一 一一 一一 一一]
九[一一 一一 一一 一一 一一 一一 一一 一一 一一]
十[一一 一一 一一 一一 红将 一一 一一 一一 一一]
";
    assert_eq!(game.to_string(), expected);
}

/// The custom positions example in README.md (examples/readme_custom.rs):
/// the snapshot parses into a validated game still in its placement phase.
#[test]
fn readme_custom_position_example() {
    let game: Game = GAME_STATE_SNAPSHOT.parse().expect("README.md snapshot must be a valid game");
    assert_eq!(game.phase(), Phase::Place);
}

/// The cyclic swap example in notation.md: `变化：[一二三四 三四一二]`
/// applied to a board must swap the two pieces.
#[test]
fn notation_md_swap_example_applies() {
    let board: Board = "零[一路 二路 三路 四路 五路]
一[一一 一一 一一 一一 一一]
二[红卒 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 红马 一一 一一]
五[一一 一一 一一 一一 一一]
"
    .parse()
    .expect("parse board");

    let reaction = NotationResolver::movement(&board)
        .parse_reaction("变化：[一二三四 三四一二]\n胜负：未分")
        .expect("parse reaction")
        .expect("reaction is a success");

    let changes = Board::normalize_changes(&reaction.changes);
    let mut applied = board;
    applied.apply(&changes);
    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(2, 3)].map(|p| p.name), Some('卒'));
}
