use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

mod api_common;
use api_common::assert_captures;
use api_common::assert_moves;
use api_common::assert_pushes;
use api_common::game_one;
use api_common::game_one_3x3;
use api_common::game_with;
use api_common::game_with_white_pool;

// ── valid_moves: edge cases ───────────────────────────────────────────────

#[test]
fn valid_moves_empty_cell_returns_empty() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    assert!(g.valid_moves(1, 1).is_empty());
}

#[test]
fn valid_moves_placement_phase_returns_empty() {
    let mut board = Board::new(9, 10);
    board[(0, 9)] = Some(Piece::RED_GENERAL);
    board[(8, 0)] = Some(Piece::BLACK_GENERAL);
    let g = Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid");
    assert!(g.valid_moves(0, 0).is_empty());
}

#[test]
fn valid_moves_decided_game_returns_empty() {
    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    g.action(Action::Resign(Player::Red)).expect("resign");
    assert!(g.valid_moves(0, 4).is_empty());
}

#[test]
fn valid_moves_wrong_player_returns_empty() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (0, 1)),
            (Piece::BLACK_PAWN, (1, 0)),
            (Piece::RED_GENERAL, (0, 2)),
            (Piece::BLACK_GENERAL, (2, 0)),
        ],
        3,
        3,
    );
    assert!(g.valid_moves(1, 0).is_empty());
}

#[test]
fn valid_moves_white_piece_returns_empty() {
    let g = game_with(
        Player::Red,
        &[(Piece::WHITE, (1, 1)), (Piece::RED_GENERAL, (0, 2)), (Piece::BLACK_GENERAL, (2, 0))],
        3,
        3,
    );
    assert!(g.valid_moves(1, 1).is_empty());
}

// ── rook: ANY_DISTANCE + DIRECTION_CROSS + CAPTURE ────────────────────────

#[test]
fn valid_moves_rook_open_board() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 0), (2, 1), (2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert!(actions.iter().all(|a| matches!(a, Action::Move(_))));
}

#[test]
fn valid_moves_rook_capture_enemy() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (2, 2)),
            (Piece::WHITE, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 1), (2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_captures(&actions, &[(2, 0)]);
}

#[test]
fn valid_moves_rook_blocked_by_ally() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (2, 2)),
            (Piece::RED_SHIELD, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 1), (2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert!(!actions.iter().any(|a| matches!(a, Action::Capture(_) | Action::Push(_))));
}

// ── pawn: DIRECTION_CROSS + CAPTURE, one step ─────────────────────────────

#[test]
fn valid_moves_pawn_one_step_all_directions() {
    let g = game_one(Player::Red, Piece::RED_PAWN, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 2), (2, 1), (2, 3), (3, 2)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_pawn_capture_and_move() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_PAWN, (2, 2)),
            (Piece::WHITE, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 2), (2, 3), (3, 2)]);
    assert_captures(&actions, &[(2, 1)]);
    assert_eq!(actions.len(), 4);
}

// ── dog: DIRECTION_DIAGONAL + CAPTURE, one step ───────────────────────────

#[test]
fn valid_moves_dog_diagonal_only() {
    let g = game_one(Player::Red, Piece::RED_DOG, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 1), (1, 3), (3, 1), (3, 3)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_dog_capture_diagonal() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_DOG, (2, 2)),
            (Piece::WHITE, (1, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 3), (3, 1), (3, 3)]);
    assert_captures(&actions, &[(1, 1)]);
    assert_eq!(actions.len(), 4);
}

// ── horse: DIRECTION_SHAPE_L + CAPTURE ────────────────────────────────────

#[test]
fn valid_moves_horse_all_eight_directions() {
    let g = game_one(Player::Red, Piece::RED_HORSE, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 1), (0, 3), (1, 0), (1, 4), (3, 0), (3, 4), (4, 1), (4, 3)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_horse_leg_blocked() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_HORSE, (2, 2)),
            (Piece::WHITE, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 1), (0, 3), (1, 4), (3, 4), (4, 1), (4, 3)]);
    assert_eq!(actions.len(), 6);
}

