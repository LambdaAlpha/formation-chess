use formation_chess_core::ability::Ability;
use formation_chess_core::board::Board;
use formation_chess_core::board::Neighbor;
use formation_chess_core::formation::Formation;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

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
fn piece_constants_match_current_names_and_group_order() {
    let strategy_group =
        [Piece::RED_GENERAL, Piece::RED_STRATAGEM, Piece::RED_MOMENTUM, Piece::RED_ADAPTATION];
    assert_eq!(strategy_group.map(|piece| piece.name), ['将', '计', '势', '变']);

    let restraint_group =
        [Piece::RED_WIND, Piece::RED_FOREST, Piece::RED_FIRE, Piece::RED_MOUNTAIN];
    assert_eq!(restraint_group.map(|piece| piece.name), ['风', '林', '火', '山']);

    assert_eq!(&Piece::RED_PLAYER_PIECES[.. 8], &[
        Piece::RED_GENERAL,
        Piece::RED_STRATAGEM,
        Piece::RED_MOMENTUM,
        Piece::RED_ADAPTATION,
        Piece::RED_WIND,
        Piece::RED_FOREST,
        Piece::RED_FIRE,
        Piece::RED_MOUNTAIN,
    ]);
    assert_eq!(&Piece::BLACK_PLAYER_PIECES[.. 8], &[
        Piece::BLACK_GENERAL,
        Piece::BLACK_STRATAGEM,
        Piece::BLACK_MOMENTUM,
        Piece::BLACK_ADAPTATION,
        Piece::BLACK_WIND,
        Piece::BLACK_FOREST,
        Piece::BLACK_FIRE,
        Piece::BLACK_MOUNTAIN,
    ]);
}

#[test]
fn formation_shapes_match_the_four_piece_groups() {
    for formation in
        [Formation::GENERAL, Formation::STRATAGEM, Formation::MOMENTUM, Formation::ADAPTATION]
    {
        assert_eq!(formation.points, Formation::CORNER);
    }
    for formation in [Formation::WIND, Formation::FOREST, Formation::FIRE, Formation::MOUNTAIN] {
        assert_eq!(formation.points, Formation::MIDDLE);
    }
    for formation in [Formation::SPEAR, Formation::SHIELD, Formation::SHELL, Formation::MINE] {
        assert_eq!(formation.points, Formation::UPPER_TRIANGLE);
    }
    for formation in [Formation::SCHOLAR, Formation::PAWN, Formation::HORSE, Formation::ROOK] {
        assert_eq!(formation.points, Formation::LOWER_PENTAGON);
    }
}

