use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::PositionChange;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Color;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

use super::MIN_TERMINAL_UTILITY;
use super::MinConfig;
use super::MinConfigError;
use super::MinEvaluationConfig;
use super::MinFeatureWeights;

/// Absolute bound of every normalized soft feature.
pub const MIN_FEATURE_SCALE: i16 = 1_000;

const BOARD_POINT_CAPACITY: usize = 256;

/// Normalized, perspective-oriented feature groups used by Min evaluation.
///
/// Every value is in `[-MIN_FEATURE_SCALE, MIN_FEATURE_SCALE]`; a larger
/// value is always better for `MinEvaluation::perspective`.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct MinFeatureVector {
    pub vital_safety: i16,
    pub effective_abilities: i16,
    pub formation_effects: i16,
    pub control: i16,
    pub mobility: i16,
    pub action_effects: i16,
    pub white_resources: i16,
    pub material: i16,
    pub tempo: i16,
    pub interactions: i16,
}

impl MinFeatureVector {
    /// Reverse the evaluation perspective without extracting features again.
    pub fn negated(self) -> Self {
        Self {
            vital_safety: -self.vital_safety,
            effective_abilities: -self.effective_abilities,
            formation_effects: -self.formation_effects,
            control: -self.control,
            mobility: -self.mobility,
            action_effects: -self.action_effects,
            white_resources: -self.white_resources,
            material: -self.material,
            tempo: -self.tempo,
            interactions: -self.interactions,
        }
    }
}

/// Per-feature weighted products before normalization to final utility.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct MinFeatureContributions {
    pub vital_safety: i64,
    pub effective_abilities: i64,
    pub formation_effects: i64,
    pub control: i64,
    pub mobility: i64,
    pub action_effects: i64,
    pub white_resources: i64,
    pub material: i64,
    pub tempo: i64,
    pub interactions: i64,
}

impl MinFeatureContributions {
    /// Sum of all weighted feature products.
    pub fn total(self) -> i64 {
        self.vital_safety
            + self.effective_abilities
            + self.formation_effects
            + self.control
            + self.mobility
            + self.action_effects
            + self.white_resources
            + self.material
            + self.tempo
            + self.interactions
    }

    /// Reverse every contribution's perspective.
    pub fn negated(self) -> Self {
        Self {
            vital_safety: -self.vital_safety,
            effective_abilities: -self.effective_abilities,
            formation_effects: -self.formation_effects,
            control: -self.control,
            mobility: -self.mobility,
            action_effects: -self.action_effects,
            white_resources: -self.white_resources,
            material: -self.material,
            tempo: -self.tempo,
            interactions: -self.interactions,
        }
    }
}

/// Complete explainable result of one static Min evaluation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MinEvaluation {
    pub perspective: Player,
    pub phase: Phase,
    pub game_result: GameResult,
    pub exact: bool,
    pub features: MinFeatureVector,
    pub contributions: MinFeatureContributions,
    pub weight_total: u32,
    pub weighted_total: i64,
    pub utility: i32,
}

impl MinEvaluation {
    /// Utility normalized to the public agent score range `[-1.0, 1.0]`.
    pub fn score(self) -> f32 {
        self.utility as f32 / f32::from(MIN_TERMINAL_UTILITY)
    }
}

/// Deterministic fixed-point evaluator used at Min search leaves.
#[derive(Debug, Copy, Clone)]
pub struct MinEvaluator {
    config: MinEvaluationConfig,
}

impl MinEvaluator {
    /// Validate and capture the evaluation portion of a complete Min config.
    pub fn new(config: &MinConfig) -> Result<Self, MinConfigError> {
        config.validate()?;
        Ok(Self { config: config.evaluation })
    }

    /// Evaluation configuration used by this instance.
    pub fn config(self) -> MinEvaluationConfig {
        self.config
    }

