use std::str::FromStr;

use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::action::PoolChange;
use formation_chess_core::action::PositionChange;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;
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
        red_pool: vec![Piece::RED_PAWN, custom_rook],
        black_pool: vec![Piece::BLACK_PAWN, Piece::BLACK_ROOK],
        result: GameResult::Unfinished,
    })
    .expect("valid custom game");
    let before = game.clone();

    let reaction = game
        .action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (1, 3) }))
        .expect("place");
    assert_eq!(reaction.changes.as_slice(), &[PositionChange {
        at: (1, 3),
        old: None,
        new: Some(custom_rook)
    }]);
    assert_eq!(reaction.pool_change, PoolChange::Removed { index: 1, piece: custom_rook });
    let placed = game.board()[(1, 3)].expect("piece placed");
    assert_eq!(placed.formation.points, custom_rook.formation.points);
    assert_eq!(placed.ability, custom_rook.ability);
    assert_eq!(game.result(), GameResult::Unfinished);

    let notation = NotationResolver::new(&before).fmt_reaction(Ok(reaction.clone()));
    let reparsed = NotationResolver::new(&before)
        .parse_reaction(&notation)
        .expect("parse reaction notation")
        .expect("successful reaction");
    assert_eq!(reparsed, reaction);

    game.undo(reaction);
    assert_same_game(&game, &before);
}

#[test]
fn piece_id_round_trips_the_canonical_piece_identity() {
    let id = Piece::RED_ROOK.id();
    assert_eq!(id.to_string(), "红车");
    assert_eq!("红车".parse::<PieceId>(), Ok(id));
    assert_eq!(Piece::lookup(id.name, id.player), Some(Piece::RED_ROOK));
}

#[test]
fn try_move_changes_keep_the_original_piece_configuration() {
    let mut board = Board::new(3, 4);
    board[(0, 1)] = Some(Piece::RED_ROOK);
    board[(1, 1)] = Some(Piece::BLACK_ROOK);

    let effective = board.effective((1, 1)).expect("moving piece");
    assert!(!effective.ability.has(Ability::ANY_DISTANCE));

    let changes = board.try_move((1, 1), (1, 2)).expect("one-step move should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (1, 1), old: Some(Piece::BLACK_ROOK), new: None },
        PositionChange { at: (1, 2), old: None, new: Some(Piece::BLACK_ROOK) },
    ]);
    board.apply(changes.as_slice());
    assert_eq!(board[(1, 2)].expect("moved piece").ability, Piece::BLACK_ROOK.ability);
}

#[test]
fn try_capture_changes_keep_the_original_mover_configuration() {
    let mut board = Board::new(5, 5);
    board[(2, 3)] = Some(Piece::RED_SPEAR);
    board[(2, 2)] = Some(Piece::RED_WIND);
    board[(2, 1)] = Some(Piece::BLACK_PAWN);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let effective = board.effective((2, 2)).expect("moving piece");
    assert!(effective.ability.has(Ability::CAPTURE));
    assert!(!Piece::RED_WIND.ability.has(Ability::CAPTURE));

    let changes = board.try_capture((2, 2), (2, 1)).expect("capture should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (2, 2), old: Some(Piece::RED_WIND), new: None },
        PositionChange { at: (2, 1), old: Some(Piece::BLACK_PAWN), new: Some(Piece::RED_WIND) },
    ]);
    board.apply(changes.as_slice());
    assert_eq!(board[(2, 1)].expect("capturing piece").ability, Piece::RED_WIND.ability);
}

#[test]
fn try_push_changes_keep_the_original_piece_configurations() {
    let mut board = Board::new(5, 5);
    board[(1, 3)] = Some(Piece::RED_SCHOLAR);
    board[(2, 2)] = Some(Piece::RED_WIND);
    board[(3, 3)] = Some(Piece::BLACK_PAWN);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let effective = board.effective((2, 2)).expect("moving piece");
    assert!(effective.ability.has(Ability::DIRECTION_DIAGONAL));
    assert!(!Piece::RED_WIND.ability.has(Ability::DIRECTION_DIAGONAL));

    let changes = board.try_push((2, 2), (3, 3)).expect("push should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (2, 2), old: Some(Piece::RED_WIND), new: None },
        PositionChange { at: (3, 3), old: Some(Piece::BLACK_PAWN), new: Some(Piece::RED_WIND) },
        PositionChange { at: (4, 4), old: None, new: Some(Piece::BLACK_PAWN) },
    ]);
    board.apply(changes.as_slice());
    assert_eq!(board[(3, 3)].expect("pushing piece").ability, Piece::RED_WIND.ability);
    assert_eq!(board[(4, 4)].expect("pushed piece").ability, Piece::BLACK_PAWN.ability);
}

