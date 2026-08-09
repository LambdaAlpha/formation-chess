//! Tests for concrete examples in docs/notation.md, README.md, and core/README.md.
//! Engine-driven examples live in doc.txt; the tests below cover examples
//! that need direct API access.

use std::str::FromStr;

use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;

mod support;

/// The game state snapshot shared by notation.md (game state example) and
/// core/README.md (custom positions example, core/examples/readme_custom.rs).
const GAME_STATE_SNAPSHOT: &str = "行棋方：黑
红方：[弹 马]
黑方：[将 士 盾]
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
    "黑车进二",
    "一二三二",
    "红风平四拉",
    "红势四四和",
    "变化：[红卒平三 红风平四]\n胜负：未分",
    "黑将认负",
    "变化：[红车平五]\n胜负：未分",
    "变化：[红雷失 黑计失]\n胜负：未分",
    "变化：[红火平二 红车平三]\n胜负：未分",
    "变化：[红车四四]\n胜负：未分",
    "变化：[红车平二]\n胜负：红胜",
    "变化：[]\n胜负：红胜",
    "变化：[]\n胜负：和棋",
    "错误：path blocked, cannot reach destination",
];

const DOCUMENTED_DRAW_SWAP_REACTION: &str = "变化：[黑将二二 红势四四]\n胜负：和棋";
const QUICK_START_POOL_LINES: &str = "红方：[计 势 变 风 林 火 山 矛 盾 弹 雷 士 卒 马 车]\n黑方：[计 势 变 风 林 火 山 矛 盾 弹 雷 士 卒 马 车]";

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

/// The draw-exchange example is implemented by the direct reaction API test.
#[test]
fn documented_draw_swap_example_matches_both_notation_documents() {
    for source in
        [include_str!("../../docs/notation.md"), include_str!("../../docs/notation.zh-Hans.md")]
    {
        assert!(source.contains(DOCUMENTED_DRAW_SWAP_REACTION));
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

/// The draw-exchange example in notation.md: `红势四四和` must be a legal
/// draw on the documented board, and the reaction `变化：[黑将二二 红势四四]`
/// must swap the two pieces and declare a draw.
#[test]
fn notation_md_draw_swap_example_applies() {
    let game: Game = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[红将 一一 一一 一一 一一]
二[一一 红势 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 黑将 一一]
五[一一 一一 一一 一一 一一]
"
    .parse()
    .expect("parse game");

    let resolver = NotationResolver::new(&game);
    let action = resolver.parse_action("红势四四和").expect("parse action");
    let reaction = game.try_action(action).expect("draw action must be legal");
    assert_eq!(reaction.game_result, GameResult::Draw);
    assert_eq!(resolver.fmt_reaction(Ok(reaction.clone())), DOCUMENTED_DRAW_SWAP_REACTION);

    let changes = Board::normalize_changes(reaction.changes.as_slice());
    let mut applied = game.board().clone();
    applied.apply(&changes);
    assert_eq!(applied[(1, 1)].map(|p| p.name), Some('将'));
    assert_eq!(applied[(3, 3)].map(|p| p.name), Some('势'));
}
