use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;

use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::piece::Player;
use serde::Deserialize;
use serde::Serialize;

use crate::record::ActionCounts;
use crate::record::ActionData;
use crate::record::ActionRecord;
use crate::record::GameRecord;
use crate::record::GameResultRecord;
use crate::record::PhaseRecord;
use crate::record::PlayerRecord;
use crate::record::TerminationRecord;
use crate::replay::ReplayError;
use crate::replay::replay_with;

/// Replay-verified descriptive metrics for one game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameMetrics {
    pub game_id: u64,
    pub pair_id: Option<u64>,
    pub game_in_pair: Option<u8>,
    pub red_participant_id: String,
    pub black_participant_id: String,
    pub result: GameResultRecord,
    pub termination: TerminationKind,
    pub last_action: Option<LastActionMetrics>,
    pub actions_by_side: ActionsBySideMetrics,
    pub action_types: ActionTypeMetrics,
    pub legal_movement_actions: DistributionMetrics,
    pub reaction_changes: ReactionChangeMetrics,
    pub final_material: FinalMaterialMetrics,
    pub state_visits: StateVisitMetrics,
}

impl GameMetrics {
    /// Verify and replay a record before calculating its metrics.
    pub fn from_record(record: &GameRecord) -> Result<Self, MetricsError> {
        let mut reaction_changes = ReactionChangeMetrics::default();
        let final_game = replay_with(record, |_, _, _, reaction| {
            reaction_changes.record(reaction);
        })?;
        let legal_movement_actions = DistributionMetrics::from_counts(
            record.actions.iter().filter_map(|action| action.legal_action_count),
        );

        Ok(Self {
            game_id: record.game_id,
            pair_id: record.pair_id,
            game_in_pair: record.game_in_pair,
            red_participant_id: record.red.participant_id.clone(),
            black_participant_id: record.black.participant_id.clone(),
            result: record.final_game_result,
            termination: TerminationKind::from(&record.termination),
            last_action: record.actions.last().map(LastActionMetrics::from),
            actions_by_side: ActionsBySideMetrics::from_record(record),
            action_types: ActionTypeMetrics::from_record(record),
            legal_movement_actions,
            reaction_changes,
            final_material: FinalMaterialMetrics::from_game(&final_game),
            state_visits: StateVisitMetrics::from_record(record),
        })
    }
}

impl TryFrom<&GameRecord> for GameMetrics {
    type Error = MetricsError;

    fn try_from(record: &GameRecord) -> Result<Self, Self::Error> {
        Self::from_record(record)
    }
}

/// Termination category without retaining an agent's diagnostic text.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminationKind {
    Completed,
    ActionLimit { limit: u32 },
    AgentFailure { player: PlayerRecord, phase: PhaseRecord },
}

impl From<&TerminationRecord> for TerminationKind {
    fn from(termination: &TerminationRecord) -> Self {
        match termination {
            TerminationRecord::Completed { .. } => Self::Completed,
            TerminationRecord::ActionLimit { limit } => Self::ActionLimit { limit: *limit },
            TerminationRecord::AgentFailure { player, phase, .. } => {
                Self::AgentFailure { player: *player, phase: *phase }
            },
        }
    }
}

/// Stable action category used by descriptive metrics.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Place,
    Move,
    Capture,
    Push,
    Pull,
    Draw,
    Pass,
    Resign,
}

impl From<&ActionData> for ActionKind {
    fn from(action: &ActionData) -> Self {
        match action {
            ActionData::Place { .. } => Self::Place,
            ActionData::Move { .. } => Self::Move,
            ActionData::Capture { .. } => Self::Capture,
            ActionData::Push { .. } => Self::Push,
            ActionData::Pull { .. } => Self::Pull,
            ActionData::Draw { .. } => Self::Draw,
            ActionData::Pass { .. } => Self::Pass,
            ActionData::Resign { .. } => Self::Resign,
        }
    }
}

/// Identity and notation of the final executed action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastActionMetrics {
    pub action_index: u64,
    pub player: PlayerRecord,
    pub phase: PhaseRecord,
    pub action_kind: ActionKind,
    pub notation: String,
}

impl From<&ActionRecord> for LastActionMetrics {
    fn from(action: &ActionRecord) -> Self {
        Self {
            action_index: action.action_index,
            player: action.player,
            phase: action.phase,
            action_kind: ActionKind::from(&action.action),
            notation: action.notation.clone(),
        }
    }
}

/// Per-side action totals split by game phase.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsBySideMetrics {
    pub red: SideActionMetrics,
    pub black: SideActionMetrics,
}

impl ActionsBySideMetrics {
    fn from_record(record: &GameRecord) -> Self {
        Self {
            red: SideActionMetrics::from_counts(&record.action_counts.red),
            black: SideActionMetrics::from_counts(&record.action_counts.black),
        }
    }
}

/// Action totals for one side.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideActionMetrics {
    pub total_actions: u64,
    pub placement_actions: u64,
    pub movement_actions: u64,
}

impl SideActionMetrics {
    fn from_counts(counts: &ActionCounts) -> Self {
        Self {
            total_actions: counts.total_actions,
            placement_actions: counts.placement_phase_actions,
            movement_actions: counts.movement_phase_actions,
        }
    }
}

/// Count and share of one action category among all actions in the game.
#[derive(Debug, Copy, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CountRatio {
    pub count: u64,
    pub ratio: Option<f64>,
}