#[test]
fn try_push_omits_an_unchanged_middle_point() {
    let mut board = Board::new(5, 5);
    board[(1, 2)] = Some(Piece::RED_WIND);
    board[(2, 2)] = Some(Piece::RED_WIND);

    let changes = board.try_push((1, 2), (2, 2)).expect("ally push should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (1, 2), old: Some(Piece::RED_WIND), new: None },
        PositionChange { at: (3, 2), old: None, new: Some(Piece::RED_WIND) },
    ]);
    for change in changes.as_slice() {
        assert_ne!(change.old, change.new);
    }

    board.apply(changes.as_slice());
    assert_eq!(board[(2, 2)], Some(Piece::RED_WIND));
    assert_eq!(board[(3, 2)], Some(Piece::RED_WIND));
}

#[test]
fn blocked_push_capture_omits_an_unchanged_destination() {
    let piece = Piece {
        ability: Piece::RED_WIND.ability | Ability::CAPTURE_ON_PUSH_BLOCKED,
        ..Piece::RED_WIND
    };
    let mut board = Board::new(5, 5);
    board[(1, 2)] = Some(piece);
    board[(2, 2)] = Some(piece);
    board[(3, 2)] = Some(Piece::BLACK_PAWN);

    let changes = board.try_push((1, 2), (2, 2)).expect("blocked push should capture");
    assert_eq!(changes.as_slice(), &[PositionChange { at: (1, 2), old: Some(piece), new: None }]);

    board.apply(changes.as_slice());
    assert_eq!(board[(2, 2)], Some(piece));
    assert_eq!(board[(3, 2)], Some(Piece::BLACK_PAWN));
}

#[test]
fn try_draw_exchanges_both_piece_configurations() {
    let red_vital = Piece::RED_GENERAL;
    let black_vital = Piece { formation: Piece::RED_WIND.formation, ..Piece::BLACK_GENERAL };
    let mut board = Board::new(3, 3);
    board[(1, 1)] = Some(red_vital);
    board[(2, 2)] = Some(black_vital);

    let changes = board.try_draw((1, 1), (2, 2)).expect("draw should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (1, 1), old: Some(red_vital), new: Some(black_vital) },
        PositionChange { at: (2, 2), old: Some(black_vital), new: Some(red_vital) },
    ]);

    board.apply(changes.as_slice());
    assert_eq!(board[(1, 1)], Some(black_vital));
    assert_eq!(board[(2, 2)], Some(red_vital));
}
#[test]
fn try_pull_changes_keep_original_piece_configurations() {
    let mut board = Board::new(5, 5);
    board[(1, 2)] = Some(Piece::RED_FIRE);
    board[(2, 2)] = Some(Piece::RED_PAWN);
    board[(2, 3)] = Some(Piece::BLACK_SPEAR);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let effective = board.effective((2, 2)).expect("pulling piece");
    assert!(effective.ability.has(Ability::PULL_ENEMY));
    assert!(!Piece::RED_PAWN.ability.has(Ability::PULL_ENEMY));

    let changes = board.try_pull((2, 2), (2, 1)).expect("pull should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (2, 2), old: Some(Piece::RED_PAWN), new: Some(Piece::BLACK_SPEAR) },
        PositionChange { at: (2, 1), old: None, new: Some(Piece::RED_PAWN) },
        PositionChange { at: (2, 3), old: Some(Piece::BLACK_SPEAR), new: None },
    ]);
    board.apply(changes.as_slice());
    assert_eq!(board[(2, 1)], Some(Piece::RED_PAWN));
    assert_eq!(board[(2, 2)], Some(Piece::BLACK_SPEAR));
    assert_eq!(board[(2, 3)], None);
}

