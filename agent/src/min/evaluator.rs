use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::PositionChange;
use formation_chess_core::action::PositionChanges;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

use super::MIN_TERMINAL_UTILITY;
use super::MinConfig;
use super::MinConfigError;
use super::MinEvaluationConfig;
use super::MinFeatureWeights;
use super::outcome::ResolvedActionKind;
use super::outcome::action_move;
use super::outcome::resolved_action_kind;
use super::outcome::result_after_changes;

/// Absolute bound of every normalized soft feature.
pub const MIN_FEATURE_SCALE: i16 = 1_000;

const BOARD_POINT_CAPACITY: usize = 256;
const STATIC_EXCHANGE_REPLY_DEPTH: u8 = 2;
const STATIC_EXCHANGE_TERMINAL_UNITS: i64 = 512;

const PLACEMENT_HORIZONTAL_SPAN_WEIGHT: i64 = 4;
const PLACEMENT_VERTICAL_SPAN_WEIGHT: i64 = 2;
const PLACEMENT_COLUMN_COVERAGE_WEIGHT: i64 = 8;
const PLACEMENT_ROW_COVERAGE_WEIGHT: i64 = 4;
const PLACEMENT_HORIZONTAL_GAP_WEIGHT: i64 = 8;
const PLACEMENT_VERTICAL_GAP_WEIGHT: i64 = 4;
const PLACEMENT_COLUMN_BALANCE_WEIGHT: i64 = 8;
const PLACEMENT_ROW_BALANCE_WEIGHT: i64 = 4;
const PLACEMENT_COMPONENT_CONCENTRATION_WEIGHT: i64 = 8;
const PLACEMENT_CROWDING_WEIGHT: i64 = 4;
const PLACEMENT_LONG_RANGE_BLOCK_WEIGHT: i64 = 6;
const PLACEMENT_FORMATION_EDGE_COST: i64 = 2;

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
    placement_space_units: i64,
    material_units: i64,
    vital_control_units: i64,
    vital_resilience_units: i64,

    interaction_units: i64,
    actions: ActionAnalysis,
}

struct ActionAnalysis {
    quiet_move_actions: u32,
    winning_actions: u32,
    capture_actions: u32,
    push_actions: u32,
    pull_actions: u32,
    positive_exchange_actions: u32,
    safe_movable_pieces: u32,
    safe_reachable_destinations: u32,
    vital_safe_actions: u32,
    best_exchange_units: i64,
    safe_movers: [bool; BOARD_POINT_CAPACITY],
    safe_destinations: [bool; BOARD_POINT_CAPACITY],
    quiet_moves_by_origin: [u16; BOARD_POINT_CAPACITY],
}

