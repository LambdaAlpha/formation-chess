use formation_chess_core::ability::Ability;
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
        black_pool: vec![Piece::BLACK_PAWN],
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
    assert_pushes(&actions, &[(2, 0)]);
    assert_eq!(actions.len(), 8);
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
        black_pool: vec![Piece::BLACK_PAWN],
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
    assert_pushes(&actions, &[(2, 2), (2, 4)]);
    assert_eq!(actions.len(), 7, "4 moves + 1 capture + 2 pushes, no duplicate capture");
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

// ── try_xxx returns original (not effective) pieces in PositionChange ───────

/// Red Rook at (0,1) strips ANY_DISTANCE from enemy Black Rook at (1,1).
/// `try_move` must return the original Black Rook in the PositionChange, not
/// the effective one that lost ANY_DISTANCE.
#[test]
fn try_move_returns_original_piece_not_effective() {
    let mut board = Board::new(3, 4);
    board[(0, 1)] = Some(Piece::RED_ROOK);
    board[(1, 1)] = Some(Piece::BLACK_ROOK);

    let changes = board.try_move((1, 1), (1, 2)).expect("1-step move should succeed");

    let placed = changes.iter().find(|c| c.at == (1, 2)).unwrap().piece.unwrap();
    assert_eq!(placed, Piece::BLACK_ROOK, "PositionChange must return the original piece");
    assert!(
        placed.ability.has(Ability::ANY_DISTANCE),
        "original Black Rook must retain ANY_DISTANCE"
    );
}

/// Red Spear at (2,1) grants CAPTURE to ally Red River at (2,2) via formation.
/// `try_capture` must return the original River in the PositionChange, not
/// the effective one that gained CAPTURE.
#[test]
fn try_capture_returns_original_mover_not_effective() {
    let mut board = Board::new(5, 5);
    board[(2, 1)] = Some(Piece::RED_SPEAR);
    board[(2, 2)] = Some(Piece::RED_RIVER);
    board[(2, 3)] = Some(Piece::BLACK_GENERAL);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let changes = board.try_capture((2, 2), (2, 3)).expect("capture should succeed");

    let placed = changes.iter().find(|c| c.at == (2, 3)).unwrap().piece.unwrap();
    assert_eq!(placed, Piece::RED_RIVER, "PositionChange must return the original piece");
    assert!(
        !placed.ability.has(Ability::CAPTURE),
        "original River must NOT have CAPTURE (that came from formation)"
    );
}

/// Red Spear (2,1) grants CAPTURE to Red River (2,2). Black Rook (1,3) grants
/// ANY_DISTANCE to Black Dog (2,3).  Red River pushes Black Dog to (2,4).
/// `try_push` must return the *original* River and Dog in PositionChanges,
/// not the effective versions with modified abilities.
#[test]
fn try_push_returns_original_pieces_not_effective() {
    let mut board = Board::new(5, 5);
    board[(2, 1)] = Some(Piece::RED_SPEAR);
    board[(2, 2)] = Some(Piece::RED_RIVER);
    board[(2, 3)] = Some(Piece::BLACK_DOG);
    board[(1, 3)] = Some(Piece::BLACK_ROOK);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let changes = board.try_push((2, 2), (2, 3)).expect("push should succeed");

    // The mover occupies the target's old position.
    let mover = changes.iter().find(|c| c.at == (2, 3)).unwrap().piece.unwrap();
    assert_eq!(mover, Piece::RED_RIVER, "mover PositionChange must return original River");
    assert!(!mover.ability.has(Ability::CAPTURE), "original River must NOT have CAPTURE");

    // The pushed piece lands one step further.
    let pushed = changes.iter().find(|c| c.at == (2, 4)).unwrap().piece.unwrap();
    assert_eq!(pushed, Piece::BLACK_DOG, "pushed PositionChange must return original Dog");
    assert!(!pushed.ability.has(Ability::ANY_DISTANCE), "original Dog must NOT have ANY_DISTANCE");
}