#[test]
fn formation_black_orientation_flips_the_vertical_pattern() {
    let red = Piece::RED_SPEAR.formation;
    let black = Piece::BLACK_SPEAR.formation;

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
fn formation_effects_use_player_relationships() {
    let grants = [
        (Formation::general as fn(Player, Player) -> _, Ability::PEACE_TALK),
        (Formation::scholar, Ability::DIAGONAL_MOVE),
        (Formation::pawn, Ability::ORTHOGONAL_MOVE),
        (Formation::horse, Ability::BROAD_STEP),
        (Formation::rook, Ability::SWIFT_MOVE),
        (Formation::fire, Ability::PUSH_FRIEND | Ability::PUSH_ENEMY),
        (Formation::wind, Ability::PULL_FRIEND | Ability::PULL_ENEMY),
        (Formation::spear, Ability::CAPTURE),
        (Formation::shell, Ability::FORCE_CAPTURE),
        (Formation::mine, Ability::COUNTER_CAPTURE),
    ];

    for (effect, ability) in grants {
        assert_eq!(effect(Player::Red, Player::Red), (ability, ability));
        assert_eq!(effect(Player::Red, Player::Black), (ability, Ability::NONE));
    }

    assert_eq!(Formation::shield(Player::Red, Player::Red), (Ability::CAPTURABLE, Ability::NONE));
    assert_eq!(
        Formation::shield(Player::Red, Player::Black),
        (Ability::CAPTURABLE, Ability::CAPTURABLE)
    );
    assert_eq!(
        Formation::mountain(Player::Red, Player::Red),
        (Ability::FRIEND_PUSH | Ability::ENEMY_PUSH, Ability::FRIEND_PUSH)
    );
    assert_eq!(
        Formation::mountain(Player::Red, Player::Black),
        (Ability::FRIEND_PUSH | Ability::ENEMY_PUSH, Ability::ENEMY_PUSH)
    );
    assert_eq!(
        Formation::forest(Player::Red, Player::Red),
        (Ability::FRIEND_PULL | Ability::ENEMY_PULL, Ability::FRIEND_PULL)
    );
    assert_eq!(
        Formation::forest(Player::Red, Player::Black),
        (Ability::FRIEND_PULL | Ability::ENEMY_PULL, Ability::ENEMY_PULL)
    );
    assert_eq!(
        Formation::stratagem(Player::Red, Player::Black),
        (Ability::PASSIVITY, Ability::PASSIVITY)
    );
}

#[test]
fn stratagem_formation_control_effects_use_player_relationships() {
    assert_eq!(Formation::stratagem(Player::Red, Player::Red), (Ability::PASSIVITY, Ability::NONE));
    assert_eq!(
        Formation::stratagem(Player::Black, Player::Black),
        (Ability::PASSIVITY, Ability::NONE)
    );
    assert_eq!(
        Formation::stratagem(Player::Black, Player::Red),
        (Ability::PASSIVITY, Ability::PASSIVITY)
    );
}

#[test]
fn strategy_and_restraint_groups_have_active_push_pull_capabilities() {
    let pieces = [
        Piece::RED_GENERAL,
        Piece::RED_STRATAGEM,
        Piece::RED_MOMENTUM,
        Piece::RED_ADAPTATION,
        Piece::RED_WIND,
        Piece::RED_FOREST,
        Piece::RED_FIRE,
        Piece::RED_MOUNTAIN,
    ];
    let required =
        [Ability::PUSH_FRIEND, Ability::PUSH_ENEMY, Ability::PULL_FRIEND, Ability::PULL_ENEMY];

    for piece in pieces {
        for ability in required {
            assert!(piece.ability.has(ability), "{} is missing {:?}", piece, ability);
        }
    }
}

#[test]
fn strategy_and_restraint_groups_have_expected_passive_capabilities() {
    let pieces = [
        (Piece::RED_GENERAL, false, true, false, true),
        (Piece::RED_STRATAGEM, false, true, false, true),
        (Piece::RED_MOMENTUM, false, true, false, true),
        (Piece::RED_ADAPTATION, false, true, false, true),
        (Piece::RED_WIND, false, true, false, true),
        (Piece::RED_FOREST, false, true, true, false),
        (Piece::RED_FIRE, false, true, false, true),
        (Piece::RED_MOUNTAIN, true, false, false, true),
    ];

    for (piece, pushed_by_ally, pushed_by_enemy, pulled_by_ally, pulled_by_enemy) in pieces {
        assert_eq!(piece.ability.has(Ability::FRIEND_PUSH), pushed_by_ally, "{piece}");
        assert_eq!(piece.ability.has(Ability::ENEMY_PUSH), pushed_by_enemy, "{piece}");
        assert_eq!(piece.ability.has(Ability::FRIEND_PULL), pulled_by_ally, "{piece}");
        assert_eq!(piece.ability.has(Ability::ENEMY_PULL), pulled_by_enemy, "{piece}");
    }
}

#[test]
fn offense_defense_and_mobility_groups_have_capture_ability() {
    let pieces = [
        Piece::RED_SPEAR,
        Piece::RED_SHIELD,
        Piece::RED_SHELL,
        Piece::RED_MINE,
        Piece::RED_SCHOLAR,
        Piece::RED_PAWN,
        Piece::RED_HORSE,
        Piece::RED_ROOK,
    ];
    for piece in pieces {
        assert!(piece.ability.has(Ability::CAPTURE), "{} is missing CAPTURE", piece);
    }
}

#[test]
fn effective_formation_updates_are_order_independent_and_denial_wins() {
    let granting_neighbor = Neighbor { dx: 0, dy: -1, piece: Some(Piece::RED_PAWN) };
    let denying_neighbor = Neighbor { dx: 0, dy: 1, piece: Some(Piece::BLACK_PAWN) };

    let mut granted = Piece::RED_FIRE;
    granted.take_effect(&[granting_neighbor]);
    assert!(granted.ability.has(Ability::ORTHOGONAL_MOVE));

    let mut forward = Piece::RED_FIRE;
    forward.take_effect(&[granting_neighbor, denying_neighbor]);
    let mut reverse = Piece::RED_FIRE;
    reverse.take_effect(&[denying_neighbor, granting_neighbor]);

    assert_eq!(forward.ability, reverse.ability);
    assert!(!forward.ability.has(Ability::ORTHOGONAL_MOVE));
}

#[test]
fn effective_piece_keeps_its_base_abilities_without_a_covering_formation() {
    let mut board = Board::new(3, 3);
    board[(0, 0)] = Some(Piece::RED_ROOK);
    board[(2, 2)] = Some(Piece::RED_PAWN);

    assert_eq!(board.effective((2, 2)).expect("uncovered piece").ability, Piece::RED_PAWN.ability);
}