impl Default for ActionAnalysis {
    fn default() -> Self {
        Self {
            quiet_move_actions: 0,
            winning_actions: 0,
            capture_actions: 0,
            push_actions: 0,
            pull_actions: 0,
            positive_exchange_actions: 0,
            safe_movable_pieces: 0,
            safe_reachable_destinations: 0,
            vital_safe_actions: 0,
            best_exchange_units: 0,
            safe_movers: [false; BOARD_POINT_CAPACITY],
            safe_destinations: [false; BOARD_POINT_CAPACITY],
            quiet_moves_by_origin: [0; BOARD_POINT_CAPACITY],
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct PlacementTopology {
    adjacent_edges: i64,
    max_component: usize,
    crowding_excess: i64,
    long_range_blocks: i64,
}

#[derive(Debug)]
struct ActionOutcome {
    changes: PositionChanges,
    game_result: GameResult,
    exchange_units: i64,
    kind: ResolvedActionKind,
}

fn analyze_position(game: &Game) -> PositionAnalysis {
    let mut analysis = PositionAnalysis::default();
    analyze_pool(&mut analysis.red, game.red_pool());
    analyze_pool(&mut analysis.black, game.black_pool());
    analyze_board(game.board(), &mut analysis);
    match game.phase() {
        Phase::Place => {
            analyze_placement_space(game.board(), &mut analysis);
            analyze_empty_mobility(game.board(), Player::Red, &mut analysis.red.actions);
            analyze_empty_mobility(game.board(), Player::Black, &mut analysis.black.actions);
        },
        Phase::Move => {
            analyze_player_actions(game, Player::Red, &mut analysis.red.actions);
            analyze_player_actions(game, Player::Black, &mut analysis.black.actions);
        },
    }
    analyze_interactions(game.board(), &mut analysis);
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

fn analyze_placement_space(board: &Board, analysis: &mut PositionAnalysis) {
    for player in [Player::Red, Player::Black] {
        let (space_units, topology) = placement_space_units(board, player);
        let side = analysis.side_mut(player);
        side.placement_space_units = space_units;
        side.formation_effect_units -= topology.adjacent_edges * PLACEMENT_FORMATION_EDGE_COST;
    }
}

fn placement_space_units(board: &Board, player: Player) -> (i64, PlacementTopology) {
    let mut occupied_columns = [false; BOARD_POINT_CAPACITY];
    let mut occupied_rows = [false; BOARD_POINT_CAPACITY];
    let mut column_loads = [0u16; BOARD_POINT_CAPACITY];
    let mut row_loads = [0u16; BOARD_POINT_CAPACITY];
    let mut count = 0usize;
    let mut min_x = board.width();
    let mut max_x = 0u8;
    let mut min_y = board.height();
    let mut max_y = 0u8;
    for ((x, y), piece) in board.iter() {
        if piece.player != player {
            continue;
        }
        let x_index = usize::from(x);
        let y_index = usize::from(y);
        occupied_columns[x_index] = true;
        occupied_rows[y_index] = true;
        column_loads[x_index] += 1;
        row_loads[y_index] += 1;
        count += 1;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if count == 0 {
        return (0, PlacementTopology::default());
    }

    let topology = placement_topology(board, player);
    let columns = i64::from(count_true(&occupied_columns));
    let rows = i64::from(count_true(&occupied_rows));
    let horizontal_gap = empty_gap_coverage(&occupied_columns, 0, board.width());
    let vertical_gap = empty_gap_coverage(&occupied_rows, 0, board.height());
    let column_balance = load_balance_units(&column_loads, count);
    let row_balance = load_balance_units(&row_loads, count);
    let count_units = i64::try_from(count).expect("placement count must fit i64");
    let max_component =
        i64::try_from(topology.max_component).expect("placement component must fit i64");
    let component_concentration = (max_component * max_component / count_units - 1).max(0);
    let units = i64::from(max_x - min_x) * PLACEMENT_HORIZONTAL_SPAN_WEIGHT
        + i64::from(max_y - min_y) * PLACEMENT_VERTICAL_SPAN_WEIGHT
        + (columns - 1) * PLACEMENT_COLUMN_COVERAGE_WEIGHT
        + (rows - 1) * PLACEMENT_ROW_COVERAGE_WEIGHT
        + horizontal_gap * PLACEMENT_HORIZONTAL_GAP_WEIGHT
        + vertical_gap * PLACEMENT_VERTICAL_GAP_WEIGHT
        + column_balance * PLACEMENT_COLUMN_BALANCE_WEIGHT
        + row_balance * PLACEMENT_ROW_BALANCE_WEIGHT
        - component_concentration * PLACEMENT_COMPONENT_CONCENTRATION_WEIGHT
        - topology.crowding_excess * PLACEMENT_CROWDING_WEIGHT
        - topology.long_range_blocks * PLACEMENT_LONG_RANGE_BLOCK_WEIGHT;
    (units, topology)
}

fn placement_topology(board: &Board, player: Player) -> PlacementTopology {
    let mut occupied = [false; BOARD_POINT_CAPACITY];
    for (position, piece) in board.iter() {
        if piece.player == player {
            occupied[board_index(board, position)] = true;
        }
    }

    let mut topology = PlacementTopology::default();
    let mut visited = [false; BOARD_POINT_CAPACITY];
    for y in 0 .. board.height() {
        for x in 0 .. board.width() {
            let position = (x, y);
            let index = board_index(board, position);
            if !occupied[index] {
                continue;
            }

            let mut neighbor_count = 0i64;
            for dy in -1 ..= 1 {
                for dx in -1 ..= 1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let Some(neighbor) = offset_position(board, position, dx, dy) else {
                        continue;
                    };
                    if occupied[board_index(board, neighbor)] {
                        neighbor_count += 1;
                    }
                }
            }
            let crowding = (neighbor_count - 3).max(0);
            topology.crowding_excess += crowding * crowding;

            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let Some(neighbor) = offset_position(board, position, dx, dy) else {
                    continue;
                };
                if occupied[board_index(board, neighbor)] {
                    topology.adjacent_edges += 1;
                }
            }

            if visited[index] {
                continue;
            }
            let component = connected_component_size(board, position, &occupied, &mut visited);
            topology.max_component = topology.max_component.max(component);
        }
    }
    topology.long_range_blocks = long_range_ally_blocks(board, player);
    topology
}

fn connected_component_size(
    board: &Board, start: (u8, u8), occupied: &[bool; BOARD_POINT_CAPACITY],
    visited: &mut [bool; BOARD_POINT_CAPACITY],
) -> usize {
    let mut stack = vec![start];
    visited[board_index(board, start)] = true;
    let mut size = 0usize;
    while let Some(position) = stack.pop() {
        size += 1;
        for dy in -1 ..= 1 {
            for dx in -1 ..= 1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(neighbor) = offset_position(board, position, dx, dy) else {
                    continue;
                };
                let index = board_index(board, neighbor);
                if !occupied[index] || visited[index] {
                    continue;
                }
                visited[index] = true;
                stack.push(neighbor);
            }
        }
    }
    size
}

fn long_range_ally_blocks(board: &Board, player: Player) -> i64 {
    let mut blocks = 0i64;
    for (position, piece) in board.iter() {
        if piece.player != player {
            continue;
        }
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        if !effective.ability.has(Ability::ANY_DISTANCE) {
            continue;
        }
        if effective.ability.has(Ability::DIRECTION_CROSS) {
            blocks +=
                blocked_directions(board, player, position, &[(0, -1), (0, 1), (-1, 0), (1, 0)]);
        }
        if effective.ability.has(Ability::DIRECTION_DIAGONAL) {
            blocks +=
                blocked_directions(board, player, position, &[(-1, -1), (1, -1), (-1, 1), (1, 1)]);
        }
        if effective.ability.has(Ability::DIRECTION_SHAPE_L) {
            blocks += blocked_directions(board, player, position, &[
                (1, 2),
                (2, 1),
                (-1, 2),
                (-2, 1),
                (1, -2),
                (2, -1),
                (-1, -2),
                (-2, -1),
            ]);
        }
    }
    blocks
}

fn blocked_directions(
    board: &Board, player: Player, position: (u8, u8), directions: &[(i8, i8)],
) -> i64 {
    let mut blocks = 0i64;
    for &(dx, dy) in directions {
        let Some(target) = offset_position(board, position, dx, dy) else {
            continue;
        };
        let Some(piece) = board.get(target) else {
            continue;
        };
        if piece.player == player {
            blocks += 1;
        }
    }
    blocks
}

fn offset_position(board: &Board, (x, y): (u8, u8), dx: i8, dy: i8) -> Option<(u8, u8)> {
    let target_x = x as i8 + dx;
    let target_y = y as i8 + dy;
    if target_x < 0 || target_y < 0 {
        return None;
    }
    let target = (target_x as u8, target_y as u8);
    if !board.in_bounds(target) {
        return None;
    }
    Some(target)
}

fn empty_gap_coverage(occupied: &[bool; BOARD_POINT_CAPACITY], start: u8, end: u8) -> i64 {
    if start >= end {
        return 0;
    }

    let mut has_occupied = false;
    let mut largest_gap = 0usize;
    let mut current_gap = 0usize;
    for &is_occupied in &occupied[usize::from(start) .. usize::from(end)] {
        if is_occupied {
            has_occupied = true;
            largest_gap = largest_gap.max(current_gap);
            current_gap = 0;
            continue;
        }
        current_gap += 1;
    }
    largest_gap = largest_gap.max(current_gap);
    if !has_occupied {
        return 0;
    }

    let span = usize::from(end - start);
    i64::try_from(span - 1 - largest_gap).expect("placement gap coverage must fit i64")
}

fn load_balance_units(loads: &[u16; BOARD_POINT_CAPACITY], count: usize) -> i64 {
    if count <= 1 {
        return 0;
    }

    let count = i64::try_from(count).expect("placement count must fit i64");
    let total_pairs = count * (count - 1) / 2;
    let mut same_line_pairs = 0;
    for &load in loads {
        let load = i64::from(load);
        same_line_pairs += load * (load - 1) / 2;
    }
    total_pairs - same_line_pairs
}

fn analyze_owned_piece(piece: Piece, effective: Piece, analysis: &mut PositionAnalysis) {
    let owner = piece.player;
    let opponent = opponent(owner);
    let side = analysis.side_mut(owner);
    let base_units = ability_units(piece.ability);
    let effective_units = ability_units(effective.ability);
    side.effective_ability_units += i64::from(base_units);
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
        analysis.side_mut(player).control_units += control_units(player, piece.player, controlled);
    }
}

fn analyze_empty_mobility(board: &Board, player: Player, analysis: &mut ActionAnalysis) {
    let mut actions = Vec::new();
    for (position, _) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        if !effective.can_controlled_by(player) {
            continue;
        }
        actions.clear();
        board.valid_moves(player, position, &mut actions);
        for action in &actions {
            let Action::Move(move_) = action else {
                continue;
            };
            record_quiet_move(board, position, move_.to, analysis);
        }
    }
    analysis.safe_movable_pieces = count_true(&analysis.safe_movers);
    analysis.safe_reachable_destinations = count_true(&analysis.safe_destinations);
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
        board.valid_moves(player, position, &mut actions);
        analyze_piece_actions(board, player, position, piece, &actions, analysis);
    }
    analysis.safe_movable_pieces = count_true(&analysis.safe_movers);
    analysis.safe_reachable_destinations = count_true(&analysis.safe_destinations);
}