#[test]
fn try_pull_omits_an_unchanged_origin() {
    let mut board = Board::new(3, 3);
    board[(1, 1)] = Some(Piece::RED_WIND);
    board[(1, 2)] = Some(Piece::RED_WIND);

    let changes = board.try_pull((1, 1), (1, 0)).expect("ally pull should succeed");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (1, 0), old: None, new: Some(Piece::RED_WIND) },
        PositionChange { at: (1, 2), old: Some(Piece::RED_WIND), new: None },
    ]);
    for change in changes.as_slice() {
        assert_ne!(change.old, change.new);
    }

    board.apply(changes.as_slice());
    assert_eq!(board[(1, 0)], Some(Piece::RED_WIND));
    assert_eq!(board[(1, 1)], Some(Piece::RED_WIND));
    assert_eq!(board[(1, 2)], None);
}

#[test]
fn capture_action_uses_mutual_destruction_bypass() {
    let mut board = Board::new(5, 5);
    board[(2, 2)] = Some(Piece::RED_ARMY);
    board[(3, 3)] = Some(Piece::BLACK_MINE);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let changes = board.try_capture((2, 2), (3, 3)).expect("retaliating target can be captured");
    assert_eq!(changes.as_slice(), &[
        PositionChange { at: (2, 2), old: Some(Piece::RED_ARMY), new: None },
        PositionChange { at: (3, 3), old: Some(Piece::BLACK_MINE), new: None },
    ]);
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
        result: GameResult::Unfinished,
    })
    .expect("valid");
    let reaction = g
        .try_action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 3) }))
        .expect("valid");
    assert_eq!(reaction.changes.as_slice(), &[PositionChange {
        at: (0, 3),
        old: None,
        new: Some(Piece::RED_ROOK)
    }]);
    assert_eq!(reaction.pool_change, PoolChange::Removed { index: 0, piece: Piece::RED_ROOK });
    assert!(!g.red_pool().is_empty(), "pool unchanged");
    assert_eq!(g.player(), Player::Red);
    g.action(Action::Place(Place { piece: Piece::RED_ROOK.id(), to: (0, 3) })).expect("real");
    assert!(g.red_pool().is_empty());
}

#[test]
fn try_action_rejected_on_decided_game() {
    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    g.action(Action::Resign(0, 4)).expect("resign");
    let err = g.try_action(Action::Move(Move { from: (2, 2), to: (2, 1) })).unwrap_err();
    assert!(err.contains("already decided"));
}

#[test]
fn try_action_resign_returns_result_without_setting_it() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let reaction = g.try_action(Action::Resign(0, 4)).expect("valid");
    assert_eq!(reaction.game_result, GameResult::BlackWin);
    assert!(reaction.changes.is_empty());
    assert_eq!(g.result(), GameResult::Unfinished);
    assert_eq!(g.player(), Player::Red);
}

#[test]
fn undo_restores_move_pass_and_resign() {
    let initial = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    for action in [
        Action::Move(Move { from: (2, 2), to: (2, 1) }),
        Action::Pass(Player::Red),
        Action::Resign(0, 4),
    ] {
        let mut game = initial.clone();
        let reaction = game.action(action).expect("action");
        game.undo(reaction);
        assert_same_game(&game, &initial);
    }
}

#[test]
fn undo_restores_captured_custom_piece() {
    let custom_target = Piece { formation: Piece::BLACK_ROOK.formation, ..Piece::BLACK_PAWN };
    let mut board = Board::new(5, 5);
    board[(1, 2)] = Some(Piece::RED_ROOK);
    board[(2, 2)] = Some(custom_target);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut game = movement_game(board);
    let before = game.clone();

    let reaction =
        game.action(Action::Capture(Move { from: (1, 2), to: (2, 2) })).expect("capture");
    assert_eq!(reaction.changes.as_slice()[1].old, Some(custom_target));

    let notation = NotationResolver::new(&before).fmt_reaction(Ok(reaction.clone()));
    let reparsed = NotationResolver::new(&before)
        .parse_reaction(&notation)
        .expect("parse reaction notation")
        .expect("successful reaction");
    assert_eq!(reparsed, reaction);

    game.undo(reaction);
    assert_same_game(&game, &before);
}