    /// Evaluate a game from either player's perspective.
    ///
    /// Finished games receive exact utilities. Unfinished games use bounded
    /// normalized features, non-negative configured weights, and deterministic
    /// integer arithmetic.
    pub fn evaluate(self, game: &Game, perspective: Player) -> MinEvaluation {
        let phase = game.phase();
        let weights = self.weights(phase);
        let weight_total = weights.total();
        if let Some(utility) = terminal_utility(game.result(), perspective) {
            return MinEvaluation {
                perspective,
                phase,
                game_result: game.result(),
                exact: true,
                features: MinFeatureVector::default(),
                contributions: MinFeatureContributions::default(),
                weight_total,
                weighted_total: 0,
                utility,
            };
        }

        let analysis = analyze_position(game);
        let red_features = red_feature_vector(game, &analysis);
        let features = match perspective {
            Player::Red => red_features,
            Player::Black => red_features.negated(),
        };
        let contributions = weighted_contributions(features, weights);
        let weighted_total = contributions.total();
        let utility = normalize_utility(
            weighted_total,
            weight_total,
            self.config.non_terminal_utility_limit.get(),
        );

        MinEvaluation {
            perspective,
            phase,
            game_result: game.result(),
            exact: false,
            features,
            contributions,
            weight_total,
            weighted_total,
            utility,
        }
    }

    fn weights(self, phase: Phase) -> MinFeatureWeights {
        match phase {
            Phase::Place => self.config.placement_weights,
            Phase::Move => self.config.movement_weights,
        }
    }
}

#[derive(Default)]
struct PositionAnalysis {
    red: SideAnalysis,
    black: SideAnalysis,
}

impl PositionAnalysis {
    fn side(&self, player: Player) -> &SideAnalysis {
        match player {
            Player::Red => &self.red,
            Player::Black => &self.black,
        }
    }

    fn side_mut(&mut self, player: Player) -> &mut SideAnalysis {
        match player {
            Player::Red => &mut self.red,
            Player::Black => &mut self.black,
        }
    }
}

#[derive(Default)]
struct SideAnalysis {
    effective_ability_units: i64,
    formation_effect_units: i64,
    control_units: i64,
    material_units: i64,
    vital_control_units: i64,
    vital_resilience_units: i64,

    controlled_white: i64,
    interaction_units: i64,
    actions: ActionAnalysis,
}

struct ActionAnalysis {
    safe_actions: u32,
    winning_actions: u32,
    losing_actions: u32,
    capture_actions: u32,
    push_actions: u32,
    divide_actions: u32,
    positive_exchange_actions: u32,
    safe_movable_pieces: u32,
    safe_reachable_destinations: u32,
    vital_safe_actions: u32,
    best_exchange_units: i64,
    safe_movers: [bool; BOARD_POINT_CAPACITY],
    safe_destinations: [bool; BOARD_POINT_CAPACITY],
}

impl Default for ActionAnalysis {
    fn default() -> Self {
        Self {
            safe_actions: 0,
            winning_actions: 0,
            losing_actions: 0,
            capture_actions: 0,
            push_actions: 0,
            divide_actions: 0,
            positive_exchange_actions: 0,
            safe_movable_pieces: 0,
            safe_reachable_destinations: 0,
            vital_safe_actions: 0,
            best_exchange_units: 0,
            safe_movers: [false; BOARD_POINT_CAPACITY],
            safe_destinations: [false; BOARD_POINT_CAPACITY],
        }
    }
}

#[derive(Debug)]
struct ActionOutcome {
    game_result: GameResult,
    exchange_units: i64,
}

fn analyze_position(game: &Game) -> PositionAnalysis {
    let mut analysis = PositionAnalysis::default();
    analyze_pool(&mut analysis.red, game.red_pool());
    analyze_pool(&mut analysis.black, game.black_pool());
    analyze_board(game.board(), &mut analysis);

    if game.phase() == Phase::Move {
        analyze_player_actions(game, Player::Red, &mut analysis.red.actions);
        analyze_player_actions(game, Player::Black, &mut analysis.black.actions);
    }
    analyze_interactions(game, &mut analysis);
    analysis
}

fn analyze_pool(side: &mut SideAnalysis, pieces: &[Piece]) {
    for piece in pieces {
        side.effective_ability_units += i64::from(ability_units(piece.ability));
        side.control_units += 4;
        if piece.ability.has(Ability::VITAL) {
            side.vital_control_units += 12;
            side.vital_resilience_units += i64::from(resilience_units(piece.ability));
        } else {
            side.material_units += 1;
        }
    }
}