fn analyze_piece_actions(
    board: &Board, player: Player, position: (u8, u8), piece: Piece, actions: &[Action],
    analysis: &mut ActionAnalysis,
) {
    for &action in actions {
        let outcome = analyze_action_outcome(board, player, action);
        record_action_result(outcome.game_result, player, analysis);
        let usable =
            !is_loss(outcome.game_result, player) && outcome.game_result != GameResult::Draw;
        if !usable {
            continue;
        }

        let exchange_units = reply_adjusted_exchange_units(board, player, action, &outcome);
        record_exchange(exchange_units, analysis);
        if exchange_units < 0 && !is_win(outcome.game_result, player) {
            continue;
        }
        if piece.player == player && piece.ability.has(Ability::VITAL) {
            analysis.vital_safe_actions += 1;
        }
        record_action_kind(outcome.kind, analysis);
        if outcome.kind != ResolvedActionKind::QuietMove {
            continue;
        }

        let Some(move_) = action_move(action) else {
            unreachable!("resolved quiet move must have movement coordinates");
        };
        record_quiet_move(board, position, move_.to, analysis);
    }
}

fn record_quiet_move(board: &Board, from: (u8, u8), to: (u8, u8), analysis: &mut ActionAnalysis) {
    let from_index = board_index(board, from);
    let destination_index = board_index(board, to);
    analysis.quiet_move_actions += 1;
    analysis.safe_movers[from_index] = true;
    analysis.safe_destinations[destination_index] = true;
    analysis.quiet_moves_by_origin[from_index] += 1;
}

