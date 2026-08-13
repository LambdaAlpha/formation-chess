use formation_chess_agent::MIN_FEATURE_SCALE;
use formation_chess_agent::MIN_TERMINAL_UTILITY;
use formation_chess_agent::MinConfig;
use formation_chess_agent::MinEvaluation;
use formation_chess_agent::MinEvaluator;
use formation_chess_core::ability::Ability;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

fn evaluator() -> MinEvaluator {
    MinEvaluator::new(&MinConfig::best()).expect("best evaluator config")
}

fn game(player: Player, pieces: &[((u8, u8), Piece)], result: GameResult) -> Game {
    let mut board = Board::new(9, 10);
    for &(position, piece) in pieces {
        board[position] = Some(piece);
    }
    Game::new(GameConfig { player, board, red_pool: Vec::new(), black_pool: Vec::new(), result })
        .expect("valid test game")
}

fn placement_game(red_positions: &[(u8, u8)]) -> Game {
    let mut red_pieces = Vec::with_capacity(red_positions.len());
    for &position in red_positions {
        red_pieces.push((position, Piece::RED_ROOK));
    }
    placement_game_with_pieces(&red_pieces)
}

fn placement_game_with_pieces(red_pieces: &[((u8, u8), Piece)]) -> Game {
    let mut board = Board::new(9, 10);
    board[(8, 9)] = Some(Piece::RED_GENERAL);
    board[(8, 0)] = Some(Piece::BLACK_GENERAL);
    for &(position, piece) in red_pieces {
        board[position] = Some(piece);
    }
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: vec![Piece::RED_SHIELD],
        black_pool: vec![Piece::BLACK_SHIELD],
        result: GameResult::Unfinished,
    })
    .expect("valid placement game")
}

fn assert_bounded(evaluation: MinEvaluation) {
    let scale = MIN_FEATURE_SCALE;
    for feature in [
        evaluation.features.vital_safety,
        evaluation.features.effective_abilities,
        evaluation.features.formation_effects,
        evaluation.features.control,
        evaluation.features.mobility,
        evaluation.features.action_effects,
        evaluation.features.material,
        evaluation.features.tempo,
        evaluation.features.interactions,
    ] {
        assert!(
            (-scale ..= scale).contains(&feature),
            "feature {feature} exceeds normalized bounds"
        );
    }
    assert!(
        evaluation.utility.abs() < i32::from(MIN_TERMINAL_UTILITY),
        "non-terminal utility must remain inside terminal bounds"
    );
}

fn swap_player(player: Player) -> Player {
    match player {
        Player::Red => Player::Black,
        Player::Black => Player::Red,
    }
}

fn swap_piece(piece: Piece) -> Piece {
    Piece::lookup(piece.name, swap_player(piece.player)).expect("opponent counterpart")
}

fn mirrored_color_swap(source: &Game) -> Game {
    let board = source.board();
    let mut swapped = Board::new(board.width(), board.height());
    for ((x, y), piece) in board.iter() {
        swapped[(x, board.height() - 1 - y)] = Some(swap_piece(piece));
    }
    Game::new(GameConfig {
        player: swap_player(source.player()),
        board: swapped,
        red_pool: source.black_pool().iter().copied().map(swap_piece).collect(),
        black_pool: source.red_pool().iter().copied().map(swap_piece).collect(),
        result: match source.result() {
            GameResult::Unfinished => GameResult::Unfinished,
            GameResult::RedWin => GameResult::BlackWin,
            GameResult::BlackWin => GameResult::RedWin,
            GameResult::Draw => GameResult::Draw,
        },
    })
    .expect("valid mirrored game")
}