fn analyze_board(board: &Board, analysis: &mut PositionAnalysis) {
    for (position, piece) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        analyze_owned_piece(piece, effective, analysis);
        analyze_control(effective, analysis);
    }
}

fn analyze_owned_piece(piece: Piece, effective: Piece, analysis: &mut PositionAnalysis) {
    let owner = match piece.color {
        Color::Red => Some(Player::Red),
        Color::Black => Some(Player::Black),
        Color::White => None,
    };
    let Some(owner) = owner else {
        return;
    };

    let opponent = opponent(owner);
    let side = analysis.side_mut(owner);
    let base_units = ability_units(piece.ability);
    let effective_units = ability_units(effective.ability);
    side.effective_ability_units += i64::from(effective_units);
    side.formation_effect_units += i64::from(effective_units - base_units);

    if piece.ability.has(Ability::VITAL) {
        if effective.can_controlled_by(owner) {
            side.vital_control_units += 12;
        } else {
            side.vital_control_units -= 12;
        }
        if effective.can_controlled_by(opponent) {
            side.vital_control_units -= 16;
        }
        side.vital_resilience_units += i64::from(resilience_units(effective.ability));
    } else {
        side.material_units += 1;
    }
}

fn analyze_control(piece: Piece, analysis: &mut PositionAnalysis) {
    for player in [Player::Red, Player::Black] {
        let controlled = piece.can_controlled_by(player);
        analysis.side_mut(player).control_units += control_units(player, piece.color, controlled);
        if piece.color == Color::White && controlled {
            analysis.side_mut(player).controlled_white += 1;
        }
    }
}

fn analyze_player_actions(game: &Game, player: Player, analysis: &mut ActionAnalysis) {
    let board = game.board();
    let mut actions = Vec::new();
    for (position, piece) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        if !effective.can_controlled_by(player) {
            continue;
        }
        actions.clear();
        board.valid_moves(player, position, game.white_pool() > 0, &mut actions);
        analyze_piece_actions(board, player, position, piece, &actions, analysis);
    }
    analysis.safe_movable_pieces = count_true(&analysis.safe_movers);
    analysis.safe_reachable_destinations = count_true(&analysis.safe_destinations);
}

fn analyze_piece_actions(
    board: &Board, player: Player, position: (u8, u8), piece: Piece, actions: &[Action],
    analysis: &mut ActionAnalysis,
) {
    let from_index = board_index(board, position);
    for &action in actions {
        let move_ = action_move(action).expect("board movement list must contain movement actions");
        let outcome = analyze_action_outcome(board, player, action);
        let safe = !is_loss(outcome.game_result, player);
        let soft_usable = safe && outcome.game_result != GameResult::Draw;
        let destination_index = board_index(board, move_.to);

        record_action_result(outcome.game_result, player, analysis);

        if soft_usable {
            analysis.safe_actions += 1;
            analysis.safe_movers[from_index] = true;
            analysis.safe_destinations[destination_index] = true;
            record_action_kind(action, analysis);
            record_exchange(outcome.exchange_units, analysis);
            if piece.color == player.color() && piece.ability.has(Ability::VITAL) {
                analysis.vital_safe_actions += 1;
            }
        }
    }
}

fn record_action_kind(action: Action, analysis: &mut ActionAnalysis) {
    match action {
        Action::Capture(_) => analysis.capture_actions += 1,
        Action::Push(_) => analysis.push_actions += 1,
        Action::Divide(_) => analysis.divide_actions += 1,
        Action::Move(_) | Action::Draw(_) => {},
        Action::Place(_) | Action::Pass(_) | Action::Resign(_) => {
            unreachable!("Board::valid_moves returned a non-movement action")
        },
    }
}

fn record_action_result(game_result: GameResult, player: Player, analysis: &mut ActionAnalysis) {
    if is_win(game_result, player) {
        analysis.winning_actions += 1;
    } else if is_loss(game_result, player) {
        analysis.losing_actions += 1;
    }
}

fn record_exchange(exchange_units: i64, analysis: &mut ActionAnalysis) {
    if exchange_units > 0 {
        analysis.positive_exchange_actions += 1;
    }
    analysis.best_exchange_units = analysis.best_exchange_units.max(exchange_units);
}