fn record_action_kind(kind: ResolvedActionKind, analysis: &mut ActionAnalysis) {
    match kind {
        ResolvedActionKind::Capture => analysis.capture_actions += 1,
        ResolvedActionKind::Push => analysis.push_actions += 1,
        ResolvedActionKind::Pull => analysis.pull_actions += 1,
        ResolvedActionKind::QuietMove | ResolvedActionKind::Other => {},
    }
}

fn record_action_result(game_result: GameResult, player: Player, analysis: &mut ActionAnalysis) {
    if is_win(game_result, player) {
        analysis.winning_actions += 1;
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
            action,
            board.try_move(move_.from, move_.to).expect("enumerated move must remain valid"),
        ),
        Action::Capture(move_) => outcome_from_changes(
            board,
            player,
            action,
            board.try_capture(move_.from, move_.to).expect("enumerated capture must remain valid"),
        ),
        Action::Push(move_) => outcome_from_changes(
            board,
            player,
            action,
            board.try_push(move_.from, move_.to).expect("enumerated push must remain valid"),
        ),
        Action::Pull(move_) => outcome_from_changes(
            board,
            player,
            action,
            board.try_pull(move_.from, move_.to).expect("enumerated pull must remain valid"),
        ),
        Action::Draw(move_) => {
            let changes =
                board.try_draw(move_.from, move_.to).expect("enumerated draw must remain valid");
            ActionOutcome {
                changes,
                game_result: GameResult::Draw,
                exchange_units: exchange_units(changes.as_slice(), player),
                kind: ResolvedActionKind::Other,
            }
        },
        Action::Resign(x, y) => {
            let piece = board
                .effective((x, y))
                .expect("enumerated resignation must retain its vital piece");
            let game_result = match piece.player {
                Player::Red => GameResult::BlackWin,
                Player::Black => GameResult::RedWin,
            };
            ActionOutcome {
                changes: PositionChanges::empty(),
                game_result,
                exchange_units: 0,
                kind: ResolvedActionKind::Other,
            }
        },
        Action::Place(_) => {
            unreachable!("Board::valid_moves returned a non-board action")
        },
    }
}

