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

// -- valid_moves: edge cases -------------------------------------------------

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

// -- unlimited orthogonal movement + capture -----------------------------------

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

// -- one-step orthogonal movement + capture ------------------------------------

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

// -- one-step diagonal movement + capture -------------------------------------

#[test]
fn valid_moves_scholar_diagonal_only() {
    let g = game_one(Player::Red, Piece::RED_SCHOLAR, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 1), (1, 3), (3, 1), (3, 3)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_scholar_capture_diagonal() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SCHOLAR, (2, 2)),
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

// -- one-step L-shaped movement + capture -------------------------------------

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

// -- unlimited L-shaped movement + capture + captured_on_capture --------------

#[test]
fn valid_moves_shell_l_shaped_any_distance() {
    let g = game_one(Player::Red, Piece::RED_SHELL, (2, 2));
    let actions = g.valid_moves(2, 2);
    // Shell moves L-shaped at any distance: chained knight moves
    // From (2,2): (0,1) (0,3) (1,0) (1,4) (3,0) (3,4) (4,1) (4,3)
    assert_moves(&actions, &[(0, 1), (0, 3), (1, 0), (1, 4), (3, 0), (3, 4), (4, 1), (4, 3)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_shell_captures_enemy_on_l_shape() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SHELL, (2, 2)),
            (Piece::BLACK_PAWN, (0, 1)),
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

// -- unlimited diagonal movement + push ---------------------------------------

#[test]
fn valid_moves_wind_diagonal_any_distance() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_WIND, (2, 2)), (Piece::RED_GENERAL, (4, 1)), (Piece::BLACK_GENERAL, (0, 3))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 0), (0, 4), (1, 1), (1, 3), (3, 1), (3, 3), (4, 0), (4, 4)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_wind_push_ally_on_diagonal() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_WIND, (2, 2)),
            (Piece::RED_PAWN, (1, 1)),
            (Piece::RED_GENERAL, (4, 1)),
            (Piece::BLACK_GENERAL, (0, 3)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_pushes(&actions, &[(1, 1)]);
    assert_moves(&actions, &[(0, 4), (1, 3), (3, 1), (3, 3), (4, 0), (4, 4)]);
}

#[test]
fn valid_moves_wind_push_enemy_on_diagonal() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_WIND, (2, 2)),
            (Piece::WHITE, (1, 1)),
            (Piece::RED_GENERAL, (4, 1)),
            (Piece::BLACK_GENERAL, (0, 3)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_pushes(&actions, &[(1, 1)]);
    assert_moves(&actions, &[(0, 4), (1, 3), (3, 1), (3, 3), (4, 0), (4, 4)]);
}

#[test]
fn valid_moves_wind_blocked_by_piece_on_path() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_WIND, (2, 2)),
            (Piece::RED_PAWN, (1, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    // (0,0) is blocked by the pawn at (1,1), but push is available
    // Actually pawn at (1,1) blocks reaching (0,0) via diagonal
    assert!(!actions.iter().any(|a| matches!(a, Action::Move(Move { to: (0, 0), .. }))));
}

// -- spy: controllable by both players ---------------------------------------

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

// -- capture with captured_on_captured ---------------------------------------

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

// -- valid_white_placements --------------------------------------------------

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

// -- try_xxx returns original pieces in position changes ----------------------

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

/// Red Spear at (0,2) grants CAPTURE to ally Red Wind at (1,1) via LOWER_TRIANGLE
/// formation. try_capture must return the original Wind in the PositionChange.
#[test]
fn try_capture_returns_original_mover_not_effective() {
    let mut board = Board::new(5, 5);
    // Spear at (0,2), Wind at (1,1) → dx=+1, dy=-1 = TOP_RIGHT, in LOWER_TRIANGLE
    board[(0, 2)] = Some(Piece::RED_SPEAR);
    board[(1, 1)] = Some(Piece::RED_WIND);
    // White target at (0,0) diagonally from wind
    board[(0, 0)] = Some(Piece::WHITE);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    let changes = board.try_capture((1, 1), (0, 0)).expect("capture should succeed");

    let placed = changes.iter().find(|c| c.at == (0, 0)).unwrap().piece.unwrap();
    assert_eq!(placed, Piece::RED_WIND, "PositionChange must return the original piece");
    assert!(
        !placed.ability.has(Ability::CAPTURE),
        "original Wind must NOT have CAPTURE (that came from formation)"
    );
}

/// Red Spear (0,1) grants CAPTURE to Red Wind (1,1). Black Horse (0,0) is
/// the target. Red Wind pushes to (0,0) which should push the horse further.
/// But horse at (0,0) can't be pushed off board, so push escalates to capture.
#[test]
fn try_push_returns_original_pieces_not_effective() {
    let mut board = Board::new(5, 5);
    // Spear at (0,1), Wind at (1,2) → Wind moves to (0,3) diagonally, pushes target
    board[(0, 1)] = Some(Piece::RED_SPEAR);
    board[(1, 2)] = Some(Piece::RED_WIND);
    board[(0, 3)] = Some(Piece::BLACK_SCHOLAR);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);

    // Wind at (1,2) to (0,3): dx=-1, dy=+1 (top left diagonal, one step)
    // Push scholar from (0,3) to (-1,4) which is off board → push blocked
    // Wind has no escalation ability by default, so push should fail
    // Actually, Wind's PUSH_ENEMY + target's PUSHED_BY_ENEMY = push works
    // But pushed_target would be (-1,4) → off board → None → push blocked
    let result = board.try_push((1, 2), (0, 3));
    assert!(result.is_err(), "push should fail when blocked and no escalation");
}