fn analyze_action_outcome(board: &Board, player: Player, action: Action) -> ActionOutcome {
    match action {
        Action::Move(move_) => outcome_from_changes(
            board,
            player,
            board.try_move(move_.from, move_.to).expect("enumerated move must remain valid"),
        ),
        Action::Capture(move_) => outcome_from_changes(
            board,
            player,
            board.try_capture(move_.from, move_.to).expect("enumerated capture must remain valid"),
        ),
        Action::Push(move_) => outcome_from_changes(
            board,
            player,
            board.try_push(move_.from, move_.to).expect("enumerated push must remain valid"),
        ),
        Action::Draw(move_) => {
            board.try_draw(move_.from, move_.to).expect("enumerated draw must remain valid");
            ActionOutcome { game_result: GameResult::Draw, exchange_units: 0 }
        },
        Action::Divide(_) => {
            ActionOutcome { game_result: GameResult::Unfinished, exchange_units: 0 }
        },
        Action::Place(_) | Action::Pass(_) | Action::Resign(_) => {
            unreachable!("Board::valid_moves returned a non-movement action")
        },
    }
}

fn outcome_from_changes(
    board: &Board, player: Player, changes: Vec<PositionChange>,
) -> ActionOutcome {
    ActionOutcome {
        game_result: result_after_changes(board, &changes),
        exchange_units: exchange_units(board, &changes, player),
    }
}

fn result_after_changes(board: &Board, changes: &[PositionChange]) -> GameResult {
    let red_alive = vital_survives(board, changes, Color::Red);
    let black_alive = vital_survives(board, changes, Color::Black);
    match (red_alive, black_alive) {
        (false, false) => GameResult::Draw,
        (false, true) => GameResult::BlackWin,
        (true, false) => GameResult::RedWin,
        (true, true) => GameResult::Unfinished,
    }
}

fn vital_survives(board: &Board, changes: &[PositionChange], color: Color) -> bool {
    let mut removed = false;
    let mut added = false;
    for change in changes {
        if let Some(old) = board.get(change.at)
            && old.color == color
            && old.ability.has(Ability::VITAL)
        {
            removed = true;
        }
        if let Some(new) = change.piece
            && new.color == color
            && new.ability.has(Ability::VITAL)
        {
            added = true;
        }
    }
    added || !removed
}

fn exchange_units(board: &Board, changes: &[PositionChange], player: Player) -> i64 {
    let mut units = 0;
    for change in changes {
        let old = if let Some(piece) = board.get(change.at) {
            exchange_piece_units(piece, player)
        } else {
            0
        };
        let new =
            if let Some(piece) = change.piece { exchange_piece_units(piece, player) } else { 0 };
        units += i64::from(new - old);
    }
    units
}

fn exchange_piece_units(piece: Piece, player: Player) -> i32 {
    if piece.color == Color::White || piece.ability.has(Ability::VITAL) {
        return 0;
    }
    let magnitude = (32 + ability_units(piece.ability)).max(8);
    if piece.color == player.color() { magnitude } else { -magnitude }
}

fn analyze_interactions(game: &Game, analysis: &mut PositionAnalysis) {
    let board = game.board();
    for (position, piece) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        let formation_delta = ability_units(effective.ability) - ability_units(piece.ability);
        for player in [Player::Red, Player::Black] {
            analyze_piece_interaction(
                board,
                game.phase(),
                player,
                position,
                piece,
                effective,
                formation_delta,
                analysis.side_mut(player),
            );
        }
    }
    analysis.red.interaction_units -= i64::from(analysis.red.actions.losing_actions) * 8;
    analysis.black.interaction_units -= i64::from(analysis.black.actions.losing_actions) * 8;
}

