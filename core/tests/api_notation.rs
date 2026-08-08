use std::str::FromStr;

use formation_chess_core::action::Action;
use formation_chess_core::action::Move;
use formation_chess_core::action::PositionChange;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Player;

mod support;
use support::api::SIMPLE;
use support::api::SWAP_STATE;

#[test]
fn reaction_changes_resolve_and_apply() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(&game)
        .parse_reaction("变化：[红车二四 红马一二]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    let mut applied = board.clone();
    applied.apply(result.changes.as_slice());

    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(1, 3)].map(|p| p.name), Some('车'));
}

#[test]
fn reaction_change_order_is_independent() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");

    let a = NotationResolver::new(&game)
        .parse_reaction("变化：[红车二四 红马一二]\n胜负：未分")
        .expect("parse result")
        .expect("success result");
    let b = NotationResolver::new(&game)
        .parse_reaction("变化：[红马一二 红车二四]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(a.changes, b.changes);
}

#[test]
fn reaction_coordinates_resolve_a_cyclic_swap() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(&game)
        .parse_reaction("变化：[二四一二 一二一四]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    let mut applied = board.clone();
    applied.apply(result.changes.as_slice());

    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(0, 3)].map(|p| p.name), Some('车'));
    assert_eq!(applied[(1, 3)], None);
}

#[test]
fn reaction_removal_entry_clears_a_position() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(&game)
        .parse_reaction("变化：[红车失]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(result.changes.as_slice(), &[PositionChange {
        at: (0, 1),
        old: board.get((0, 1)),
        new: None
    }]);

    let mut applied = board.clone();
    applied.apply(result.changes.as_slice());
    assert_eq!(applied[(0, 1)], None);
}
#[test]
fn notation_rejects_an_ambiguous_identifier() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红卒进一").expect_err("must be ambiguous");
    assert!(err.contains("multiple 红卒"), "unexpected error: {err}");
}

#[test]
fn coordinate_notation_selects_a_unique_origin() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action = NotationResolver::new(&game).parse_action("一三直二").expect("parse action");
    game.action(action).expect("action");
    assert_eq!(game.board()[(0, 1)].map(|p| p.name), Some('卒'));
}

#[test]
fn state_parser_rejects_invalid_vital_configuration() {
    let state = "行棋方：红
红方：[将]
黑方：[卒]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("double vital must fail");
    };
    assert!(err.contains("at most one vital piece"), "unexpected error: {err}");
}

#[test]
fn placement_notation_rejects_relative_position() {
    // During placement phase, only name + absolute position is a valid
    // placement. Name + relative position (e.g. 红车进一) is rejected.
    let state = "行棋方：红
红方：[车]
黑方：[将]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红车进一").expect_err("must fail");
    assert!(err.contains("not on board"), "unexpected error: {err}");
}

#[test]
fn long_moves_on_tall_boards_format_and_parse() {
    let mut rows = String::new();
    for y in 0 .. 15u8 {
        let label = [
            "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "甲", "乙", "丙", "丁",
            "戊",
        ][y as usize];
        rows.push_str(label);
        rows.push('[');
        for x in 0 .. 5u8 {
            if x > 0 {
                rows.push(' ');
            }
            rows.push_str(match (x, y) {
                (0, 14) => "红车",
                (1, 14) => "红将",
                (4, 0) => "黑将",
                _ => "一一",
            });
        }
        rows.push_str("]\n");
    }
    let state = format!(
        "行棋方：红\n红方：[]\n黑方：[]\n胜负：未分\n棋盘：\n零[一路 二路 三路 四路 五路]\n{rows}"
    );
    let game = Game::from_str(&state).expect("parse");
    let action = Action::Push(Move { from: (0, 14), to: (0, 0) });
    let formatted = NotationResolver::new(&game).fmt_action(&action);
    assert_eq!(formatted, "红车进丁推");
    let reparsed = NotationResolver::new(&game).parse_action(&formatted).expect("parse action");
    let Action::Push(move_) = reparsed else {
        panic!("expected move, got {reparsed}");
    };
    assert_eq!((move_.from, move_.to), ((0, 14), (0, 0)));
}

#[test]
fn state_formatter_preserves_decided_result() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：红胜
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 红将]
";
    let game = Game::from_str(state).expect("decided game should parse");
    let display = format!("{game}");
    assert!(display.contains("胜负：红胜"), "expected 红胜 in display, got:\n{display}");
}

