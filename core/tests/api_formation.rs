use formation_chess_core::ability::Ability;
use formation_chess_core::board::Board;
use formation_chess_core::board::Neighbor;
use formation_chess_core::formation::Formation;
use formation_chess_core::piece::Color;
use formation_chess_core::piece::Piece;

#[test]
fn formation_contains_rejects_offsets_outside_the_local_zone() {
    assert!(!Formation::GENERAL.contains(0, 0));
    assert!(!Formation::ROOK.contains(0, 0));
    assert!(!Formation::GENERAL.contains(2, 0));
    assert!(!Formation::ROOK.contains(-2, 2));
}

#[test]
fn formation_middle_constants_match_covered_offsets() {
    let left = Formation { points: Formation::MIDDLE_LEFT, effect: Formation::general };
    assert!(left.contains(-1, 0));
    assert!(!left.contains(1, 0));

    let right = Formation { points: Formation::MIDDLE_RIGHT, effect: Formation::general };
    assert!(right.contains(1, 0));
    assert!(!right.contains(-1, 0));
}

#[test]
fn formation_black_orientation_flips_the_vertical_pattern() {
    let red = Piece::RED_WIND.formation;
    let black = Piece::BLACK_WIND.formation;

    assert!(red.contains(0, -1));
    assert!(red.contains(-1, 1));
    assert!(red.contains(1, 1));
    assert!(!red.contains(-1, -1));

    assert!(black.contains(-1, -1));
    assert!(black.contains(1, -1));
    assert!(black.contains(0, 1));
    assert!(!black.contains(-1, 1));
}

#[test]
fn formation_effects_ignore_neutral_targets_except_for_control() {
    #[expect(clippy::type_complexity)]
    let effects: [fn(Color, Color) -> (Ability, Ability); 13] = [
        Formation::general,
        Formation::scholar,
        Formation::pawn,
        Formation::rook,
        Formation::horse,
        Formation::wind,
        Formation::mountain,
        Formation::fire,
        Formation::forest,
        Formation::spear,
        Formation::shield,
        Formation::shell,
        Formation::mine,
    ];

    for owner in [Color::Red, Color::Black] {
        for effect in effects {
            assert_eq!(effect(owner, Color::White), (Ability::NONE, Ability::NONE));
        }
    }
}

#[test]
fn formation_control_effects_can_control_neutral_targets() {
    assert_eq!(
        Formation::army(Color::Red, Color::White),
        (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED)
    );
    assert_eq!(
        Formation::agent(Color::Black, Color::White),
        (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
    );
    assert_eq!(
        Formation::spy(Color::Red, Color::White),
        (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
    );
}

#[test]
fn effective_formation_updates_are_order_independent_and_denial_wins() {
    let granting_neighbor = Neighbor { dx: 0, dy: -1, piece: Some(Piece::RED_PAWN) };
    let denying_neighbor = Neighbor { dx: 0, dy: 1, piece: Some(Piece::BLACK_PAWN) };

    let mut granted = Piece::RED_WIND;
    granted.take_effect(&[granting_neighbor]);
    assert!(granted.ability.has(Ability::DIRECTION_CROSS));

    let mut forward = Piece::RED_WIND;
    forward.take_effect(&[granting_neighbor, denying_neighbor]);
    let mut reverse = Piece::RED_WIND;
    reverse.take_effect(&[denying_neighbor, granting_neighbor]);

    assert_eq!(forward.ability, reverse.ability);
    assert!(!forward.ability.has(Ability::DIRECTION_CROSS));
}

#[test]
fn neutral_target_keeps_base_abilities_under_non_control_formation() {
    let mut board = Board::new(3, 3);
    board[(1, 0)] = Some(Piece::RED_ROOK);
    board[(1, 1)] = Some(Piece::WHITE);

    assert_eq!(board.effective((1, 1)).expect("neutral target").ability, Piece::WHITE.ability);
}