#[test]
fn undo_restores_mutual_destruction_and_push() {
    let mut capture_board = Board::new(5, 5);
    capture_board[(2, 2)] = Some(Piece::RED_ARMY);
    capture_board[(3, 3)] = Some(Piece::BLACK_MINE);
    capture_board[(0, 4)] = Some(Piece::RED_GENERAL);
    capture_board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut capture_game = movement_game(capture_board);
    let capture_before = capture_game.clone();
    let reaction = capture_game
        .action(Action::Capture(Move { from: (2, 2), to: (3, 3) }))
        .expect("mutual destruction");
    capture_game.undo(reaction);
    assert_same_game(&capture_game, &capture_before);

    let mut push_board = Board::new(5, 5);
    push_board[(2, 2)] = Some(Piece::RED_WIND);
    push_board[(3, 2)] = Some(Piece::BLACK_SHIELD);
    push_board[(0, 4)] = Some(Piece::RED_GENERAL);
    push_board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut push_game = movement_game(push_board);
    let push_before = push_game.clone();
    let reaction = push_game.action(Action::Push(Move { from: (2, 2), to: (3, 2) })).expect("push");
    push_game.undo(reaction);
    assert_same_game(&push_game, &push_before);
}

#[test]
fn undo_restores_draw_exchange_and_pull() {
    let mut draw_board = Board::new(5, 5);
    draw_board[(1, 2)] = Some(Piece::RED_GENERAL);
    draw_board[(2, 1)] =
        Some(Piece { formation: Piece::RED_WIND.formation, ..Piece::BLACK_GENERAL });
    let mut draw_game = movement_game(draw_board);
    let draw_before = draw_game.clone();
    let reaction = draw_game.action(Action::Draw(Move { from: (1, 2), to: (2, 1) })).expect("draw");
    assert_eq!(draw_game.result(), GameResult::Draw);
    assert_eq!(draw_game.board()[(1, 2)], Some(Piece::BLACK_GENERAL));
    assert_eq!(draw_game.board()[(2, 1)], Some(Piece::RED_GENERAL));
    draw_game.undo(reaction);
    assert_same_game(&draw_game, &draw_before);

    let mut pull_board = Board::new(5, 5);
    pull_board[(1, 1)] = Some(Piece::RED_WIND);
    pull_board[(1, 2)] = Some(Piece::RED_PAWN);
    pull_board[(0, 4)] = Some(Piece::RED_GENERAL);
    pull_board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut pull_game = movement_game(pull_board);
    let pull_before = pull_game.clone();
    let reaction = pull_game.action(Action::Pull(Move { from: (1, 1), to: (1, 0) })).expect("pull");
    let notation = NotationResolver::new(&pull_before).fmt_reaction(Ok(reaction.clone()));
    let reparsed = NotationResolver::new(&pull_before)
        .parse_reaction(&notation)
        .expect("parse reaction notation")
        .expect("successful reaction");
    assert_eq!(
        Board::normalize_changes(reparsed.changes.as_slice()),
        Board::normalize_changes(reaction.changes.as_slice())
    );
    assert_eq!(reparsed.pool_change, reaction.pool_change);
    assert_eq!(reparsed.game_result, reaction.game_result);
    pull_game.undo(reaction);
    assert_same_game(&pull_game, &pull_before);
}

#[test]
fn undo_restores_placement_phase_transition() {
    let custom_rook = Piece { ability: Piece::BLACK_PAWN.ability, ..Piece::BLACK_ROOK };
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let mut game = Game::new(GameConfig {
        player: Player::Black,
        board,
        red_pool: Vec::new(),
        black_pool: vec![custom_rook],
        result: GameResult::Unfinished,
    })
    .expect("valid final placement state");
    let before = game.clone();

    let reaction = game
        .action(Action::Place(Place { piece: custom_rook.id(), to: (1, 1) }))
        .expect("final placement");
    assert_eq!(game.phase(), Phase::Move);
    game.undo(reaction);
    assert_same_game(&game, &before);
    assert_eq!(game.phase(), Phase::Place);
}

fn movement_game(board: Board) -> Game {
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

fn assert_same_game(actual: &Game, expected: &Game) {
    assert_eq!(actual.player(), expected.player(), "player must be restored");
    assert_eq!(actual.red_pool(), expected.red_pool(), "red pool must be restored");
    assert_eq!(actual.black_pool(), expected.black_pool(), "black pool must be restored");
    assert_eq!(actual.result(), expected.result(), "result must be restored");
    assert_eq!(actual.phase(), expected.phase(), "phase must be restored");
    assert_eq!(actual.board().width(), expected.board().width(), "board width must be restored");
    assert_eq!(actual.board().height(), expected.board().height(), "board height must be restored");
    assert_eq!(
        actual.board().iter().collect::<Vec<_>>(),
        expected.board().iter().collect::<Vec<_>>(),
        "board pieces must be restored"
    );
}