impl CountRatio {
    fn new(count: u64, total_actions: u64) -> Self {
        let ratio =
            if total_actions == 0 { None } else { Some(count as f64 / total_actions as f64) };
        Self { count, ratio }
    }
}

/// Counts and all-action shares for every action category.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionTypeMetrics {
    pub total_actions: u64,
    pub placements: CountRatio,
    pub moves: CountRatio,
    pub captures: CountRatio,
    pub pushes: CountRatio,
    pub pulls: CountRatio,
    pub draws: CountRatio,
    pub passes: CountRatio,
    pub resignations: CountRatio,
}

impl ActionTypeMetrics {
    fn from_record(record: &GameRecord) -> Self {
        let red = &record.action_counts.red;
        let black = &record.action_counts.black;
        let total_actions = red.total_actions + black.total_actions;
        Self {
            total_actions,
            placements: CountRatio::new(red.placements + black.placements, total_actions),
            moves: CountRatio::new(red.moves + black.moves, total_actions),
            captures: CountRatio::new(red.captures + black.captures, total_actions),
            pushes: CountRatio::new(red.pushes + black.pushes, total_actions),
            pulls: CountRatio::new(red.pulls + black.pulls, total_actions),
            draws: CountRatio::new(red.draws + black.draws, total_actions),
            passes: CountRatio::new(red.passes + black.passes, total_actions),
            resignations: CountRatio::new(red.resignations + black.resignations, total_actions),
        }
    }
}

/// Distribution of integer observations.
#[derive(Debug, Copy, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DistributionMetrics {
    pub count: u64,
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub p25: Option<f64>,
    pub p75: Option<f64>,
    pub p90: Option<f64>,
    pub p95: Option<f64>,
}

impl DistributionMetrics {
    /// Calculate a Type-7 linearly interpolated distribution.
    pub fn from_counts<I>(counts: I) -> Self
    where I: IntoIterator<Item = u64> {
        let mut values = counts.into_iter().collect::<Vec<_>>();
        values.sort_unstable();
        let Some(&min) = values.first() else {
            return Self::default();
        };
        let max = values[values.len() - 1];
        let count = values.len() as u64;
        let mean = values.iter().map(|&value| value as f64).sum::<f64>() / count as f64;

        Self {
            count,
            min: Some(min),
            max: Some(max),
            mean: Some(mean),
            median: Some(type_7_percentile(&values, 0.5)),
            p25: Some(type_7_percentile(&values, 0.25)),
            p75: Some(type_7_percentile(&values, 0.75)),
            p90: Some(type_7_percentile(&values, 0.9)),
            p95: Some(type_7_percentile(&values, 0.95)),
        }
    }
}

/// Occupancy transitions across all verified reactions.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionChangeMetrics {
    pub additions: u64,
    pub removals: u64,
    pub replacements: u64,
}

impl ReactionChangeMetrics {
    fn record(&mut self, reaction: &Reaction) {
        for change in reaction.changes.as_slice() {
            match (change.old, change.new) {
                (None, Some(_)) => self.additions += 1,
                (Some(_), None) => self.removals += 1,
                (Some(_), Some(_)) => self.replacements += 1,
                (None, None) => {},
            }
        }
    }
}

/// Final board and pool material counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalMaterialMetrics {
    pub board_pieces: PiecePlayerCounts,
    pub red_pool_pieces: u64,
    pub black_pool_pieces: u64,
}

impl FinalMaterialMetrics {
    fn from_game(game: &Game) -> Self {
        let mut board_pieces = PiecePlayerCounts::default();
        for (_, piece) in game.board().iter() {
            board_pieces.record(piece.player);
        }
        Self {
            board_pieces,
            red_pool_pieces: game.red_pool().len() as u64,
            black_pool_pieces: game.black_pool().len() as u64,
        }
    }
}

/// Piece counts on the final board, split by owning player.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiecePlayerCounts {
    pub total: u64,
    pub red: u64,
    pub black: u64,
}

impl PiecePlayerCounts {
    fn record(&mut self, player: Player) {
        self.total += 1;
        match player {
            Player::Red => self.red += 1,
            Player::Black => self.black += 1,
        }
    }
}

/// Visits and repetition derived from verified state hashes.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateVisitMetrics {
    pub total_visits: u64,
    pub unique_states: u64,
    pub repeated_visits: u64,
    pub unique_state_ratio: f64,
}

impl StateVisitMetrics {
    fn from_record(record: &GameRecord) -> Self {
        let mut unique_states = HashSet::with_capacity(record.actions.len() + 1);
        unique_states.insert(record.initial_state_sha256.as_str());
        for action in &record.actions {
            unique_states.insert(action.state_after_sha256.as_str());
        }
        let total_visits = record.actions.len() as u64 + 1;
        let unique_states = unique_states.len() as u64;
        Self {
            total_visits,
            unique_states,
            repeated_visits: total_visits - unique_states,
            unique_state_ratio: unique_states as f64 / total_visits as f64,
        }
    }
}

/// Failure to calculate metrics from an invalid record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricsError {
    Replay(ReplayError),
}

impl Display for MetricsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Replay(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for MetricsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Replay(error) => Some(error),
        }
    }
}

impl From<ReplayError> for MetricsError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}

fn type_7_percentile(sorted: &[u64], probability: f64) -> f64 {
    debug_assert!(!sorted.is_empty(), "percentiles require at least one observation");
    let index = (sorted.len() - 1) as f64 * probability;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let lower_value = sorted[lower] as f64;
    let upper_value = sorted[upper] as f64;
    lower_value + (upper_value - lower_value) * (index - lower as f64)
}