#[test]
fn terminal_results_use_exact_zero_sum_utility() {
    let evaluator = evaluator();
    let red_win = game(Player::Red, &[((4, 8), Piece::RED_GENERAL)], GameResult::RedWin);
    let black_win = game(Player::Black, &[((4, 1), Piece::BLACK_GENERAL)], GameResult::BlackWin);
    let draw = game(Player::Red, &[], GameResult::Draw);

    assert_eq!(evaluator.evaluate(&red_win, Player::Red).utility, 10_000);
    assert_eq!(evaluator.evaluate(&red_win, Player::Black).utility, -10_000);
    assert_eq!(evaluator.evaluate(&black_win, Player::Red).utility, -10_000);
    assert_eq!(evaluator.evaluate(&black_win, Player::Black).utility, 10_000);
    assert_eq!(evaluator.evaluate(&draw, Player::Red).utility, 0);
    assert!(evaluator.evaluate(&draw, Player::Black).exact, "draw evaluation must be exact");
}

#[test]
fn standard_initial_placement_is_symmetric_except_for_tempo() {
    let evaluation = evaluator().evaluate(&Game::default(), Player::Red);

    assert_eq!(evaluation.phase, Phase::Place);
    assert_eq!(evaluation.features.vital_safety, 0);
    assert_eq!(evaluation.features.effective_abilities, 0);
    assert_eq!(evaluation.features.formation_effects, 0);
    assert_eq!(evaluation.features.control, 0);
    assert_eq!(evaluation.features.mobility, 0);
    assert_eq!(evaluation.features.action_effects, 0);
    assert_eq!(evaluation.features.material, 0);
    assert_eq!(evaluation.features.tempo, MIN_FEATURE_SCALE);
    assert_eq!(evaluation.features.interactions, 0);
    assert!(evaluation.utility > 0, "side-to-move tempo should favor Red");
    assert_bounded(evaluation);
}

#[test]
fn opposite_perspectives_negate_features_contributions_and_utility() {
    let position = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((8, 0), Piece::BLACK_GENERAL),
            ((2, 7), Piece::RED_STRATAGEM),
            ((0, 8), Piece::RED_ROOK),
            ((6, 1), Piece::BLACK_SHIELD),
            ((1, 6), Piece::BLACK_PAWN),
        ],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let red = evaluator.evaluate(&position, Player::Red);
    let black = evaluator.evaluate(&position, Player::Black);

    assert_eq!(red.features, black.features.negated());
    assert_eq!(red.contributions, black.contributions.negated());
    assert_eq!(red.weighted_total, -black.weighted_total);
    assert_eq!(red.utility, -black.utility);
    assert_eq!(red.weighted_total, red.contributions.total());
    assert_bounded(red);
    assert_bounded(black);
}

#[test]
fn color_swap_and_vertical_mirror_preserve_player_evaluation() {
    let original = game(
        Player::Red,
        &[
            ((7, 9), Piece::RED_GENERAL),
            ((6, 0), Piece::BLACK_GENERAL),
            ((4, 7), Piece::RED_SHIELD),
            ((3, 6), Piece::RED_PAWN),
            ((2, 2), Piece::BLACK_ROOK),
            ((1, 5), Piece::BLACK_PAWN),
        ],
        GameResult::Unfinished,
    );
    let swapped = mirrored_color_swap(&original);
    let evaluator = evaluator();
    let original_red = evaluator.evaluate(&original, Player::Red);
    let swapped_black = evaluator.evaluate(&swapped, Player::Black);

    assert_eq!(original_red.features, swapped_black.features);
    assert_eq!(original_red.contributions, swapped_black.contributions);
    assert_eq!(original_red.utility, swapped_black.utility);
}

#[test]
fn placement_evaluation_rewards_spatial_coverage() {
    let clustered = placement_game(&[(0, 5), (1, 5), (0, 6), (1, 6)]);
    let spread = placement_game(&[(0, 5), (1, 5), (7, 5), (8, 5)]);
    let evaluator = evaluator();
    let clustered = evaluator.evaluate(&clustered, Player::Red);
    let spread = evaluator.evaluate(&spread, Player::Red);

    assert!(
        spread.features.control > clustered.features.control,
        "spread placement should improve control: clustered={}, spread={}",
        clustered.features.control,
        spread.features.control,
    );
    assert!(spread.utility > clustered.utility, "spread placement should improve utility");
}