#[test]
fn valid_moves_horse_capture_at_adjacent_enemy() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_HORSE, (2, 2)),
            (Piece::WHITE, (0, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_captures(&actions, &[(0, 1)]);
    assert_moves(&actions, &[(0, 3), (1, 0), (1, 4), (3, 0), (3, 4), (4, 1), (4, 3)]);
}

// ── cannon: ANY_DISTANCE + DIRECTION_CROSS + JUMP_CAPTURE ─────────────────

#[test]
fn valid_moves_cannon_move_only_no_capture() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_CANNON, (2, 2)),
            (Piece::BLACK_PAWN, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 1), (2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_captures(&actions, &[]);
}

#[test]
fn valid_moves_cannon_jump_capture_over_one_screen() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_CANNON, (2, 3)),
            (Piece::BLACK_PAWN, (2, 1)),
            (Piece::BLACK_DOG, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 3);
    assert_moves(&actions, &[(2, 2), (2, 4), (0, 3), (1, 3), (3, 3), (4, 3)]);
    assert_captures(&actions, &[(2, 0)]);
}

#[test]
fn valid_moves_cannon_two_screens_no_jump_capture() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_CANNON, (2, 4)),
            (Piece::WHITE, (2, 2)),
            (Piece::WHITE, (2, 1)),
            (Piece::BLACK_HORSE, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 4);
    assert_moves(&actions, &[(1, 4), (2, 3), (3, 4), (4, 4)]);
    assert_captures(&actions, &[(2, 1)]);
    assert!(!actions.iter().any(|a| matches!(a, Action::Capture(Move { to: (2, 0), .. }))));
}

// ── river: ANY_DISTANCE + DIRECTION_CROSS + PUSH ──────────────────────────

#[test]
fn valid_moves_river_push_ally() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_RIVER, (2, 2)),
            (Piece::RED_PAWN, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_pushes(&actions, &[(2, 1)]);
    assert_eq!(actions.len(), 7);
}

#[test]
fn valid_moves_river_push_enemy() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_RIVER, (2, 2)),
            (Piece::WHITE, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_pushes(&actions, &[(2, 1)]);
    assert_eq!(actions.len(), 7);
}

#[test]
fn valid_moves_river_push_blocked_by_edge() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_RIVER, (2, 1)),
            (Piece::RED_PAWN, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 1);
    assert_moves(&actions, &[(2, 2), (2, 3), (2, 4), (0, 1), (1, 1), (3, 1), (4, 1)]);
    assert!(!actions.iter().any(|a| matches!(a, Action::Push(_) | Action::Capture(_))));
}

// ── wind: PASS_ENEMY + PASS_ALLY ─────────────────────────────────────────

#[test]
fn valid_moves_wind_passes_through_enemy() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_WIND, (2, 3)),
            (Piece::BLACK_PAWN, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 3);
    assert_moves(&actions, &[(2, 2), (2, 0), (2, 4), (0, 3), (1, 3), (3, 3), (4, 3)]);
    assert!(!actions.iter().any(|a| matches!(a, Action::Capture(_))));
}

// ── spy: controllable by both players ─────────────────────────────────────

#[test]
fn valid_moves_spy_red_controls_black_spy() {
    let g = game_one_3x3(Player::Red, Piece::BLACK_SPY, (1, 1));
    assert!(!g.valid_moves(1, 1).is_empty(), "Red should control black spy");
}

#[test]
fn valid_moves_spy_black_controls_red_spy() {
    let g = game_one_3x3(Player::Black, Piece::RED_SPY, (1, 1));
    assert!(!g.valid_moves(1, 1).is_empty(), "Black should control red spy");
}

// ── mine: mutual destruction ──────────────────────────────────────────────

#[test]
fn valid_moves_mine_capture_action_listed() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (2, 3)),
            (Piece::BLACK_MINE, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 3);
    assert_moves(&actions, &[(2, 2), (2, 4), (0, 3), (1, 3), (3, 3), (4, 3)]);
    assert_captures(&actions, &[(2, 1)]);
}

