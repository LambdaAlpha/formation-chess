use std::str::FromStr;

use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::action::PositionChange;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

mod api_common;
use api_common::SWAP_STATE;
use api_common::game_one;

#[test]
fn placement_uses_the_canonical_pool_piece() {
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
五[一一 一一 一一 一一 一一]
六[一一 一一 一一 一一 一一]
";
    let mut game = Game::from_str(state).expect("parse");
    let forged = Piece { ability: Piece::RED_ROOK.ability, ..Piece::RED_GENERAL };
    game.action(Action::Place(Place { piece: forged, to: (0, 5) })).expect("place");
    let placed = game.board()[(0, 5)].expect("piece placed");
    assert!(placed.ability.has_ability(Ability::VITAL));
    assert_eq!(game.result(), GameResult::Unfinished);
}

// ── reaction: swap & removal ──────────────────────────────────────────────

#[test]
fn swap_changes_resolve_and_apply() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[红车二四 红马一二]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    let mut applied = board.clone();
    applied.apply(&result.changes);

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

    assert_eq!(a.changes, b.changes);
}

#[test]
fn coordinate_identified_swap_with_relative_position() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[二四一二 一二一四]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    let mut applied = board.clone();
    applied.apply(&result.changes);

    assert_eq!(applied[(0, 1)].map(|p| p.name), Some('马'));
    assert_eq!(applied[(0, 3)].map(|p| p.name), Some('车'));
    assert_eq!(applied[(1, 3)], None);
}

#[test]
fn removal_entry_clears_point() {
    let game = Game::from_str(SWAP_STATE).expect("parse game");
    let board = game.board();

    let result = NotationResolver::new(board)
        .parse_reaction("变化：[红车提]\n胜负：未分")
        .expect("parse result")
        .expect("success result");

    assert_eq!(result.changes, vec![PositionChange { at: (0, 1), piece: None }]);

    let mut applied = board.clone();
    applied.apply(&result.changes);
    assert_eq!(applied[(0, 1)], None);
}

// ── try_action ───────────────────────────────────────────────────────────

#[test]
fn try_action_move_validates_without_mutating() {
    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let reaction =
        g.try_action(Action::Move(Move { from: (2, 2), to: (2, 1) })).expect("valid move");
    assert_eq!(reaction.game_result, GameResult::Unfinished);
    assert_eq!(reaction.changes.len(), 2);
    assert!(g.board()[(2, 2)].is_some(), "piece still at origin");
    assert!(g.board()[(2, 1)].is_none(), "destination still empty");
    assert_eq!(g.player(), Player::Red);
    g.action(Action::Move(Move { from: (2, 2), to: (2, 1) })).expect("real move");
    assert!(g.board()[(2, 2)].is_none());
    assert!(g.board()[(2, 1)].is_some());
}

#[test]
fn try_action_place_validates_without_mutating() {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut g = Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid");
    let reaction =
        g.try_action(Action::Place(Place { piece: Piece::RED_ROOK, to: (0, 3) })).expect("valid");
    assert_eq!(reaction.changes, vec![PositionChange { at: (0, 3), piece: Some(Piece::RED_ROOK) }]);
    assert!(!g.red_pool().is_empty(), "pool unchanged");
    assert_eq!(g.player(), Player::Red);
    g.action(Action::Place(Place { piece: Piece::RED_ROOK, to: (0, 3) })).expect("real");
    assert!(g.red_pool().is_empty());
}

#[test]
fn try_action_rejected_on_decided_game() {
    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    g.action(Action::Resign(Player::Red)).expect("resign");
    let err = g.try_action(Action::Move(Move { from: (2, 2), to: (2, 1) })).unwrap_err();
    assert!(err.contains("already decided"));
}

#[test]
fn try_action_resign_returns_result_without_setting_it() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let reaction = g.try_action(Action::Resign(Player::Red)).expect("valid");
    assert_eq!(reaction.game_result, GameResult::BlackWin);
    assert!(reaction.changes.is_empty());
    assert_eq!(g.result(), GameResult::Unfinished);
    assert_eq!(g.player(), Player::Red);
}