#[test]
fn state_parser_requires_result_after_vital_loss() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("unfinished result on a decided position must fail");
    };
    assert!(err.contains("validate_vital_result failed"), "unexpected error: {err}");
}

#[test]
fn state_parser_does_not_infer_draw_from_formation() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 红将 一一 一一]
四[一一 一一 一一 黑将 一一]
五[一一 一一 一一 一一 一一]
";
    Game::from_str(state).expect("generals in formation is no longer an automatic draw");
}

#[test]
fn notation_rejects_out_of_bounds_placement() {
    let state = "行棋方：红
红方：[车]
黑方：[卒]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红车九九").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_out_of_bounds_origin() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(&game).parse_action("九九平一").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_zero_numerals() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let resolver = NotationResolver::new(&game);
    let err = resolver.parse_action("红将零一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    let err = resolver.parse_action("红将平零").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    let err = resolver.parse_action("零一平二").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    let err = resolver.parse_action("红车零一拉").expect_err("must fail");
    assert!(err.contains("not on board"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_advance_past_board_edge() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[红车 一一 一一 一一 黑将]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红车进一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_truncated_position() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红将平").expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_trailing_action_text() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(&game).parse_action("红将平五推吃").expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn notation_rejects_empty_coordinate_origin() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(&game).parse_action("二二平三").expect_err("must fail");
    assert!(err.contains("no piece at (1,1)"), "unexpected error: {err}");
}

#[test]
fn state_parser_rejects_uneven_placement_pools() {
    let state = "行棋方：红
红方：[车 卒]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("uneven placement pools must fail");
    };
    assert!(err.contains("cannot alternate"), "unexpected error: {err}");
}

#[test]
fn state_parser_rejects_malformed_board_header() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘x：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 红将]
";
    let Err(err) = Game::from_str(state) else {
        panic!("malformed board header must fail");
    };
    assert!(err.contains("invalid board line"), "unexpected error: {err}");
}

#[test]
fn state_parser_rejects_wrong_column_header() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 六六]
一[黑将 一一 一一 一一 红将]
";
    let Err(err) = Game::from_str(state) else {
        panic!("wrong column header must fail");
    };
    assert!(err.contains("column header"), "unexpected error: {err}");
}

#[test]
fn state_parser_rejects_ragged_board_row() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一]
三[红将 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("ragged board row must fail");
    };
    assert!(err.contains("cells"), "unexpected error: {err}");
}

#[test]
fn state_parser_rejects_wrong_row_label() {
    let state = "行棋方：红
红方：[]
黑方：[]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
三[红将 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("wrong row label must fail");
    };
    assert!(err.contains("label"), "unexpected error: {err}");
}

#[test]
fn notation_formats_action_intent() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Move(Move { from: (0, 4), to: (0, 3) });
    assert_eq!(NotationResolver::new(&game).fmt_action(&action), "红将进一");
    let action = Action::Push(Move { from: (0, 4), to: (0, 2) });
    assert_eq!(NotationResolver::new(&game).fmt_action(&action), "红将进二推");
}

#[test]
fn notation_parses_coordinate_action_with_vertical_direction() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action =
        NotationResolver::new(&game).parse_action("三三直二").expect("coordinate vertical action");
    assert_eq!(action, Action::Move(Move { from: (2, 2), to: (2, 1) }));
}

#[test]
fn notation_formats_missing_origin_with_absolute_position() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Move(Move { from: (2, 3), to: (2, 1) });
    assert_eq!(NotationResolver::new(&game).fmt_action(&action), "三四三二");
}

#[test]
fn notation_formats_and_parses_pull_action() {
    let game = Game::from_str(SWAP_STATE).expect("parse");
    let action = Action::Pull(Move { from: (0, 1), to: (0, 2) });
    let notation = NotationResolver::new(&game).fmt_action(&action);
    assert_eq!(notation, "红车退一拉");
    assert_eq!(NotationResolver::new(&game).parse_action(&notation), Ok(action));
}

#[test]
fn notation_resolves_pass_and_target_resign() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let resolver = NotationResolver::new(&game);
    assert_eq!(resolver.parse_action("红将按兵"), Ok(Action::Pass(Player::Red)));
    assert_eq!(resolver.parse_action("红将认负"), Ok(Action::Resign(0, 4)));
}
