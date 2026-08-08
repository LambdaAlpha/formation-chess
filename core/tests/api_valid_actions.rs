use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

mod support;
use support::api::assert_captures;
use support::api::assert_moves;
use support::api::assert_pulls;
use support::api::assert_pushes;
use support::api::game_one;
use support::api::game_with;

#[test]
fn board_valid_moves_appends_resign_for_controlled_vital() {
    let mut controlled_black_general = Piece::BLACK_GENERAL;
    controlled_black_general.ability.add(Ability::CONTROLLED_BY_RED);
    let mut board = Board::new(5, 5);
    board[(2, 2)] = Some(controlled_black_general);
    let mut actions = Vec::new();

    board.valid_moves(Player::Red, (2, 2), &mut actions);

    assert_eq!(actions.last(), Some(&Action::Resign(2, 2)));
}

#[test]
fn board_valid_moves_omits_resign_for_uncontrolled_vital() {
    let mut board = Board::new(5, 5);
    board[(2, 2)] = Some(Piece::BLACK_GENERAL);
    let mut actions = Vec::new();

    board.valid_moves(Player::Red, (2, 2), &mut actions);

    assert!(!actions.iter().any(|action| matches!(action, Action::Resign(..))));
}

// -- all_valid_moves ---------------------------------------------------------

#[test]
fn all_valid_moves_matches_controlled_piece_queries() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (0, 2)),
            (Piece::BLACK_ROOK, (4, 2)),
            (Piece::RED_PAWN, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );

    let mut expected = g.valid_moves(0, 2);
    expected.extend(g.valid_moves(2, 3));
    expected.extend(g.valid_moves(0, 4));

    let mut actions = Vec::new();
    g.all_valid_moves(&mut actions);
    assert_eq!(actions, expected);
}

#[test]
fn all_valid_moves_appends_to_existing_actions() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let prefix = Action::Resign(9, 9);
    let mut actions = vec![prefix];

    g.all_valid_moves(&mut actions);

    assert_eq!(actions[0], prefix);
    assert!(actions.len() > 1);
}

#[test]
fn all_valid_moves_outside_unfinished_movement_phase_appends_nothing() {
    let sentinel = Action::Resign(9, 9);
    let mut actions = vec![sentinel];
    Game::default().all_valid_moves(&mut actions);
    assert_eq!(actions, [sentinel]);

    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    g.action(Action::Resign(0, 4)).expect("resign");
    g.all_valid_moves(&mut actions);
    assert_eq!(actions, [sentinel]);
}

// -- valid_moves: edge cases -------------------------------------------------

#[test]
fn valid_moves_empty_origin_returns_empty() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    assert!(g.valid_moves(1, 1).is_empty());
}

#[test]
fn valid_moves_during_placement_phase_returns_empty() {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    let g = Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_ROOK],
        black_pool: vec![Piece::BLACK_PAWN],
        result: GameResult::Unfinished,
    })
    .expect("valid");
    assert!(g.valid_moves(0, 4).is_empty());
}

#[test]
fn valid_moves_decided_game_returns_empty() {
    let mut g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    g.action(Action::Resign(0, 4)).expect("resign");
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
fn valid_moves_cross_any_distance_on_open_board() {
    let g = game_one(Player::Red, Piece::RED_ROOK, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 0), (2, 1), (2, 3), (2, 4), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert!(actions.iter().all(|a| matches!(a, Action::Move(_))));
}

#[test]
fn valid_moves_cross_capture_opponent_target() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_ROOK, (2, 2)),
            (Piece::BLACK_PAWN, (2, 0)),
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
fn valid_moves_cross_blocked_by_ally() {
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

// -- DIRECTION_CROSS ----------------------------------------------------------

#[test]
fn valid_moves_cross_one_step_all_directions() {
    let g = game_one(Player::Red, Piece::RED_PAWN, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 2), (2, 1), (2, 3), (3, 2)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_cross_capture_and_move() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_PAWN, (2, 2)),
            (Piece::BLACK_PAWN, (2, 1)),
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

// -- DIRECTION_DIAGONAL -------------------------------------------------------

#[test]
fn valid_moves_diagonal_one_step_only() {
    let g = game_one(Player::Red, Piece::RED_SCHOLAR, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 1), (1, 3), (3, 1), (3, 3)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_diagonal_capture_and_move() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SCHOLAR, (2, 2)),
            (Piece::BLACK_PAWN, (1, 1)),
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

// -- DIRECTION_SHAPE_L --------------------------------------------------------

#[test]
fn valid_moves_shape_l_one_step_reaches_eight_points() {
    let g = game_one(Player::Red, Piece::RED_HORSE, (2, 2));
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 1), (0, 3), (1, 0), (1, 4), (3, 0), (3, 4), (4, 1), (4, 3)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_shape_l_leg_blocking_removes_targets() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_HORSE, (2, 2)),
            (Piece::BLACK_PAWN, (2, 1)),
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
fn valid_moves_shape_l_capture_and_move() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_HORSE, (2, 2)),
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