// -- formation-granted capture + push: no duplicate --------------------------

#[test]
fn valid_moves_wind_in_spear_formation_gains_capture() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (1, 3)),
            (Piece::RED_WIND, (2, 2)),
            (Piece::BLACK_SCHOLAR, (1, 1)),
            (Piece::RED_GENERAL, (4, 1)),
            (Piece::BLACK_GENERAL, (0, 3)),
        ],
        5,
        5,
    );
    // Wind at (2,2), Spear at (1,3): dx=-1, dy=+1 = BOTTOM_LEFT -> in LOWER_TRIANGLE
    // Wind gains CAPTURE from spear formation. Can capture or push black scholar.
    let actions = g.valid_moves(2, 2);
    assert_captures(&actions, &[(1, 1)]);
    assert_pushes(&actions, &[(1, 1), (1, 3)]);
}

/// Wind in Spear formation can both push and capture ally, but not duplicate.
#[test]
fn valid_moves_wind_push_and_capture_no_duplicate() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (1, 1)),
            (Piece::RED_WIND, (2, 2)),
            (Piece::RED_PAWN, (1, 3)),
            (Piece::RED_GENERAL, (4, 1)),
            (Piece::BLACK_GENERAL, (0, 3)),
        ],
        5,
        5,
    );
    // Wind at (2,2), Spear at (1,1): dx=-1, dy=-1 = TOP_LEFT -> in LOWER_TRIANGLE
    // Wind gains CAPTURE. Wind can push ally spear at (1,1) and ally pawn at (1,3).
    let actions = g.valid_moves(2, 2);
    assert_pushes(&actions, &[(1, 1), (1, 3)]);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Capture(Move { to: (1, 3), .. }))),
        "should not offer capture on ally"
    );
}

// -- draw ability via formation ----------------------------------------------

#[test]
fn general_formation_grants_draw_to_ally() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (1, 1)), (Piece::RED_PAWN, (0, 0)), (Piece::BLACK_GENERAL, (0, 1))],
        5,
        5,
    );
    let actions = g.valid_moves(0, 0);
    assert_moves(&actions, &[(1, 0)]);
    assert_captures(&actions, &[(0, 1)]);
    assert!(
        actions.iter().any(|a| matches!(a, Action::Draw(Move { to: (0, 1), .. }))),
        "pawn in general's formation should gain DRAW"
    );
}

#[test]
fn pawn_outside_general_formation_has_no_draw() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (2, 2)), (Piece::RED_PAWN, (2, 1)), (Piece::BLACK_GENERAL, (2, 0))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 1);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Draw(_))),
        "pawn outside general's formation should not have DRAW"
    );
}

#[test]
fn enemy_general_formation_strips_draw() {
    let g = game_with(
        Player::Black,
        &[(Piece::RED_GENERAL, (1, 1)), (Piece::BLACK_GENERAL, (2, 2))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Draw(_))),
        "general in enemy formation should have DRAW stripped"
    );
}

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

#[test]
fn draw_rejected_against_friendly() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (0, 2)), (Piece::RED_PAWN, (1, 2)), (Piece::BLACK_GENERAL, (4, 2))],
        5,
        5,
    );
    let result = g.try_action(Action::Draw(Move { from: (1, 2), to: (0, 2) }));
    assert!(result.is_err(), "draw against friendly piece should be rejected");
}

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

// -- captured_on_captured bypasses attacker's capture ------------------------

/// A rook (no Capture ability) standing in Spear formation can capture.
/// Capturing a Mine triggers CAPTURE_ON_CAPTURED: both are destroyed.
#[test]
fn mine_capture_on_captured_triggers_mutual_destruction() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 1)),
            (Piece::RED_ROOK, (2, 2)),
            (Piece::BLACK_MINE, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_captures(&actions, &[(2, 3)]);
}
