use std::str::FromStr;

use formation_chess_core::action::Action;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::formation::Formation;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;

mod api_common;
use api_common::SIMPLE;

#[test]
fn ambiguous_piece_name_is_a_parse_error() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::movement(game.board())
        .parse_action("红卒进一")
        .expect_err("must be ambiguous");
    assert!(err.contains("multiple 红卒"), "unexpected error: {err}");
}

#[test]
fn coordinates_disambiguate_same_name_pieces() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action =
        NotationResolver::movement(game.board()).parse_action("一三直二").expect("parse action");
    game.action(action).expect("action");
    assert_eq!(game.board()[(0, 1)].map(|p| p.name), Some('卒'));
}

#[test]
fn invalid_config_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[将]
黑方：[卒]
白方：0
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
fn moving_to_own_point_is_rejected() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Capture(Move { from: (0, 4), to: (0, 4) });
    let err = game.action(action).expect_err("from == to must fail");
    assert!(err.contains("cannot move"), "unexpected error: {err}");
}

#[test]
fn placement_with_relative_position_is_rejected() {
    // During placement phase, only name + absolute position is a valid
    // placement. Name + relative position (e.g. 红车进一) is rejected.
    let state = "行棋方：红
红方：[车]
黑方：[将]
白方：0
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
    let err = NotationResolver::new(game.board(), game.phase())
        .parse_action("红车进一")
        .expect_err("must fail");
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
        "行棋方：红\n红方：[]\n黑方：[]\n白方：0\n胜负：未分\n棋盘：\n零[一路 二路 三路 四路 五路]\n{rows}"
    );
    let game = Game::from_str(&state).expect("parse");
    let action = Action::Push(Move { from: (0, 14), to: (0, 0) });
    let formatted = NotationResolver::movement(game.board()).fmt_action(&action);
    assert_eq!(formatted, "红车进丁推");
    let reparsed =
        NotationResolver::movement(game.board()).parse_action(&formatted).expect("parse action");
    let Action::Push(move_) = reparsed else {
        panic!("expected move, got {reparsed}");
    };
    assert_eq!((move_.from, move_.to), ((0, 14), (0, 0)));
}

#[test]
fn formation_contains_out_of_range_returns_false() {
    assert!(!Formation::GENERAL.contains(0, 0));
    assert!(!Formation::ROOK.contains(0, 0));
    assert!(!Formation::GENERAL.contains(2, 0));
    assert!(!Formation::ROOK.contains(-2, 2));
}

#[test]
fn middle_point_constants_match_contains() {
    let left = Formation { points: Formation::MIDDLE_LEFT, effect: Formation::general };
    assert!(left.contains(-1, 0));
    assert!(!left.contains(1, 0));
    let right = Formation { points: Formation::MIDDLE_RIGHT, effect: Formation::general };
    assert!(right.contains(1, 0));
    assert!(!right.contains(-1, 0));
}

#[test]
fn decided_game_displays_result() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn one_side_no_vital_must_declare_result() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn unfinished_with_generals_in_formation_is_valid() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn out_of_bounds_placement_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[车]
黑方：[卒]
白方：0
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
    let err =
        NotationResolver::movement(game.board()).parse_action("红车九九留").expect_err("must fail");
    assert!(err.contains("not on board"), "unexpected error: {err}");
}

#[test]
fn out_of_bounds_move_is_an_error_not_a_panic() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Push(Move { from: (0, 4), to: (0, 14) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    let action = Action::Push(Move { from: (9, 9), to: (0, 0) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("no piece"), "unexpected error: {err}");
    let err =
        NotationResolver::movement(game.board()).parse_action("九九平一").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn numeral_zero_is_an_error_not_a_panic() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let resolver = NotationResolver::movement(game.board());
    let err = resolver.parse_action("红将零一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    let err = resolver.parse_action("红将平零").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    let err = resolver.parse_action("零一平二").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    let err = resolver.parse_action("红车零一留").expect_err("must fail");
    assert!(err.contains("not on board"), "unexpected error: {err}");
}

#[test]
fn advance_past_the_edge_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
    let err =
        NotationResolver::movement(game.board()).parse_action("红车进一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
}

#[test]
fn truncated_position_is_an_error_not_a_panic() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err =
        NotationResolver::movement(game.board()).parse_action("红将平").expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn trailing_garbage_in_action_is_rejected() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::movement(game.board())
        .parse_action("红将平五推吃")
        .expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn coordinate_move_from_empty_point_is_rejected() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err =
        NotationResolver::movement(game.board()).parse_action("二二平三").expect_err("must fail");
    assert!(err.contains("no piece at (1,1)"), "unexpected error: {err}");
}

#[test]
fn uneven_placement_pools_are_rejected() {
    let state = "行棋方：红
红方：[车 卒]
黑方：[]
白方：0
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
fn malformed_board_header_is_rejected() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn wrong_column_header_is_rejected() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn ragged_board_row_is_rejected() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn wrong_row_label_is_rejected() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
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
fn simple_move_formats_without_combat_suffix() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Move(Move { from: (0, 4), to: (0, 3) });
    assert_eq!(NotationResolver::movement(game.board()).fmt_action(&action), "红将进一");
    let action = Action::Push(Move { from: (0, 4), to: (0, 2) });
    assert_eq!(NotationResolver::movement(game.board()).fmt_action(&action), "红将进二推");
}

#[test]
fn action_move_from_oob_origin_uses_vertical_notation() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Move(Move { from: (2, 3), to: (2, 1) });
    assert_eq!(NotationResolver::movement(game.board()).fmt_action(&action), "三四直二");
}

#[test]
fn mismatched_white_piece_is_rejected() {
    // White pieces cannot be placed via the Place action.
    // They only appear through captures or Leave actions.
    let state = "行棋方：红
红方：[]
黑方：[]
白方：1
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 红巫 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let mut game = Game::from_str(state).expect("parse");
    let err = game
        .action(Action::Place(Place { piece: Piece::WHITE, to: (1, 2) }))
        .expect_err("white placement must fail");
    assert!(err.contains("cannot place piece of color"), "unexpected error: {err}");
}