// -- DIRECTION_SHAPE_L + ANY_DISTANCE ----------------------------------------

#[test]
fn valid_moves_shape_l_any_distance_reaches_eight_points() {
    let long_horse =
        Piece { ability: Piece::RED_HORSE.ability | Ability::ANY_DISTANCE, ..Piece::RED_HORSE };
    let g = game_one(Player::Red, long_horse, (2, 2));
    let actions = g.valid_moves(2, 2);
    // Shell moves L-shaped at any distance: chained knight moves
    // From (2,2): (0,1) (0,3) (1,0) (1,4) (3,0) (3,4) (4,1) (4,3)
    assert_moves(&actions, &[(0, 1), (0, 3), (1, 0), (1, 4), (3, 0), (3, 4), (4, 1), (4, 3)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_shape_l_capture_target() {
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

// -- DIRECTION_DIAGONAL + PUSH -----------------------------------------------

#[test]
fn valid_moves_diagonal_any_distance_on_open_board() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_STRATAGEM, (2, 2)),
            (Piece::RED_GENERAL, (4, 1)),
            (Piece::BLACK_GENERAL, (0, 3)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 0), (0, 4), (1, 1), (1, 3), (3, 1), (3, 3), (4, 0), (4, 4)]);
    assert_eq!(actions.len(), 8);
}

#[test]
fn valid_moves_diagonal_pushes_an_ally() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_STRATAGEM, (2, 2)),
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
fn valid_moves_diagonal_pushes_an_opponent_target() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_STRATAGEM, (2, 2)),
            (Piece::BLACK_PAWN, (1, 1)),
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
fn valid_moves_diagonal_path_blocking_stops_scan() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_STRATAGEM, (2, 2)),
            (Piece::RED_PAWN, (1, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(1, 3), (3, 1), (3, 3), (4, 4)]);
    assert_pushes(&actions, &[(1, 1)]);
    assert_pulls(&actions, &[(3, 3), (4, 4)]);
    assert_eq!(actions.len(), 7);
}

// -- CONTROLLED_BY_* ----------------------------------------------------------

#[test]
fn valid_moves_foreign_control_grant_allows_red() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::BLACK_ROOK, (1, 1)),
            (Piece::RED_STRATAGEM, (0, 0)),
            (Piece::RED_GENERAL, (0, 2)),
            (Piece::BLACK_GENERAL, (2, 0)),
        ],
        3,
        3,
    );
    let actions = g.valid_moves(1, 1);
    assert_moves(&actions, &[(1, 0), (0, 1), (2, 1), (1, 2)]);
    assert_eq!(actions.len(), 4);
}

#[test]
fn valid_moves_foreign_control_grant_allows_black() {
    let g = game_with(
        Player::Black,
        &[
            (Piece::RED_ROOK, (1, 1)),
            (Piece::BLACK_STRATAGEM, (2, 2)),
            (Piece::RED_GENERAL, (0, 2)),
            (Piece::BLACK_GENERAL, (2, 0)),
        ],
        3,
        3,
    );
    let actions = g.valid_moves(1, 1);
    assert_moves(&actions, &[(1, 0), (0, 1), (2, 1), (1, 2)]);
    assert_eq!(actions.len(), 4);
}

// -- CAPTURE_ON_CAPTURED ------------------------------------------------------

#[test]
fn valid_moves_mutual_destruction_target_is_capturable() {
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

// -- formation-granted CAPTURE + PUSH -----------------------------------------

#[test]
fn valid_moves_formation_grant_adds_capture_action() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 3)),
            (Piece::RED_FIRE, (2, 2)),
            (Piece::BLACK_SCHOLAR, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    // The spear directly below the fire covers its top-middle point.
    // The fire therefore gains CAPTURE while retaining its cross movement.
    let actions = g.valid_moves(2, 2);
    assert_captures(&actions, &[(2, 1), (2, 3)]);
    assert_pushes(&actions, &[(2, 1), (2, 3)]);
}