#[expect(clippy::too_many_arguments)]
fn analyze_piece_interaction(
    board: &Board, phase: Phase, player: Player, position: (u8, u8), piece: Piece,
    effective: Piece, formation_delta: i32, side: &mut SideAnalysis,
) {
    if !effective.can_controlled_by(player) {
        if piece.color == player.color() && formation_delta > 0 {
            side.interaction_units -= i64::from(formation_delta);
        }
        return;
    }

    let index = board_index(board, position);
    let mobility_factor = match phase {
        Phase::Place => 2,
        Phase::Move if side.actions.safe_movers[index] => 4,
        Phase::Move => 1,
    };
    side.interaction_units += i64::from(active_power_units(effective.ability) * mobility_factor);

    if piece.color != player.color() {
        return;
    }
    if formation_delta > 0 {
        side.interaction_units += i64::from(formation_delta * mobility_factor / 2);
    } else {
        side.interaction_units += i64::from(formation_delta * 2);
    }
}

fn red_feature_vector(game: &Game, analysis: &PositionAnalysis) -> MinFeatureVector {
    let red = analysis.side(Player::Red);
    let black = analysis.side(Player::Black);
    MinFeatureVector {
        vital_safety: normalized_feature(vital_units(red) - vital_units(black), 48),
        effective_abilities: normalized_feature(
            red.effective_ability_units - black.effective_ability_units,
            96,
        ),
        formation_effects: normalized_feature(
            red.formation_effect_units - black.formation_effect_units,
            32,
        ),
        control: normalized_feature(red.control_units - black.control_units, 64),
        mobility: normalized_feature(
            mobility_units(game.phase(), red) - mobility_units(game.phase(), black),
            128,
        ),
        action_effects: normalized_feature(
            action_effect_units(red) - action_effect_units(black),
            128,
        ),
        white_resources: normalized_feature(
            white_resource_units(red) - white_resource_units(black),
            32,
        ),
        material: normalized_feature(red.material_units - black.material_units, 4),
        tempo: match game.player() {
            Player::Red => MIN_FEATURE_SCALE,
            Player::Black => -MIN_FEATURE_SCALE,
        },
        interactions: normalized_feature(red.interaction_units - black.interaction_units, 160),
    }
}

fn vital_units(side: &SideAnalysis) -> i64 {
    side.vital_control_units
        + side.vital_resilience_units
        + i64::from(side.actions.winning_actions) * 96
        + i64::from(side.actions.vital_safe_actions) * 4
}

fn mobility_units(phase: Phase, side: &SideAnalysis) -> i64 {
    match phase {
        Phase::Place => 0,
        Phase::Move => {
            i64::from(side.actions.safe_actions)
                + i64::from(side.actions.safe_reachable_destinations) * 2
                + i64::from(side.actions.safe_movable_pieces) * 12
        },
    }
}

fn action_effect_units(side: &SideAnalysis) -> i64 {
    side.actions.best_exchange_units * 4
        + i64::from(side.actions.positive_exchange_actions) * 6
        + i64::from(side.actions.capture_actions) * 2
        + i64::from(side.actions.push_actions)
        + i64::from(side.actions.divide_actions) * 2
}

fn white_resource_units(side: &SideAnalysis) -> i64 {
    side.controlled_white * 16 + i64::from(side.actions.divide_actions) * 2
}

fn weighted_contributions(
    features: MinFeatureVector, weights: MinFeatureWeights,
) -> MinFeatureContributions {
    MinFeatureContributions {
        vital_safety: weighted_feature(features.vital_safety, weights.vital_safety),
        effective_abilities: weighted_feature(
            features.effective_abilities,
            weights.effective_abilities,
        ),
        formation_effects: weighted_feature(features.formation_effects, weights.formation_effects),
        control: weighted_feature(features.control, weights.control),
        mobility: weighted_feature(features.mobility, weights.mobility),
        action_effects: weighted_feature(features.action_effects, weights.action_effects),
        white_resources: weighted_feature(features.white_resources, weights.white_resources),
        material: weighted_feature(features.material, weights.material),
        tempo: weighted_feature(features.tempo, weights.tempo),
        interactions: weighted_feature(features.interactions, weights.interactions),
    }
}

fn weighted_feature(feature: i16, weight: u16) -> i64 {
    i64::from(feature) * i64::from(weight)
}

fn normalize_utility(weighted_total: i64, weight_total: u32, limit: u16) -> i32 {
    let denominator = i64::from(weight_total) * i64::from(MIN_FEATURE_SCALE);
    let numerator = weighted_total * i64::from(limit);
    let rounded = symmetric_round_division(numerator, denominator);
    i32::try_from(rounded).expect("bounded Min utility must fit i32")
}