fn outcome_from_changes(
    board: &Board, player: Player, action: Action, changes: PositionChanges,
) -> ActionOutcome {
    ActionOutcome {
        changes,
        game_result: result_after_changes(changes.as_slice()),
        exchange_units: exchange_units(changes.as_slice(), player),
        kind: resolved_action_kind(board, action, changes),
    }
}

fn reply_adjusted_exchange_units(
    board: &Board, player: Player, action: Action, outcome: &ActionOutcome,
) -> i64 {
    if outcome.game_result != GameResult::Unfinished || outcome.exchange_units <= 0 {
        return outcome.exchange_units;
    }
    let Some(move_) = action_move(action) else {
        return outcome.exchange_units;
    };

    let mut next_board = board.clone();
    apply_position_changes(&mut next_board, outcome.changes.as_slice());
    outcome.exchange_units
        + static_exchange_replies(
            &next_board,
            player,
            opponent(player),
            move_.to,
            STATIC_EXCHANGE_REPLY_DEPTH,
        )
}

fn static_exchange_replies(
    board: &Board, perspective: Player, player: Player, target: (u8, u8), depth: u8,
) -> i64 {
    if depth == 0 || board.get(target).is_none() {
        return 0;
    }

    let maximizing = player == perspective;
    let mut best_units = 0;
    for (from, _) in board.iter() {
        let effective = board.effective(from).expect("occupied point must have an effective piece");
        if !effective.can_controlled_by(player) {
            continue;
        }

        for action in
            [Action::Capture(Move { from, to: target }), Action::Push(Move { from, to: target })]
        {
            let Some(outcome) = try_static_exchange_action(board, player, action) else {
                continue;
            };
            if is_loss(outcome.game_result, player) || outcome.game_result == GameResult::Draw {
                continue;
            }

            let exchange_units = static_exchange_action_units(&outcome, perspective);
            if exchange_units == 0 {
                continue;
            }
            let continuation_units = if outcome.game_result == GameResult::Unfinished {
                let mut next_board = board.clone();
                apply_position_changes(&mut next_board, outcome.changes.as_slice());
                static_exchange_replies(
                    &next_board,
                    perspective,
                    opponent(player),
                    target,
                    depth - 1,
                )
            } else {
                0
            };
            let units = exchange_units + continuation_units;
            if maximizing {
                best_units = best_units.max(units);
            } else {
                best_units = best_units.min(units);
            }
        }
    }
    best_units
}

fn try_static_exchange_action(
    board: &Board, player: Player, action: Action,
) -> Option<ActionOutcome> {
    let changes = match action {
        Action::Capture(move_) => board.try_capture(move_.from, move_.to).ok()?,
        Action::Push(move_) => board.try_push(move_.from, move_.to).ok()?,
        _ => unreachable!("static exchange only probes captures and pushes"),
    };
    Some(outcome_from_changes(board, player, action, changes))
}

