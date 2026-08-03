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
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;

mod support;
use support::api::SIMPLE;
use support::api::game_one;

#[test]
fn placement_action_uses_the_pool_piece_configuration() {
    let custom_rook = Piece {
        formation: Piece::RED_PAWN.formation,
        ability: Piece::RED_PAWN.ability,
        ..Piece::RED_ROOK
    };
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut game = Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![custom_rook],
        black_pool: vec![Piece::BLACK_PAWN],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid custom game");

    let reaction = game
        .action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (1, 3) }))
        .expect("place");
    assert_eq!(reaction.changes, vec![PositionChange { at: (1, 3), piece: Some(custom_rook) }]);
    let placed = game.board()[(1, 3)].expect("piece placed");
    assert_eq!(placed.formation.points, custom_rook.formation.points);
    assert_eq!(placed.ability, custom_rook.ability);
    assert_eq!(game.result(), GameResult::Unfinished);
}

#[test]
fn piece_id_round_trips_the_canonical_piece_identity() {
    let id = Piece::RED_ROOK.id();
    assert_eq!(id.to_string(), "红车");
    assert_eq!("红车".parse::<PieceId>(), Ok(id));
    assert_eq!(Piece::lookup(id.name, id.color), Some(Piece::RED_ROOK));
}

#[test]
fn try_move_changes_keep_the_original_piece_configuration() {
    let mut board = Board::new(3, 4);
    board[(0, 1)] = Some(Piece::RED_ROOK);
    board[(1, 1)] = Some(Piece::BLACK_ROOK);

    let effective = board.effective((1, 1)).expect("moving piece");
    assert!(!effective.ability.has(Ability::ANY_DISTANCE));

    let changes = board.try_move((1, 1), (1, 2)).expect("one-step move should succeed");
    assert_eq!(changes, vec![PositionChange { at: (1, 1), piece: None }, PositionChange {
        at: (1, 2),
        piece: Some(Piece::BLACK_ROOK)
    },]);
    board.apply(&changes);
    assert_eq!(board[(1, 2)].expect("moved piece").ability, Piece::BLACK_ROOK.ability);
}

#[test]
fn try_capture_changes_keep_the_original_mover_configuration() {
    let mut board = Board::new(5, 5);
    board[(0, 2)] = Some(Piece::RED_SPEAR);
    board[(1, 1)] = Some(Piece::RED_WIND);
    board[(0, 0)] = Some(Piece::WHITE);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let effective = board.effective((1, 1)).expect("moving piece");
    assert!(effective.ability.has(Ability::CAPTURE));
    assert!(!Piece::RED_WIND.ability.has(Ability::CAPTURE));

    let changes = board.try_capture((1, 1), (0, 0)).expect("capture should succeed");
    assert_eq!(changes, vec![PositionChange { at: (1, 1), piece: None }, PositionChange {
        at: (0, 0),
        piece: Some(Piece::RED_WIND)
    },]);
    board.apply(&changes);
    assert_eq!(board[(0, 0)].expect("capturing piece").ability, Piece::RED_WIND.ability);
}

#[test]
fn try_push_changes_keep_the_original_piece_configurations() {
    let mut board = Board::new(5, 5);
    board[(1, 3)] = Some(Piece::RED_SPEAR);
    board[(2, 2)] = Some(Piece::RED_WIND);
    board[(3, 3)] = Some(Piece::BLACK_SCHOLAR);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let effective = board.effective((2, 2)).expect("moving piece");
    assert!(effective.ability.has(Ability::CAPTURE));
    assert!(!Piece::RED_WIND.ability.has(Ability::CAPTURE));

    let changes = board.try_push((2, 2), (3, 3)).expect("push should succeed");
    assert_eq!(changes, vec![
        PositionChange { at: (2, 2), piece: None },
        PositionChange { at: (3, 3), piece: Some(Piece::RED_WIND) },
        PositionChange { at: (4, 4), piece: Some(Piece::BLACK_SCHOLAR) },
    ]);
    board.apply(&changes);
    assert_eq!(board[(3, 3)].expect("pushing piece").ability, Piece::RED_WIND.ability);
    assert_eq!(board[(4, 4)].expect("pushed piece").ability, Piece::BLACK_SCHOLAR.ability);
}

#[test]
fn capture_action_uses_mutual_destruction_bypass() {
    let mut board = Board::new(5, 5);
    board[(2, 2)] = Some(Piece::RED_ARMY);
    board[(2, 3)] = Some(Piece::BLACK_MINE);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let changes = board.try_capture((2, 2), (2, 3)).expect("retaliating target can be captured");
    assert_eq!(changes, vec![PositionChange { at: (2, 2), piece: None }, PositionChange {
        at: (2, 3),
        piece: None
    },]);
}
#[test]
fn action_rejects_same_origin_and_destination() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Capture(Move { from: (0, 4), to: (0, 4) });
    let err = game.action(action).expect_err("from == to must fail");
    assert!(err.contains("cannot move"), "unexpected error: {err}");
}

#[test]
fn action_rejects_out_of_bounds_coordinates() {
    let mut game = Game::from_str(SIMPLE).expect("parse");
    let action = Action::Push(Move { from: (0, 4), to: (0, 14) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("outside the board"), "unexpected error: {err}");
    let action = Action::Push(Move { from: (9, 9), to: (0, 0) });
    let err = game.action(action).expect_err("must fail");
    assert!(err.contains("no piece"), "unexpected error: {err}");
}

#[test]
fn placement_rejects_neutral_piece() {
    // White pieces cannot be placed via the Place action.
    // They only appear through captures or DIVIDE actions.
    let state = "行棋方：红
红方：[]
黑方：[]
白方：1
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[一一 一一 红军 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";
    let mut game = Game::from_str(state).expect("parse");
    let err = game
        .action(Action::Place(Place { piece: Piece::WHITE.id(), to: (1, 2) }))
        .expect_err("white placement must fail");
    assert!(err.contains("cannot place piece of color"), "unexpected error: {err}");
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
        black_pool: vec![Piece::BLACK_PAWN],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid");
    let reaction = g
        .try_action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 3) }))
        .expect("valid");
    assert_eq!(reaction.changes, vec![PositionChange { at: (0, 3), piece: Some(Piece::RED_ROOK) }]);
    assert!(!g.red_pool().is_empty(), "pool unchanged");
    assert_eq!(g.player(), Player::Red);
    g.action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 3) })).expect("real");
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