#[test]
fn placement_evaluation_anchors_single_piece_away_from_edge() {
    let edge = placement_game(&[(0, 5)]);
    let center = placement_game(&[(4, 5)]);
    let evaluator = evaluator();
    let edge = evaluator.evaluate(&edge, Player::Red);
    let center = evaluator.evaluate(&center, Player::Red);

    assert!(
        center.features.control > edge.features.control,
        "center anchor should improve control: edge={}, center={}",
        edge.features.control,
        center.features.control,
    );
}

#[test]
fn placement_evaluation_rewards_balanced_column_loads() {
    let clustered = placement_game(&[(0, 5), (0, 6), (0, 7), (1, 5), (1, 6), (1, 7)]);
    let balanced = placement_game(&[(0, 5), (1, 5), (3, 5), (4, 5), (6, 5), (7, 5)]);
    let evaluator = evaluator();
    let clustered = evaluator.evaluate(&clustered, Player::Red);
    let balanced = evaluator.evaluate(&balanced, Player::Red);

    assert!(
        balanced.features.control > clustered.features.control,
        "balanced columns should improve control: clustered={}, balanced={}",
        clustered.features.control,
        balanced.features.control,
    );
}

#[test]
fn placement_evaluation_penalizes_dense_connected_components() {
    let dense = placement_game(&[(0, 5), (1, 5), (2, 5), (0, 6), (1, 6), (2, 6), (0, 7), (1, 7)]);
    let distributed =
        placement_game(&[(0, 5), (2, 5), (4, 5), (6, 5), (8, 5), (1, 7), (4, 7), (7, 7)]);
    let evaluator = evaluator();
    let dense = evaluator.evaluate(&dense, Player::Red);
    let distributed = evaluator.evaluate(&distributed, Player::Red);

    assert!(
        distributed.features.control > dense.features.control,
        "distributed placement should reduce component concentration: dense={}, distributed={}",
        dense.features.control,
        distributed.features.control,
    );
    assert!(distributed.utility > dense.utility);
}

#[test]
fn placement_mobility_counts_open_destinations() {
    let blocked = placement_game_with_pieces(&[
        ((4, 7), Piece::RED_ROOK),
        ((4, 6), Piece::RED_SHIELD),
        ((4, 8), Piece::RED_SHIELD),
        ((3, 7), Piece::RED_SHIELD),
        ((5, 7), Piece::RED_SHIELD),
    ]);
    let open = placement_game_with_pieces(&[
        ((4, 7), Piece::RED_ROOK),
        ((3, 6), Piece::RED_SHIELD),
        ((5, 6), Piece::RED_SHIELD),
        ((3, 8), Piece::RED_SHIELD),
        ((5, 8), Piece::RED_SHIELD),
    ]);
    let evaluator = evaluator();
    let blocked = evaluator.evaluate(&blocked, Player::Red);
    let open = evaluator.evaluate(&open, Player::Red);

    assert!(
        open.features.mobility > blocked.features.mobility,
        "open rook lines should improve estimated mobility: blocked={}, open={}",
        blocked.features.mobility,
        open.features.mobility,
    );
}