fn static_exchange_action_units(outcome: &ActionOutcome, perspective: Player) -> i64 {
    if is_win(outcome.game_result, perspective) {
        return STATIC_EXCHANGE_TERMINAL_UNITS;
    }
    if is_loss(outcome.game_result, perspective) {
        return -STATIC_EXCHANGE_TERMINAL_UNITS;
    }
    exchange_units(outcome.changes.as_slice(), perspective)
}

fn apply_position_changes(board: &mut Board, changes: &[PositionChange]) {
    for change in changes {
        board[change.at] = change.new;
    }
}

fn exchange_units(changes: &[PositionChange], player: Player) -> i64 {
    let mut units = 0;
    for change in changes {
        let old =
            if let Some(piece) = change.old { exchange_piece_units(piece, player) } else { 0 };
        let new =
            if let Some(piece) = change.new { exchange_piece_units(piece, player) } else { 0 };
        units += i64::from(new - old);
    }
    units
}

fn exchange_piece_units(piece: Piece, player: Player) -> i32 {
    if piece.ability.has(Ability::VITAL) {
        return 0;
    }
    let magnitude = (32 + ability_units(piece.ability)).max(8);
    if piece.player == player { magnitude } else { -magnitude }
}

fn analyze_interactions(board: &Board, analysis: &mut PositionAnalysis) {
    for (position, _) in board.iter() {
        let effective =
            board.effective(position).expect("occupied point must have an effective piece");
        for player in [Player::Red, Player::Black] {
            analyze_piece_interaction(
                board,
                player,
                position,
                effective,
                analysis.side_mut(player),
            );
        }
    }
}

fn analyze_piece_interaction(
    board: &Board, player: Player, position: (u8, u8), effective: Piece, side: &mut SideAnalysis,
) {
    if !effective.can_controlled_by(player) {
        return;
    }

    let index = board_index(board, position);
    let quiet_moves = side.actions.quiet_moves_by_origin[index].min(4);
    if quiet_moves == 0 {
        return;
    }
    let mobility_factor = i32::from(quiet_moves);
    side.interaction_units += i64::from(active_power_units(effective.ability) * mobility_factor);
}