fn symmetric_round_division(numerator: i64, denominator: i64) -> i64 {
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        -((-numerator + denominator / 2) / denominator)
    }
}

fn normalized_feature(raw: i64, half_saturation: i64) -> i16 {
    if raw == 0 {
        return 0;
    }
    let scaled = raw * i64::from(MIN_FEATURE_SCALE) / (raw.abs() + half_saturation);
    i16::try_from(scaled.clamp(-i64::from(MIN_FEATURE_SCALE), i64::from(MIN_FEATURE_SCALE)))
        .expect("normalized feature must fit i16")
}

fn terminal_utility(game_result: GameResult, perspective: Player) -> Option<i32> {
    let terminal = i32::from(MIN_TERMINAL_UTILITY);
    match (game_result, perspective) {
        (GameResult::Unfinished, _) => None,
        (GameResult::Draw, _) => Some(0),
        (GameResult::RedWin, Player::Red) | (GameResult::BlackWin, Player::Black) => Some(terminal),
        (GameResult::RedWin, Player::Black) | (GameResult::BlackWin, Player::Red) => {
            Some(-terminal)
        },
    }
}

fn ability_units(ability: Ability) -> i32 {
    ability_weight(ability, Ability::PUSH_ALLY, 1)
        + ability_weight(ability, Ability::PUSH_ENEMY, 4)
        + ability_weight(ability, Ability::PUSHED_BY_ALLY, 1)
        + ability_weight(ability, Ability::PUSHED_BY_ENEMY, -3)
        + ability_weight(ability, Ability::CAPTURE_ON_PUSH_BLOCKED, 4)
        + ability_weight(ability, Ability::CAPTURED_ON_PUSH_BLOCKED, -4)
        + ability_weight(ability, Ability::PUSH_ON_CAPTURE_UNBLOCKED, -1)
        + ability_weight(ability, Ability::PUSHED_ON_CAPTURE_UNBLOCKED, 3)
        + ability_weight(ability, Ability::CAPTURE, 5)
        + ability_weight(ability, Ability::CAPTURED, -5)
        + ability_weight(ability, Ability::CAPTURE_ON_CAPTURED, 5)
        + ability_weight(ability, Ability::CAPTURED_ON_CAPTURE, -1)
        + ability_weight(ability, Ability::ANY_DISTANCE, 5)
        + ability_weight(ability, Ability::DIRECTION_CROSS, 2)
        + ability_weight(ability, Ability::DIRECTION_DIAGONAL, 2)
        + ability_weight(ability, Ability::DIRECTION_SHAPE_L, 3)
        + ability_weight(ability, Ability::DIVIDE, 4)
}

fn active_power_units(ability: Ability) -> i32 {
    ability_weight(ability, Ability::PUSH_ALLY, 1)
        + ability_weight(ability, Ability::PUSH_ENEMY, 4)
        + ability_weight(ability, Ability::CAPTURE_ON_PUSH_BLOCKED, 4)
        + ability_weight(ability, Ability::CAPTURE, 5)
        + ability_weight(ability, Ability::CAPTURED_ON_CAPTURE, 1)
        + ability_weight(ability, Ability::ANY_DISTANCE, 5)
        + ability_weight(ability, Ability::DIRECTION_CROSS, 2)
        + ability_weight(ability, Ability::DIRECTION_DIAGONAL, 2)
        + ability_weight(ability, Ability::DIRECTION_SHAPE_L, 3)
        + ability_weight(ability, Ability::DIVIDE, 4)
}

fn resilience_units(ability: Ability) -> i32 {
    ability_weight(ability, Ability::PUSHED_BY_ENEMY, -4)
        + ability_weight(ability, Ability::CAPTURED_ON_PUSH_BLOCKED, -5)
        + ability_weight(ability, Ability::PUSHED_ON_CAPTURE_UNBLOCKED, 4)
        + if ability.has(Ability::CAPTURED) { -6 } else { 6 }
        + ability_weight(ability, Ability::CAPTURE_ON_CAPTURED, 6)
}