#[test]
fn placement_space_is_disabled_during_movement() {
    let clustered = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((8, 0), Piece::BLACK_GENERAL),
            ((0, 5), Piece::RED_ROOK),
            ((1, 5), Piece::RED_ROOK),
            ((0, 6), Piece::RED_ROOK),
            ((1, 6), Piece::RED_ROOK),
        ],
        GameResult::Unfinished,
    );
    let spread = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((8, 0), Piece::BLACK_GENERAL),
            ((0, 5), Piece::RED_ROOK),
            ((1, 5), Piece::RED_ROOK),
            ((7, 5), Piece::RED_ROOK),
            ((8, 5), Piece::RED_ROOK),
        ],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let clustered = evaluator.evaluate(&clustered, Player::Red);
    let spread = evaluator.evaluate(&spread, Player::Red);

    assert_eq!(spread.features.control, clustered.features.control);
}
#[test]
fn material_and_open_mobility_raise_red_evaluation() {
    let baseline = game(
        Player::Red,
        &[((8, 9), Piece::RED_GENERAL), ((8, 0), Piece::BLACK_GENERAL)],
        GameResult::Unfinished,
    );
    let improved = game(
        Player::Red,
        &[((8, 9), Piece::RED_GENERAL), ((8, 0), Piece::BLACK_GENERAL), ((0, 8), Piece::RED_ROOK)],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let baseline = evaluator.evaluate(&baseline, Player::Red);
    let improved = evaluator.evaluate(&improved, Player::Red);

    assert!(
        improved.features.material > baseline.features.material,
        "extra Red material must improve material feature"
    );
    assert!(
        improved.features.effective_abilities > baseline.features.effective_abilities,
        "extra Red ability resources must improve ability feature"
    );
    assert!(
        improved.features.mobility > baseline.features.mobility,
        "open Red rook must improve mobility feature"
    );
    assert!(improved.utility > baseline.utility, "joint Red improvements must raise final utility");
}

#[test]
fn formation_feature_tracks_actual_ability_changes() {
    let separated = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((8, 0), Piece::BLACK_GENERAL),
            ((0, 9), Piece::RED_SHIELD),
            ((4, 6), Piece::RED_PAWN),
        ],
        GameResult::Unfinished,
    );
    let formed = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((8, 0), Piece::BLACK_GENERAL),
            ((4, 7), Piece::RED_SHIELD),
            ((4, 6), Piece::RED_PAWN),
        ],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let separated = evaluator.evaluate(&separated, Player::Red);
    let formed = evaluator.evaluate(&formed, Player::Red);

    assert_eq!(
        formed.features.effective_abilities, separated.features.effective_abilities,
        "formation gains must not be counted again as inherent abilities",
    );
    assert!(
        formed.features.formation_effects > separated.features.formation_effects,
        "beneficial formation must improve formation feature: separated={}, formed={}",
        separated.features.formation_effects,
        formed.features.formation_effects,
    );
    assert!(
        formed.features.interactions > separated.features.interactions,
        "usable formation ability must improve interaction feature"
    );
}

#[test]
fn immediate_vital_capture_is_reflected_in_vital_safety() {
    let blocked = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((4, 1), Piece::BLACK_GENERAL),
            ((4, 8), Piece::RED_ROOK),
            ((4, 4), Piece::RED_PAWN),
        ],
        GameResult::Unfinished,
    );
    let open = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((4, 1), Piece::BLACK_GENERAL),
            ((4, 8), Piece::RED_ROOK),
            ((0, 4), Piece::RED_PAWN),
        ],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let blocked = evaluator.evaluate(&blocked, Player::Red);
    let open = evaluator.evaluate(&open, Player::Red);

    assert!(
        open.features.vital_safety > blocked.features.vital_safety,
        "immediate leader capture must improve leader feature"
    );
}

#[test]
fn draw_ability_does_not_change_soft_evaluation() {
    let mut draw_shield = Piece::RED_SHIELD;
    draw_shield.ability.add(Ability::PEACE_TALK);
    let without_draw = game(
        Player::Red,
        &[
            ((8, 9), Piece::RED_GENERAL),
            ((5, 3), Piece::BLACK_GENERAL),
            ((3, 7), Piece::RED_SHIELD),
        ],
        GameResult::Unfinished,
    );
    let with_draw = game(
        Player::Red,
        &[((8, 9), Piece::RED_GENERAL), ((5, 3), Piece::BLACK_GENERAL), ((3, 7), draw_shield)],
        GameResult::Unfinished,
    );
    let evaluator = evaluator();
    let without_draw = evaluator.evaluate(&without_draw, Player::Red);
    let with_draw = evaluator.evaluate(&with_draw, Player::Red);

    assert_eq!(with_draw.features, without_draw.features);
    assert_eq!(with_draw.contributions, without_draw.contributions);
    assert_eq!(with_draw.utility, without_draw.utility);
}
