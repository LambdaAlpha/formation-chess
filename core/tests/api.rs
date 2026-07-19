use std::str::FromStr;

use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::PieceChange;
use formation_chess_core::action::Place;
use formation_chess_core::formation::Formation;
use formation_chess_core::game::Game;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;

const SIMPLE: &str = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[红卒 一一 红卒 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";

#[test]
fn ambiguous_piece_name_is_a_parse_error() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(game.board())
        .parse_action("红卒进一")
        .expect_err("must be ambiguous");
    assert!(err.contains("multiple 红卒"), "unexpected error: {err}");
}

#[test]
fn coordinates_disambiguate_same_name_pieces() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action =
        NotationResolver::new(game.board()).parse_action("一三直二").expect("parse action");
    game.action(action).expect("action");
    assert_eq!(game.board()[(0, 1)].map(|p| p.name), Some('卒'));
}

#[test]
fn invalid_config_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[将]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
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
fn explicit_placement_suffix_parses_and_formats() {
    let state = "行棋方：红
红方：[车]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    let action =
        NotationResolver::new(game.board()).parse_action("红车三四占").expect("parse action");
    let Action::Place(place) = action else {
        panic!("expected placement, got {action}");
    };
    assert_eq!(place.to, (2, 3));
    assert_eq!(NotationResolver::new(game.board()).fmt_action(&action), "红车三四占");

    // implicit placement (no suffix, piece not on board) still works
    let implicit =
        NotationResolver::new(game.board()).parse_action("红车三四").expect("parse action");
    let Action::Place(place) = implicit else {
        panic!("expected placement, got {implicit}");
    };
    assert_eq!(place.to, (2, 3));
}

#[test]
fn placement_with_relative_position_is_rejected() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err =
        NotationResolver::new(game.board()).parse_action("红车进一占").expect_err("must fail");
    assert!(err.contains("absolute position"), "unexpected error: {err}");
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
        "行棋方：红\n红方：[]\n黑方：[]\n白方：0\n胜负：未分\n棋盘：\n零[一一 二二 三三 四四 五五]\n{rows}"
    );
    let game = Game::from_str(&state).expect("parse");
    let action = Action::Push(Move { from: (0, 14), to: (0, 0) });
    let formatted = NotationResolver::new(game.board()).fmt_action(&action);
    assert_eq!(formatted, "红车进丁推");
    let reparsed =
        NotationResolver::new(game.board()).parse_action(&formatted).expect("parse action");
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
零[一一 二二 三三 四四 五五]
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
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("unfinished result on a decided position must fail");
    };
    assert!(err.contains("already decided"), "unexpected error: {err}");
}

#[test]
fn unfinished_but_already_drawn_is_error() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[一一 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 红将 一一 一一]
四[一一 一一 一一 黑将 一一]
五[一一 一一 一一 一一 一一]
";
    let Err(err) = Game::from_str(state) else {
        panic!("unfinished result on a decided position must fail");
    };
    assert!(err.contains("already decided"), "unexpected error: {err}");
}

#[test]
fn out_of_bounds_placement_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[车]
黑方：[卒]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    let err =
        NotationResolver::new(game.board()).parse_action("红车九九占").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn out_of_bounds_move_is_an_error_not_a_panic() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    // destination outside the board
    let action = Action::Push(Move { from: (0, 4), to: (0, 14) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    // origin outside the board
    let action = Action::Push(Move { from: (9, 9), to: (0, 0) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("no piece"), "unexpected error: {err}");
    // out-of-bounds coordinates in text notation
    let err = NotationResolver::new(game.board()).parse_action("九九平一").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn numeral_zero_is_an_error_not_a_panic() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let resolver = NotationResolver::new(game.board());
    // 零 (0) in an absolute position
    let err = resolver.parse_action("红将零一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    // 零 in a horizontal move
    let err = resolver.parse_action("红将平零").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
    // 零 in a coordinate piece reference
    let err = resolver.parse_action("零一平二").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    // 零 in a placement
    let err = resolver.parse_action("红车零一占").expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
}

#[test]
fn advance_past_the_edge_is_an_error_not_a_panic() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[红车 一一 一一 一一 黑将]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let game = Game::from_str(state).expect("parse");
    // the red rook already stands on the top row
    let err = NotationResolver::new(game.board()).parse_action("红车进一").expect_err("must fail");
    assert!(err.contains("cannot resolve position"), "unexpected error: {err}");
}

#[test]
fn truncated_position_is_an_error_not_a_panic() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(game.board()).parse_action("红将平").expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn trailing_garbage_in_action_is_rejected() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err =
        NotationResolver::new(game.board()).parse_action("红将平五推吃").expect_err("must fail");
    assert!(err.contains("invalid position"), "unexpected error: {err}");
}

#[test]
fn coordinate_move_from_empty_point_is_rejected() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let err = NotationResolver::new(game.board()).parse_action("二二平三").expect_err("must fail");
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
零[一一 二二 三三 四四 五五]
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
零[一一 二二 三三 四四 五五]
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
零[一一 二二 三三 四四 六六]
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
零[一一 二二 三三 四四 五五]
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
零[一一 二二 三三 四四 五五]
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
    // (0,4)→(0,3) is empty: no suffix
    let action = Action::Move(Move { from: (0, 4), to: (0, 3) });
    assert_eq!(NotationResolver::new(game.board()).fmt_action(&action), "红将进一");
    // (0,4)→(0,2) holds a red pawn: suffix kept
    let action = Action::Push(Move { from: (0, 4), to: (0, 2) });
    assert_eq!(NotationResolver::new(game.board()).fmt_action(&action), "红将进二推");
}

