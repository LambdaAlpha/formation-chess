//! Tests for concrete examples in docs/notation.md, README.md, and core/README.md.
//! Engine-driven examples live in doc.txt; the tests below cover examples
//! that need direct API access.

use std::str::FromStr;

use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;

mod support;

/// The game state snapshot shared by notation.md (game state example) and
/// core/README.md (custom positions example, core/examples/readme_custom.rs).
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

const DOCUMENTED_PROTOCOL_SNIPPETS: &[&str] = &[
    "红士三四",
    "黑将进二",
    "一二三二",
    "白子直二",
    "红军直四分",
    "红按兵",
    "黑认负",
    "变化：[红车平五]\n胜负：未分",
    "变化：[红雷失 黑车失]\n胜负：未分",
    "变化：[红风平二 红车平三]\n胜负：未分",
    "变化：[红车四四]\n胜负：未分",
    "变化：[红车平二]\n胜负：红胜",
    "变化：[]\n胜负：未分",
    "变化：[]\n胜负：红胜",
    "变化：[]\n胜负：和棋",
    "错误：path blocked, cannot reach destination",
];

const DOCUMENTED_SWAP_REACTION: &str = "变化：[一二三四 三四一二]\n胜负：未分";
const QUICK_START_POOL_LINES: &str = "红方：[军 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹 雷]\n黑方：[军 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹 雷]";

#[test]
fn text_protocol_examples() {
    support::spec::run_tests(include_str!("doc.txt"));
}

/// The protocol fixture and both notation documents share the same concrete examples.
#[test]
fn documented_protocol_examples_match_the_fixtures() {
    let sources = [
        ("doc.txt", include_str!("doc.txt")),
        ("notation.md", include_str!("../../docs/notation.md")),
        ("notation.zh-Hans.md", include_str!("../../docs/notation.zh-Hans.md")),
    ];

    for (name, source) in sources {
        for &snippet in DOCUMENTED_PROTOCOL_SNIPPETS {
            assert!(source.contains(snippet), "{name} is missing documented snippet: {snippet}");
        }
    }
}

/// The cyclic-swap example is implemented by the direct reaction API test.
#[test]
fn documented_swap_example_matches_both_notation_documents() {
    for source in
        [include_str!("../../docs/notation.md"), include_str!("../../docs/notation.zh-Hans.md")]
    {
        assert!(source.contains(DOCUMENTED_SWAP_REACTION));
    }
}

/// README snippets and the runnable example must use the game-aware resolver API.
#[test]
fn documented_readme_examples_use_the_game_aware_resolver() {
    let sources = [
        include_str!("../../README.md"),
        include_str!("../../README.zh-Hans.md"),
        include_str!("../../core/README.md"),
        include_str!("../../core/examples/readme.rs"),
    ];
    for source in sources {
        assert!(source.contains("NotationResolver::new(&game)"));
        assert!(source.contains("红将五十"));
        assert!(source.contains("黑将五一"));
    }
    let fixture = include_str!("doc.txt");
    assert!(fixture.contains("红将五十"));
    assert!(fixture.contains("黑将五一"));
    assert!(fixture.contains(QUICK_START_POOL_LINES));
    assert!(include_str!("../../core/README.md").contains(QUICK_START_POOL_LINES));
}

/// The custom-position snapshot stays synchronized across its documentation sources.
#[test]
fn documented_state_snapshot_matches_sources() {
    for (name, source) in [
        ("notation.md", include_str!("../../docs/notation.md")),
        ("notation.zh-Hans.md", include_str!("../../docs/notation.zh-Hans.md")),
        ("core/README.md", include_str!("../../core/README.md")),
        ("readme_custom.rs", include_str!("../../core/examples/readme_custom.rs")),
    ] {
        let source = source.replace("\r\n", "\n");
        assert!(source.contains(GAME_STATE_SNAPSHOT), "{name} is missing the documented snapshot");
    }
}

/// The game state example in notation.md must parse, validate, and format
/// back to the exact same text.
#[test]
fn notation_md_game_state_example_round_trips() {
    let game =
        Game::from_str(GAME_STATE_SNAPSHOT).expect("notation.md snapshot must be a valid game");
    assert_eq!(game.to_string(), GAME_STATE_SNAPSHOT);
}

/// The custom positions example in core/README.md (core/examples/readme_custom.rs):
/// the snapshot parses into a validated game still in its placement phase.
#[test]
fn readme_custom_position_example() {
    let game: Game =
        GAME_STATE_SNAPSHOT.parse().expect("core/README.md snapshot must be a valid game");
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

    let game = Game::new(GameConfig {
        board: board.clone(),
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Draw,
        ..GameConfig::default()
    })
    .expect("construct resolver game");
    let reaction = NotationResolver::new(&game)
        .parse_reaction(DOCUMENTED_SWAP_REACTION)
        .expect("parse reaction")
        .expect("reaction is a success");

    let changes = Board::normalize_changes(reaction.changes.as_slice());
    let mut applied = board;
    applied.apply(&changes);
    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(2, 3)].map(|p| p.name), Some('卒'));
}
