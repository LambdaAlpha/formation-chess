use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;

use formation_chess_agent::ActionSelectionPolicy;
use formation_chess_agent::AgentError;
use formation_chess_agent::legal_movement_actions;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::PositionChange;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use crate::AgentDescriptor;
use crate::ExecutedAction;
use crate::GameRun;
use crate::GameRunConfig;
use crate::GameTermination;
use crate::MatchRunner;
use crate::SEED_DERIVATION_VERSION;
use crate::Schedule;
use crate::ScheduleMode;

/// Version of the persisted Arena record schema.
pub const RECORD_SCHEMA_VERSION: u32 = 1;
/// Hash algorithm used for canonical game-state text.
pub const STATE_HASH_ALGORITHM: &str = "sha256";

/// Reproducible metadata shared by every game in one dataset directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaManifest {
    pub schema_version: u32,
    pub arena_version: String,
    pub core_version: String,
    pub seed_derivation_version: u32,
    pub state_hash_algorithm: String,
    pub root_seed: u64,
    pub schedule: ScheduleRecord,
    pub game_run_config: GameRunConfigRecord,
    pub participant_a: ParticipantRecord,
    pub participant_b: ParticipantRecord,
}

impl ArenaManifest {
    /// Build metadata directly from the schedule and runner that will produce
    /// the dataset, rejecting accidentally mismatched matchups.
    pub fn new(schedule: &Schedule, runner: &MatchRunner<'_>) -> Result<Self, RecordError> {
        if schedule.matchup() != runner.matchup() {
            return Err(RecordError::MismatchedMatchup);
        }
        let (participant_a, participant_b) = runner.participant_descriptors();
        Ok(Self {
            schema_version: RECORD_SCHEMA_VERSION,
            arena_version: crate::VERSION.to_owned(),
            core_version: formation_chess_core::VERSION.to_owned(),
            seed_derivation_version: SEED_DERIVATION_VERSION,
            state_hash_algorithm: STATE_HASH_ALGORITHM.to_owned(),
            root_seed: schedule.root_seed(),
            schedule: ScheduleRecord::from(schedule.mode()),
            game_run_config: GameRunConfigRecord::from(runner.config()),
            participant_a: ParticipantRecord {
                id: runner.matchup().participant_a().as_str().to_owned(),
                agent: AgentDescriptorRecord::from(&participant_a),
            },
            participant_b: ParticipantRecord {
                id: runner.matchup().participant_b().as_str().to_owned(),
                agent: AgentDescriptorRecord::from(&participant_b),
            },
        })
    }

    pub fn total_games(&self) -> u64 {
        self.schedule.total_games()
    }