fn red_feature_vector(game: &Game, analysis: &PositionAnalysis) -> MinFeatureVector {
    let red = analysis.side(Player::Red);
    let black = analysis.side(Player::Black);
    let control_units = red.control_units - black.control_units
        + match game.phase() {
            Phase::Place => red.placement_space_units - black.placement_space_units,
            Phase::Move => 0,
        };
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
        control: normalized_feature(control_units, 64),
        mobility: normalized_feature(mobility_units(red) - mobility_units(black), 128),
        action_effects: normalized_feature(
            action_effect_units(red) - action_effect_units(black),
            128,
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

fn mobility_units(side: &SideAnalysis) -> i64 {
    i64::from(side.actions.quiet_move_actions)
        + i64::from(side.actions.safe_reachable_destinations) * 2
        + i64::from(side.actions.safe_movable_pieces) * 12
}

fn action_effect_units(side: &SideAnalysis) -> i64 {
    side.actions.best_exchange_units * 4
        + i64::from(side.actions.positive_exchange_actions) * 6
        + i64::from(side.actions.capture_actions) * 2
        + i64::from(side.actions.push_actions)
        + i64::from(side.actions.pull_actions)
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
        + ability_weight(ability, Ability::PULL_ALLY, 1)
        + ability_weight(ability, Ability::PULL_ENEMY, 4)
        + ability_weight(ability, Ability::PULLED_BY_ALLY, 1)
        + ability_weight(ability, Ability::PULLED_BY_ENEMY, -3)
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
}

fn active_power_units(ability: Ability) -> i32 {
    ability_weight(ability, Ability::PUSH_ALLY, 1)
        + ability_weight(ability, Ability::PUSH_ENEMY, 4)
        + ability_weight(ability, Ability::PULL_ALLY, 1)
        + ability_weight(ability, Ability::PULL_ENEMY, 4)
        + ability_weight(ability, Ability::CAPTURE_ON_PUSH_BLOCKED, 4)
        + ability_weight(ability, Ability::CAPTURE, 5)
        + ability_weight(ability, Ability::CAPTURED_ON_CAPTURE, 1)
        + ability_weight(ability, Ability::ANY_DISTANCE, 5)
        + ability_weight(ability, Ability::DIRECTION_CROSS, 2)
        + ability_weight(ability, Ability::DIRECTION_DIAGONAL, 2)
        + ability_weight(ability, Ability::DIRECTION_SHAPE_L, 3)
}

fn resilience_units(ability: Ability) -> i32 {
    ability_weight(ability, Ability::PUSHED_BY_ENEMY, -4)
        + ability_weight(ability, Ability::PULLED_BY_ENEMY, -4)
        + ability_weight(ability, Ability::CAPTURED_ON_PUSH_BLOCKED, -5)
        + ability_weight(ability, Ability::PUSHED_ON_CAPTURE_UNBLOCKED, 4)
        + if ability.has(Ability::CAPTURED) { -6 } else { 6 }
        + ability_weight(ability, Ability::CAPTURE_ON_CAPTURED, 6)
}

fn ability_weight(abilities: Ability, ability: Ability, weight: i32) -> i32 {
    if abilities.has(ability) { weight } else { 0 }
}

fn control_units(player: Player, owner: Player, controlled: bool) -> i64 {
    if controlled {
        if owner == player {
            return 4;
        }
        return 5;
    }
    if owner == player { -3 } else { 0 }
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
    use formation_chess_core::ability::Ability;
    use formation_chess_core::action::Action;
    use formation_chess_core::action::GameResult;
    use formation_chess_core::action::Move;
    use formation_chess_core::board::Board;
    use formation_chess_core::game::Game;
    use formation_chess_core::piece::Piece;
    use formation_chess_core::piece::Player;

    use super::ActionAnalysis;
    use super::MinFeatureVector;
    use super::SideAnalysis;
    use super::action_effect_units;
    use super::analyze_action_outcome;
    use super::analyze_piece_actions;
    use super::exchange_units;
    use super::record_action_kind;
    use super::reply_adjusted_exchange_units;
    use super::weighted_contributions;
    use crate::ActionSelector;
    use crate::MinFeatureWeights;
    use crate::RandomAgent;
    use crate::legal_movement_actions;
    use crate::min::outcome::ResolvedActionKind;
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
            material: 7,
            tempo: 8,
            interactions: 9,
        };
        let weights = MinFeatureWeights {
            vital_safety: 11,
            effective_abilities: 12,
            formation_effects: 13,
            control: 14,
            mobility: 15,
            action_effects: 16,
            material: 17,
            tempo: 18,
            interactions: 19,
        };
        let contributions = weighted_contributions(features, weights);

        assert_eq!(contributions.vital_safety, 11);
        assert_eq!(contributions.effective_abilities, 24);
        assert_eq!(contributions.formation_effects, 39);
        assert_eq!(contributions.control, 56);
        assert_eq!(contributions.mobility, 75);
        assert_eq!(contributions.action_effects, 96);
        assert_eq!(contributions.material, 119);
        assert_eq!(contributions.tempo, 144);
        assert_eq!(contributions.interactions, 171);
    }

    #[test]
    fn pull_actions_contribute_to_action_effects() {
        let mut analysis = ActionAnalysis::default();

        record_action_kind(ResolvedActionKind::Pull, &mut analysis);

        let side = SideAnalysis { actions: analysis, ..SideAnalysis::default() };
        assert_eq!(side.actions.pull_actions, 1);
        assert_eq!(action_effect_units(&side), 1);
    }

    #[test]
    fn harmful_own_capture_does_not_create_soft_action_value() {
        let mut controlled_black_rook = Piece::BLACK_ROOK;
        controlled_black_rook.ability |= Ability::CONTROLLED_BY_RED;
        let mut board = Board::new(5, 5);
        board[(2, 1)] = Some(controlled_black_rook);
        board[(2, 4)] = Some(Piece::RED_PAWN);
        let harmful_capture = Action::Capture(Move { from: (2, 1), to: (2, 4) });
        let mut actions = Vec::new();
        board.valid_moves(Player::Red, (2, 1), &mut actions);
        assert!(actions.contains(&harmful_capture));

        let outcome = analyze_action_outcome(&board, Player::Red, harmful_capture);
        assert!(outcome.exchange_units < 0);
        assert_eq!(outcome.kind, ResolvedActionKind::Capture);

        let mut analysis = ActionAnalysis::default();
        analyze_piece_actions(
            &board,
            Player::Red,
            (2, 1),
            controlled_black_rook,
            &actions,
            &mut analysis,
        );
        let quiet_moves = actions.iter().filter(|action| matches!(action, Action::Move(_))).count();

        assert_eq!(analysis.capture_actions, 0);
        assert_eq!(analysis.quiet_move_actions as usize, quiet_moves);
        let side = SideAnalysis { actions: analysis, ..SideAnalysis::default() };
        assert_eq!(action_effect_units(&side), 0);
    }

    #[test]
    fn static_exchange_replies_reject_losing_captures_and_keep_defended_captures() {
        let capture = Action::Capture(Move { from: (0, 2), to: (2, 2) });
        let mut hanging = Board::new(5, 5);
        hanging[(0, 2)] = Some(Piece::RED_ROOK);
        hanging[(2, 0)] = Some(Piece::BLACK_ROOK);
        hanging[(2, 2)] = Some(Piece::BLACK_PAWN);
        let hanging_outcome = analyze_action_outcome(&hanging, Player::Red, capture);

        assert!(hanging_outcome.exchange_units > 0);
        assert!(
            reply_adjusted_exchange_units(&hanging, Player::Red, capture, &hanging_outcome) < 0,
            "taking a low-value piece with an undefended rook must lose exchange value",
        );

        let mut defended = hanging.clone();
        defended[(4, 2)] = Some(Piece::RED_ROOK);
        let defended_outcome = analyze_action_outcome(&defended, Player::Red, capture);
        assert!(
            reply_adjusted_exchange_units(&defended, Player::Red, capture, &defended_outcome) > 0,
            "a recapture must restore the defended capture's exchange value",
        );

        let mut hanging_analysis = ActionAnalysis::default();
        analyze_piece_actions(
            &hanging,
            Player::Red,
            (0, 2),
            Piece::RED_ROOK,
            &[capture],
            &mut hanging_analysis,
        );
        let mut defended_analysis = ActionAnalysis::default();
        analyze_piece_actions(
            &defended,
            Player::Red,
            (0, 2),
            Piece::RED_ROOK,
            &[capture],
            &mut defended_analysis,
        );

        assert_eq!(hanging_analysis.capture_actions, 0);
        assert_eq!(defended_analysis.capture_actions, 1);
        assert_eq!(hanging_analysis.positive_exchange_actions, 0);
        assert_eq!(defended_analysis.positive_exchange_actions, 1);
    }

    #[test]
    fn harmful_vital_capture_does_not_create_escape_value() {
        let mut red_general = Piece::RED_GENERAL;
        red_general.ability |= Ability::CAPTURE;
        let mut board = Board::new(3, 3);
        board[(0, 0)] = Some(red_general);
        board[(2, 2)] = Some(Piece::RED_PAWN);
        let harmful_capture = Action::Capture(Move { from: (0, 0), to: (2, 2) });
        board.try_capture((0, 0), (2, 2)).expect("vital own capture must be legal");

        let mut analysis = ActionAnalysis::default();
        analyze_piece_actions(
            &board,
            Player::Red,
            (0, 0),
            red_general,
            &[harmful_capture],
            &mut analysis,
        );

        assert_eq!(analysis.vital_safe_actions, 0);
        assert_eq!(analysis.capture_actions, 0);
        assert_eq!(analysis.quiet_move_actions, 0);
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

        let mut actions = Vec::new();
        for _ in 0 .. 32 {
            actions.clear();
            legal_movement_actions(&game, &mut actions);
            for &action in &actions {
                let expected = game.try_action(action).expect("enumerated action must be legal");
                let predicted = analyze_action_outcome(game.board(), game.player(), action);
                assert_eq!(
                    predicted.game_result, expected.game_result,
                    "predicted result differs for {action:?}"
                );
                assert_eq!(
                    predicted.exchange_units,
                    exchange_units(expected.changes.as_slice(), game.player()),
                    "predicted exchange differs for {action:?}"
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