// ── draw via general formation ────────────────────────────────────────────

/// Red Pawn in Red General's CORNERS formation gains DRAW and may draw
/// with the Black General.
#[test]
fn general_formation_grants_draw_to_ally() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (1, 1)), (Piece::RED_PAWN, (0, 0)), (Piece::BLACK_GENERAL, (0, 1))],
        5,
        5,
    );
    let actions = g.valid_moves(0, 0);
    // Pawn at (0,0) can move right to (1,0)
    assert_moves(&actions, &[(1, 0)]);
    // Can also capture and draw at (0,1) where Black General sits
    assert_captures(&actions, &[(0, 1)]);
    assert!(
        actions.iter().any(|a| matches!(a, Action::Draw(Move { to: (0, 1), .. }))),
        "pawn in general's formation should gain DRAW"
    );
}

/// Red Pawn near Red General but NOT in formation does not gain DRAW.
#[test]
fn pawn_outside_general_formation_has_no_draw() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (2, 2)), (Piece::RED_PAWN, (2, 1)), (Piece::BLACK_GENERAL, (2, 0))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 1);
    // Pawn at (2,1) is above general at (2,2): dy=-1, dx=0 → EDGE, not CORNERS
    // Black general at (2,0) is 1 step up
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Draw(_))),
        "pawn outside general's formation should not have DRAW"
    );
}

/// Black General inside Red General's CORNERS formation loses DRAW.
#[test]
fn enemy_general_formation_strips_draw() {
    let g = game_with(
        Player::Black,
        &[(Piece::RED_GENERAL, (1, 1)), (Piece::BLACK_GENERAL, (2, 2))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    // Black General at (2,2) in CORNERS of Red General at (1,1): loses DRAW
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Draw(_))),
        "general in enemy formation should have DRAW stripped"
    );
}

/// Red General itself has DRAW by default and may draw with Black General.
#[test]
fn general_draws_own_ability() {
    let g = game_with(
        Player::Red,
        &[(Piece::BLACK_GENERAL, (0, 1)), (Piece::RED_GENERAL, (0, 2))],
        5,
        5,
    );
    let actions = g.valid_moves(0, 2);
    assert!(
        actions.iter().any(|a| matches!(a, Action::Draw(Move { to: (0, 1), .. }))),
        "general should have DRAW by default"
    );
}

/// Draw action rejected when the target is a friendly piece.
#[test]
fn draw_rejected_against_friendly() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (0, 2)), (Piece::RED_PAWN, (1, 2)), (Piece::BLACK_GENERAL, (4, 2))],
        5,
        5,
    );
    // Pawn at (1,2) is not in CORNERS of (0,2) (dx=+1, dy=0 → EDGE)
    // So pawn doesn't have DRAW anyway. But even if it did, (1,2)→(0,2)
    // would be a friendly target — try_draw should reject.
    let result = g.try_action(Action::Draw(Move { from: (1, 2), to: (0, 2) }));
    assert!(result.is_err(), "draw against friendly piece should be rejected");
}

/// Draw action rejected when the mover lacks DRAW.
#[test]
fn draw_rejected_without_draw_ability() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_PAWN, (2, 2)), (Piece::BLACK_GENERAL, (2, 1)), (Piece::RED_GENERAL, (4, 4))],
        5,
        5,
    );
    let result = g.try_action(Action::Draw(Move { from: (2, 2), to: (2, 1) }));
    assert!(result.is_err(), "draw without DRAW ability should be rejected");
}

/// Draw action rejected when target is not a vital piece.
#[test]
fn draw_rejected_against_non_vital() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_GENERAL, (2, 2)),
            (Piece::BLACK_PAWN, (2, 1)),
            (Piece::BLACK_GENERAL, (4, 4)),
        ],
        5,
        5,
    );
    let result = g.try_action(Action::Draw(Move { from: (2, 2), to: (2, 1) }));
    assert!(result.is_err(), "draw against non-vital should be rejected");
}