// ── valid_white_placements ───────────────────────────────────────────────

#[test]
fn valid_white_placements_returns_diagonal_positions() {
    let g = game_with_white_pool(
        Player::Red,
        &[
            (Piece::RED_WIZARD, (2, 2)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        1,
    );
    let mut got = g.valid_white_placements();
    got.sort_unstable();
    assert_eq!(got, vec![(1, 1), (1, 3), (3, 1), (3, 3)]);
}

#[test]
fn valid_white_placements_empty_when_placement_phase() {
    let mut board = Board::new(9, 10);
    board[(4, 7)] = Some(Piece::RED_WIZARD);
    board[(0, 9)] = Some(Piece::RED_GENERAL);
    board[(8, 0)] = Some(Piece::BLACK_GENERAL);
    let g = Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![],
        white: Piece::WHITE,
        white_pool: 2,
        result: GameResult::Unfinished,
    })
    .expect("valid");
    assert!(g.valid_white_placements().is_empty());
}

#[test]
fn valid_white_placements_empty_when_white_pool_zero() {
    let g = game_with_white_pool(
        Player::Red,
        &[
            (Piece::RED_WIZARD, (2, 2)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        0,
    );
    assert!(g.valid_white_placements().is_empty());
}

#[test]
fn valid_white_placements_empty_when_decided() {
    let mut g = game_with_white_pool(
        Player::Red,
        &[
            (Piece::RED_WIZARD, (2, 2)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        1,
    );
    g.action(Action::Resign(Player::Red)).expect("resign");
    assert!(g.valid_white_placements().is_empty());
}

#[test]
fn valid_white_placements_empty_when_no_control_white() {
    let g = game_with_white_pool(
        Player::Red,
        &[(Piece::RED_ROOK, (2, 2)), (Piece::RED_GENERAL, (0, 4)), (Piece::BLACK_GENERAL, (4, 0))],
        1,
    );
    assert!(g.valid_white_placements().is_empty());
}

// ── formation-granted CAPTURE + PUSH: no duplicate ────────────────────────

#[test]
fn valid_moves_river_in_spear_formation_capture_and_push_no_duplicate() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 1)),
            (Piece::RED_RIVER, (2, 2)),
            (Piece::BLACK_DOG, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_captures(&actions, &[(2, 3)]);
    assert_pushes(&actions, &[(2, 1), (2, 3)]);
    assert_eq!(actions.len(), 7, "4 moves + 1 capture + 2 pushes, no duplicates");
}

#[test]
fn valid_moves_river_push_blocked_escalates_to_single_capture() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 2)),
            (Piece::RED_RIVER, (2, 3)),
            (Piece::BLACK_DOG, (2, 4)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 3);
    assert_moves(&actions, &[(0, 3), (1, 3), (3, 3), (4, 3)]);
    assert_captures(&actions, &[(2, 4)]);
    assert_pushes(&actions, &[(2, 2)]);
    assert_eq!(actions.len(), 6, "4 moves + 1 capture + 1 push (ally spear), no duplicate capture");
}

// ── formation-granted CAPTURE + JUMP_CAPTURE: no duplicate ─────────────────

/// Cannon in Spear formation gains CAPTURE; in Wind formation gains
/// PASS_ENEMY.  A passable White piece at (2,2) sits between the cannon
/// and a Dog target at (2,3).  Without the guard the cannon would list
/// two identical Capture actions (one normal through the screen, one
/// jump capture over it).
#[test]
fn valid_moves_cannon_jump_capture_no_duplicate_when_normal_also_available() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 0)),
            (Piece::RED_WIND, (1, 2)),
            (Piece::RED_CANNON, (2, 1)),
            (Piece::WHITE, (2, 2)),
            (Piece::BLACK_DOG, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 1);
    assert_moves(&actions, &[(0, 1), (1, 1), (3, 1), (4, 1)]);
    assert_captures(&actions, &[(2, 2), (2, 3)]);
    assert_eq!(actions.len(), 6, "4 moves + 2 captures, no jump-capture duplicate at (2,3)");
}