#[test]
fn action_move_from_oob_origin_uses_vertical_notation() {
    let game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Move(Move { from: (2, 3), to: (2, 1) });
    assert_eq!(NotationResolver::new(game.board()).fmt_action(&action), "三四直二");
}

#[test]
fn mismatched_white_piece_is_rejected() {
    let state = "行棋方：红
红方：[]
黑方：[]
白方：1
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 红巫 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let mut game = Game::from_str(state).expect("parse");
    let rogue = Piece { name: '皇', ..Piece::WHITE };
    let err = game
        .action(Action::Place(Place { piece: rogue, to: (2, 3) }))
        .expect_err("mismatched white piece must fail");
    assert!(err.contains("does not match"), "unexpected error: {err}");
}

#[test]
fn placement_uses_the_canonical_pool_piece() {
    let state = "行棋方：红
红方：[将]
黑方：[卒]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 一一]
六[一一 一一 一一 一一 一一]
";
    let mut game = Game::from_str(state).expect("parse");
    // Same name and color as the pool general (so equality matches), but
    // with forged abilities: VITAL stripped, rook powers instead.
    let forged = Piece { ability: Piece::RED_ROOK.ability, ..Piece::RED_GENERAL };
    game.action(Action::Place(Place { piece: forged, to: (0, 5) })).expect("place");
    let placed = game.board()[(0, 5)].expect("piece placed");
    assert!(
        placed.ability.has_ability(Ability::VITAL),
        "the canonical pool piece must be placed, not the caller-supplied one"
    );
    assert_eq!(game.result(), GameResult::Unfinished);
}

const SWAP_STATE: &str = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一一 二二 三三 四四 五五]
一[红将 一一 一一 一一 一一]
二[红车 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 红马 一一 一一 黑将]
五[一一 一一 一一 一一 一一]
";

#[test]
fn swap_changes_resolve_and_apply() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[红车二四 红马一二]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(result.changes, vec![
        PieceChange::Move(Move { from: (0, 1), to: (1, 3) }),
        PieceChange::Move(Move { from: (1, 3), to: (0, 1) }),
    ]);

    let changes = board.resolve_changes(&result.changes).expect("resolve changes");
    let mut applied = board.clone();
    applied.apply(&changes);

    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(1, 3)].map(|p| p.name), Some('车'));
}

#[test]
fn swap_changes_are_order_independent() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let a = NotationResolver::new(board)
        .parse_reaction("变化：[红车二四 红马一二]\n胜负：未分")
        .expect("parse result")
        .expect("success result");
    let b = NotationResolver::new(board)
        .parse_reaction("变化：[红马一二 红车二四]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    let a_changes = board.resolve_changes(&a.changes).expect("resolve changes");
    let b_changes = board.resolve_changes(&b.changes).expect("resolve changes");
    assert_eq!(a_changes, b_changes);
}

#[test]
fn coordinate_identified_swap_with_relative_position() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[二四一二 一二一四]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(result.changes, vec![
        PieceChange::Move(Move { from: (1, 3), to: (0, 1) }),
        PieceChange::Move(Move { from: (0, 1), to: (0, 3) }),
    ]);
}

#[test]
fn removal_entry_clears_point() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[红车提]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(result.changes, vec![PieceChange::Remove(0, 1)]);

    let changes = board.resolve_changes(&result.changes).expect("resolve changes");
    let mut applied = board.clone();
    applied.apply(&changes);
    assert_eq!(applied[(0, 1)], None);
}