/// Allied pieces are capturable when the normal CAPTURE/CAPTURED contract is met.
#[test]
fn valid_moves_same_player_targets_offer_capture_and_push() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_SPEAR, (2, 3)),
            (Piece::RED_FIRE, (2, 2)),
            (Piece::RED_PAWN, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_captures(&actions, &[(2, 1), (2, 3)]);
    assert_pushes(&actions, &[(2, 1), (2, 3)]);
    assert_eq!(actions.len(), 8);
}

// -- DRAW actions -------------------------------------------------------------

#[test]
fn valid_moves_formation_grant_adds_draw_action() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_GENERAL, (1, 1)), (Piece::RED_PAWN, (0, 0)), (Piece::BLACK_GENERAL, (0, 1))],
        5,
        5,
    );
    let actions = g.valid_moves(0, 0);
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Draw(Move { from: (0, 0), to: (0, 1) })))
    );
}

#[test]
fn valid_moves_draw_requires_an_opponent_vital_target() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_GENERAL, (1, 1)),
            (Piece::RED_PAWN, (2, 2)),
            (Piece::BLACK_PAWN, (2, 1)),
            (Piece::BLACK_GENERAL, (4, 4)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert!(!actions.iter().any(|action| matches!(action, Action::Draw(_))));
}

// -- blocked PUSH escalation --------------------------------------------------

#[test]
fn valid_moves_active_push_escalation_includes_blocked_landing() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_MOMENTUM, (1, 1)),
            (Piece::BLACK_PAWN, (2, 0)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(1, 1);
    assert_pushes(&actions, &[(2, 0)]);
}

#[test]
fn valid_moves_passive_push_escalation_includes_blocked_landing() {
    let passive_target = Piece {
        formation: Piece::RED_SPEAR.formation,
        ability: Piece::BLACK_PAWN.ability | Ability::CAPTURED_ON_PUSH_BLOCKED,
        ..Piece::BLACK_PAWN
    };
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_FIRE, (1, 1)),
            (passive_target, (2, 1)),
            (Piece::RED_PAWN, (3, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(1, 1);
    assert_pushes(&actions, &[(2, 1)]);
}

// -- CAPTURE_ON_CAPTURED bypasses the attacker's CAPTURE requirement -------

#[test]
fn valid_moves_mutual_destruction_target_bypasses_capture_ability() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_STRATAGEM, (2, 2)),
            (Piece::BLACK_MINE, (3, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    assert!(!Piece::RED_STRATAGEM.ability.has(Ability::CAPTURE));
    let actions = g.valid_moves(2, 2);
    assert_captures(&actions, &[(3, 3)]);
}

// -- PULL actions --------------------------------------------------------------

#[test]
fn valid_moves_pull_adds_action_alongside_move() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_FIRE, (2, 2)),
            (Piece::RED_PAWN, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_moves(&actions, &[(2, 0), (2, 1), (0, 2), (1, 2), (3, 2), (4, 2)]);
    assert_pulls(&actions, &[(2, 0), (2, 1)]);
}

#[test]
fn valid_moves_pull_requires_a_source_behind_the_origin() {
    let g = game_with(
        Player::Red,
        &[(Piece::RED_FIRE, (2, 2)), (Piece::RED_GENERAL, (0, 4)), (Piece::BLACK_GENERAL, (4, 0))],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_pulls(&actions, &[]);
}

#[test]
fn valid_moves_pull_requires_the_mover_ability() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_PAWN, (2, 2)),
            (Piece::RED_PAWN, (2, 3)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_pulls(&actions, &[]);
}

#[test]
fn valid_moves_pull_requires_an_empty_destination() {
    let g = game_with(
        Player::Red,
        &[
            (Piece::RED_FIRE, (2, 2)),
            (Piece::RED_PAWN, (2, 3)),
            (Piece::BLACK_PAWN, (2, 1)),
            (Piece::RED_GENERAL, (0, 4)),
            (Piece::BLACK_GENERAL, (4, 0)),
        ],
        5,
        5,
    );
    let actions = g.valid_moves(2, 2);
    assert_pulls(&actions, &[]);
}