    pub(crate) fn participant(&self, id: &str) -> Option<&ParticipantRecord> {
        if self.participant_a.id == id {
            Some(&self.participant_a)
        } else if self.participant_b.id == id {
            Some(&self.participant_b)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantRecord {
    pub id: String,
    pub agent: AgentDescriptorRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDescriptorRecord {
    pub kind: String,
    pub display_name: String,
    pub implementation_version: String,
    pub parameters: BTreeMap<String, Value>,
}

impl From<&AgentDescriptor> for AgentDescriptorRecord {
    fn from(descriptor: &AgentDescriptor) -> Self {
        Self {
            kind: descriptor.kind.clone(),
            display_name: descriptor.display_name.clone(),
            implementation_version: descriptor.implementation_version.clone(),
            parameters: descriptor.parameters.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ScheduleRecord {
    Fixed { games: u32 },
    Paired { pairs: u32 },
}

impl ScheduleRecord {
    pub fn total_games(self) -> u64 {
        match self {
            Self::Fixed { games } => u64::from(games),
            Self::Paired { pairs } => u64::from(pairs) * 2,
        }
    }
}

impl From<ScheduleMode> for ScheduleRecord {
    fn from(mode: ScheduleMode) -> Self {
        match mode {
            ScheduleMode::Fixed { games } => Self::Fixed { games: games.get() },
            ScheduleMode::Paired { pairs } => Self::Paired { pairs: pairs.get() },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRunConfigRecord {
    pub max_actions: u32,
    #[serde(default)]
    pub action_selection: ActionSelectionPolicyRecord,
}

impl From<GameRunConfig> for GameRunConfigRecord {
    fn from(config: GameRunConfig) -> Self {
        Self {
            max_actions: config.max_actions.get(),
            action_selection: ActionSelectionPolicyRecord::from(config.action_selection),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ActionSelectionPolicyRecord {
    #[default]
    Best,
    RankSoftmax {
        top_k: u8,
        temperature: f32,
    },
    ScoreSoftmax {
        top_k: u8,
        temperature: f32,
        deterministic_gap: f32,
    },
}

impl ActionSelectionPolicyRecord {
    pub const fn top_k(self) -> u8 {
        match self {
            Self::Best => 1,
            Self::RankSoftmax { top_k, .. } | Self::ScoreSoftmax { top_k, .. } => top_k,
        }
    }
}

impl From<ActionSelectionPolicy> for ActionSelectionPolicyRecord {
    fn from(policy: ActionSelectionPolicy) -> Self {
        match policy {
            ActionSelectionPolicy::Best => Self::Best,
            ActionSelectionPolicy::RankSoftmax(policy) => {
                Self::RankSoftmax { top_k: policy.top_k().get(), temperature: policy.temperature() }
            },
            ActionSelectionPolicy::ScoreSoftmax(policy) => Self::ScoreSoftmax {
                top_k: policy.top_k().get(),
                temperature: policy.temperature(),
                deterministic_gap: policy.deterministic_gap(),
            },
        }
    }
}

/// One self-contained game entry in `games.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    pub schema_version: u32,
    pub game_id: u64,
    pub pair_id: Option<u64>,
    pub game_in_pair: Option<u8>,
    pub scenario_seed: u64,
    pub red: SeatRecord,
    pub black: SeatRecord,
    pub initial_state: String,
    pub initial_state_sha256: String,
    pub final_state: String,
    pub final_state_sha256: String,
    pub final_game_result: GameResultRecord,
    pub action_counts: ActionCountsBySide,
    pub termination: TerminationRecord,
    pub actions: Vec<ActionRecord>,
}

impl GameRecord {
    /// Convert a run while replaying every action from the initial state.
    ///
    /// Replay verifies player, phase, reaction, final state, and termination
    /// consistency before any data is persisted.
    pub fn from_game_run(run: &GameRun) -> Result<Self, RecordError> {
        let mut replay = run.initial_game.clone();
        let initial_state = replay.to_string();
        let mut action_counts = ActionCountsBySide::default();
        let mut actions = Vec::with_capacity(run.actions.len());

        for (action_index, executed) in run.actions.iter().enumerate() {
            let action_index = u64::try_from(action_index)
                .map_err(|_| RecordError::InvalidGameRun("action index exceeds u64".to_owned()))?;
            validate_turn_context(&replay, executed, action_index)?;
            let actual_reaction = replay.try_action(executed.action).map_err(|message| {
                RecordError::InvalidGameRun(format!(
                    "action {action_index} failed replay validation: {message}"
                ))
            })?;
            if actual_reaction != executed.reaction {
                return Err(RecordError::InvalidGameRun(format!(
                    "action {action_index} reaction differs from replay"
                )));
            }
            let (notation, reaction_notation) = {
                let resolver = NotationResolver::new(&replay);
                (
                    resolver.fmt_action(&executed.action),
                    resolver.fmt_reaction(Ok(actual_reaction.clone())),
                )
            };
            let applied_reaction = replay.action(executed.action).map_err(|message| {
                RecordError::InvalidGameRun(format!(
                    "validated action {action_index} failed replay: {message}"
                ))
            })?;
            if applied_reaction != actual_reaction {
                return Err(RecordError::InvalidGameRun(format!(
                    "action {action_index} validation and execution reactions differ"
                )));
            }
            action_counts.record(executed.player, executed.phase, executed.action);
            let state_after = replay.to_string();
            actions.push(ActionRecord {
                action_index,
                player: PlayerRecord::from(executed.player),
                phase: PhaseRecord::from(executed.phase),
                action: ActionData::from(executed.action),
                notation,
                score: executed.score,
                candidate_rank: executed.candidate_rank.get(),
                legal_action_count: executed
                    .legal_action_count
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| {
                        RecordError::InvalidGameRun(format!(
                            "action {action_index} legal-action count exceeds u64"
                        ))
                    })?,
                reaction: ReactionRecord::from(&actual_reaction),
                reaction_notation,
                state_after_sha256: state_sha256(&state_after),
            });
        }

        let replayed_final_state = replay.to_string();
        let final_state = run.final_game.to_string();
        if replayed_final_state != final_state {
            return Err(RecordError::InvalidGameRun(
                "final game differs from replayed actions".to_owned(),
            ));
        }
        let termination = termination_record(run, &replay, &action_counts)?;

        Ok(Self {
            schema_version: RECORD_SCHEMA_VERSION,
            game_id: run.plan.game_id,
            pair_id: run.plan.pair_id,
            game_in_pair: run.plan.game_in_pair,
            scenario_seed: run.plan.scenario_seed,
            red: SeatRecord {
                participant_id: run.plan.red.as_str().to_owned(),
                agent_seed: run.plan.red_agent_seed,
            },
            black: SeatRecord {
                participant_id: run.plan.black.as_str().to_owned(),
                agent_seed: run.plan.black_agent_seed,
            },
            initial_state_sha256: state_sha256(&initial_state),
            initial_state,
            final_state_sha256: state_sha256(&final_state),
            final_state,
            final_game_result: GameResultRecord::from(replay.result()),
            action_counts,
            termination,
            actions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatRecord {
    pub participant_id: String,
    pub agent_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Zero-based action index within the game.
    pub action_index: u64,
    pub player: PlayerRecord,
    pub phase: PhaseRecord,
    pub action: ActionData,
    pub notation: String,
    pub score: f32,
    /// One-based rank of the selected candidate.
    #[serde(default = "default_candidate_rank")]
    pub candidate_rank: u8,
    /// Number of legal movement actions, or None during placement.
    pub legal_action_count: Option<u64>,
    pub reaction: ReactionRecord,
    pub reaction_notation: String,
    /// Hash of the canonical full game state after this action.
    pub state_after_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionData {
    Place { piece: PieceRecord, to: PositionRecord },
    Move { from: PositionRecord, to: PositionRecord },
    Capture { from: PositionRecord, to: PositionRecord },
    Push { from: PositionRecord, to: PositionRecord },
    Pull { from: PositionRecord, to: PositionRecord },
    Draw { from: PositionRecord, to: PositionRecord },
    Resign { at: PositionRecord },
}

impl From<Action> for ActionData {
    fn from(action: Action) -> Self {
        match action {
            Action::Place(place) => Self::Place {
                piece: PieceRecord::from(place.piece),
                to: PositionRecord::from(place.to),
            },
            Action::Move(move_) => Self::Move {
                from: PositionRecord::from(move_.from),
                to: PositionRecord::from(move_.to),
            },
            Action::Capture(move_) => Self::Capture {
                from: PositionRecord::from(move_.from),
                to: PositionRecord::from(move_.to),
            },
            Action::Push(move_) => Self::Push {
                from: PositionRecord::from(move_.from),
                to: PositionRecord::from(move_.to),
            },
            Action::Pull(move_) => Self::Pull {
                from: PositionRecord::from(move_.from),
                to: PositionRecord::from(move_.to),
            },
            Action::Draw(move_) => Self::Draw {
                from: PositionRecord::from(move_.from),
                to: PositionRecord::from(move_.to),
            },
            Action::Resign(x, y) => Self::Resign { at: PositionRecord { x, y } },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionRecord {
    pub changes: Vec<PositionChangeRecord>,
    pub game_result: GameResultRecord,
}

impl From<&Reaction> for ReactionRecord {
    fn from(reaction: &Reaction) -> Self {
        Self {
            changes: reaction.changes.iter().copied().map(PositionChangeRecord::from).collect(),
            game_result: GameResultRecord::from(reaction.game_result),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionChangeRecord {
    pub at: PositionRecord,
    pub piece: Option<PieceRecord>,
}

impl From<PositionChange> for PositionChangeRecord {
    fn from(change: PositionChange) -> Self {
        Self { at: PositionRecord::from(change.at), piece: change.new.map(PieceRecord::from) }
    }
}

/// Zero-based board coordinate matching the core API.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionRecord {
    pub x: u8,
    pub y: u8,
}

impl From<(u8, u8)> for PositionRecord {
    fn from((x, y): (u8, u8)) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceRecord {
    pub name: char,
    pub player: PlayerRecord,
}

impl From<Piece> for PieceRecord {
    fn from(piece: Piece) -> Self {
        Self { name: piece.name, player: PlayerRecord::from(piece.player) }
    }
}

impl From<PieceId> for PieceRecord {
    fn from(piece: PieceId) -> Self {
        Self { name: piece.name, player: PlayerRecord::from(piece.player) }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRecord {
    Red,
    Black,
}

impl From<Player> for PlayerRecord {
    fn from(player: Player) -> Self {
        match player {
            Player::Red => Self::Red,
            Player::Black => Self::Black,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseRecord {
    Placement,
    Movement,
}

impl From<Phase> for PhaseRecord {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Place => Self::Placement,
            Phase::Move => Self::Movement,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResultRecord {
    Unfinished,
    RedWin,
    BlackWin,
    Draw,
}

impl From<GameResult> for GameResultRecord {
    fn from(result: GameResult) -> Self {
        match result {
            GameResult::Unfinished => Self::Unfinished,
            GameResult::RedWin => Self::RedWin,
            GameResult::BlackWin => Self::BlackWin,
            GameResult::Draw => Self::Draw,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminationRecord {
    Completed { result: GameResultRecord },
    ActionLimit { limit: u32 },
    AgentFailure { player: PlayerRecord, phase: PhaseRecord, error: AgentErrorRecord },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AgentErrorRecord {
    Decision(String),
    InvalidAnalysis(String),
    GameState(String),
}

impl From<&AgentError> for AgentErrorRecord {
    fn from(error: &AgentError) -> Self {
        match error {
            AgentError::Decision(message) => Self::Decision(message.clone()),
            AgentError::InvalidAnalysis(message) => Self::InvalidAnalysis(message.clone()),
            AgentError::GameState(message) => Self::GameState(message.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionCountsBySide {
    pub red: ActionCounts,
    pub black: ActionCounts,
}

impl ActionCountsBySide {
    pub(crate) fn record(&mut self, player: Player, phase: Phase, action: Action) {
        match player {
            Player::Red => self.red.record(phase, action),
            Player::Black => self.black.record(phase, action),
        }
    }

    pub(crate) fn total_actions(&self) -> u64 {
        self.red.total_actions + self.black.total_actions
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionCounts {
    pub total_actions: u64,
    pub placement_phase_actions: u64,
    pub movement_phase_actions: u64,
    pub placements: u64,
    pub moves: u64,
    pub captures: u64,
    pub pushes: u64,
    pub pulls: u64,
    pub draws: u64,
    pub resignations: u64,
}

impl ActionCounts {
    fn record(&mut self, phase: Phase, action: Action) {
        self.total_actions += 1;
        match phase {
            Phase::Place => self.placement_phase_actions += 1,
            Phase::Move => self.movement_phase_actions += 1,
        }
        match action {
            Action::Place(_) => self.placements += 1,
            Action::Move(_) => self.moves += 1,
            Action::Capture(_) => self.captures += 1,
            Action::Push(_) => self.pushes += 1,
            Action::Pull(_) => self.pulls += 1,
            Action::Draw(_) => self.draws += 1,
            Action::Resign(..) => self.resignations += 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    MismatchedMatchup,
    InvalidGameRun(String),
}

impl Display for RecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MismatchedMatchup => {
                formatter.write_str("schedule and runner use different matchups")
            },
            Self::InvalidGameRun(message) => write!(formatter, "invalid game run: {message}"),
        }
    }
}

impl Error for RecordError {}

pub fn state_sha256(state: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(state.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

const fn default_candidate_rank() -> u8 {
    1
}

fn validate_turn_context(
    replay: &formation_chess_core::game::Game, executed: &ExecutedAction, action_index: u64,
) -> Result<(), RecordError> {
    if replay.player() != executed.player {
        return Err(RecordError::InvalidGameRun(format!(
            "action {action_index} player differs from replay"
        )));
    }
    if replay.phase() != executed.phase {
        return Err(RecordError::InvalidGameRun(format!(
            "action {action_index} phase differs from replay"
        )));
    }
    if !executed.score.is_finite() {
        return Err(RecordError::InvalidGameRun(format!(
            "action {action_index} score is not finite"
        )));
    }
    match (executed.phase, executed.legal_action_count) {
        (Phase::Place, Some(_)) => Err(RecordError::InvalidGameRun(format!(
            "placement action {action_index} has a legal-action count"
        ))),
        (Phase::Move, None) => Err(RecordError::InvalidGameRun(format!(
            "movement action {action_index} has no legal-action count"
        ))),
        (Phase::Move, Some(actual)) => {
            let mut legal_actions = Vec::new();
            legal_movement_actions(replay, &mut legal_actions);
            let expected = legal_actions.len();
            if actual == expected {
                Ok(())
            } else {
                Err(RecordError::InvalidGameRun(format!(
                    "action {action_index} legal-action count is {actual}, expected {expected}"
                )))
            }
        },
        (Phase::Place, None) => Ok(()),
    }
}

fn termination_record(
    run: &GameRun, final_game: &formation_chess_core::game::Game,
    action_counts: &ActionCountsBySide,
) -> Result<TerminationRecord, RecordError> {
    match &run.termination {
        GameTermination::Completed { result } => {
            if *result == GameResult::Unfinished || final_game.result() != *result {
                return Err(RecordError::InvalidGameRun(
                    "completed termination disagrees with final result".to_owned(),
                ));
            }
            Ok(TerminationRecord::Completed { result: GameResultRecord::from(*result) })
        },
        GameTermination::ActionLimit { limit } => {
            if final_game.result() != GameResult::Unfinished
                || action_counts.total_actions() != u64::from(limit.get())
            {
                return Err(RecordError::InvalidGameRun(
                    "action limit termination disagrees with final state or counts".to_owned(),
                ));
            }
            Ok(TerminationRecord::ActionLimit { limit: limit.get() })
        },
        GameTermination::AgentFailure { player, phase, error } => {
            if final_game.result() != GameResult::Unfinished
                || final_game.player() != *player
                || final_game.phase() != *phase
            {
                return Err(RecordError::InvalidGameRun(
                    "agent failure termination disagrees with final state".to_owned(),
                ));
            }
            Ok(TerminationRecord::AgentFailure {
                player: PlayerRecord::from(*player),
                phase: PhaseRecord::from(*phase),
                error: AgentErrorRecord::from(error),
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use formation_chess_agent::ActionSelectionPolicy;

    use super::ActionSelectionPolicyRecord;

    #[test]
    fn score_softmax_policy_round_trips_to_record() {
        let policy = ActionSelectionPolicy::score_softmax(
            NonZeroU8::new(4).expect("nonzero top_k"),
            0.02,
            0.05,
        )
        .expect("valid score policy");
        let record = ActionSelectionPolicyRecord::from(policy);

        assert_eq!(record, ActionSelectionPolicyRecord::ScoreSoftmax {
            top_k: 4,
            temperature: 0.02,
            deterministic_gap: 0.05,
        });
        let json = serde_json::to_value(record).expect("serialize score policy");
        assert_eq!(json["mode"], "score_softmax");
    }
}