fn ability_weight(abilities: Ability, ability: Ability, weight: i32) -> i32 {
    if abilities.has(ability) { weight } else { 0 }
}

fn control_units(player: Player, color: Color, controlled: bool) -> i64 {
    if controlled {
        return match color {
            Color::White => 3,
            color if color == player.color() => 4,
            Color::Red | Color::Black => 5,
        };
    }
    if color == player.color() { -3 } else { 0 }
}

fn action_move(action: Action) -> Option<formation_chess_core::action::Move> {
    match action {
        Action::Move(move_)
        | Action::Capture(move_)
        | Action::Push(move_)
        | Action::Draw(move_)
        | Action::Divide(move_) => Some(move_),
        Action::Place(_) | Action::Pass(_) | Action::Resign(_) => None,
    }
}

fn is_win(result: GameResult, player: Player) -> bool {
    matches!(
        (result, player),
        (GameResult::RedWin, Player::Red) | (GameResult::BlackWin, Player::Black)
    )
}

fn is_loss(result: GameResult, player: Player) -> bool {
    matches!(
        (result, player),
        (GameResult::RedWin, Player::Black) | (GameResult::BlackWin, Player::Red)
    )
}

fn opponent(player: Player) -> Player {
    match player {
        Player::Red => Player::Black,
        Player::Black => Player::Red,
    }
}

fn board_index(board: &Board, (x, y): (u8, u8)) -> usize {
    usize::from(y) * usize::from(board.width()) + usize::from(x)
}

fn count_true(values: &[bool; BOARD_POINT_CAPACITY]) -> u32 {
    values.iter().filter(|&&value| value).count() as u32
}

#[cfg(test)]
mod tests {
    use formation_chess_core::action::Action;
    use formation_chess_core::action::GameResult;
    use formation_chess_core::game::Game;

    use super::MinFeatureVector;
    use super::analyze_action_outcome;
    use super::weighted_contributions;
    use crate::ActionSelector;
    use crate::MinFeatureWeights;
    use crate::RandomAgent;
    use crate::legal_movement_actions;
    use crate::play_agent_turn;

    #[test]
    fn feature_groups_use_their_matching_weights() {
        let features = MinFeatureVector {
            vital_safety: 1,
            effective_abilities: 2,
            formation_effects: 3,
            control: 4,
            mobility: 5,
            action_effects: 6,
            white_resources: 7,
            material: 8,
            tempo: 9,
            interactions: 10,
        };
        let weights = MinFeatureWeights {
            vital_safety: 11,
            effective_abilities: 12,
            formation_effects: 13,
            control: 14,
            mobility: 15,
            action_effects: 16,
            white_resources: 17,
            material: 18,
            tempo: 19,
            interactions: 20,
        };
        let contributions = weighted_contributions(features, weights);

        assert_eq!(contributions.vital_safety, 11);
        assert_eq!(contributions.effective_abilities, 24);
        assert_eq!(contributions.formation_effects, 39);
        assert_eq!(contributions.control, 56);
        assert_eq!(contributions.mobility, 75);
        assert_eq!(contributions.action_effects, 96);
        assert_eq!(contributions.white_resources, 119);
        assert_eq!(contributions.material, 144);
        assert_eq!(contributions.tempo, 171);
        assert_eq!(contributions.interactions, 200);
    }

    #[test]
    fn predicted_action_results_match_core_reactions() {
        let mut game = Game::default();
        let mut random = RandomAgent::with_seed(20260802);
        let mut selector = ActionSelector::default();
        for _ in 0 .. 32 {
            play_agent_turn(&mut game, &mut random, &mut selector)
                .expect("complete deterministic placement");
        }

        for _ in 0 .. 32 {
            for action in legal_movement_actions(&game) {
                if matches!(action, Action::Pass(_)) {
                    continue;
                }
                let expected = game.try_action(action).expect("enumerated action must be legal");
                let predicted = analyze_action_outcome(game.board(), game.player(), action);
                assert_eq!(
                    predicted.game_result, expected.game_result,
                    "predicted result differs for {action:?}"
                );
            }
            if game.result() != GameResult::Unfinished {
                break;
            }
            play_agent_turn(&mut game, &mut random, &mut selector)
                .expect("advance deterministic movement");
        }
    }
}
