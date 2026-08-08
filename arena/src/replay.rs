use std::error::Error;
use std::fmt::Display;

use formation_chess_agent::legal_movement_actions;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;

use crate::record::ActionCountsBySide;
use crate::record::ActionData;
use crate::record::ActionRecord;
use crate::record::GameRecord;
use crate::record::GameResultRecord;
use crate::record::PhaseRecord;
use crate::record::PieceRecord;
use crate::record::PlayerRecord;
use crate::record::PositionRecord;
use crate::record::RECORD_SCHEMA_VERSION;
use crate::record::ReactionRecord;
use crate::record::TerminationRecord;
use crate::record::state_sha256;

/// Strictly replays persisted game records without modifying or migrating them.
#[derive(Debug, Copy, Clone, Default)]
pub struct ReplayVerifier;

impl ReplayVerifier {
    /// Verify every reproducible field in one game record.
    ///
    /// Agent scores cannot be reproduced and are therefore checked only for
    /// finiteness. Agent failure text is retained data; its player and phase are
    /// verified against the final replay state.
    pub fn verify(record: &GameRecord) -> Result<(), ReplayError> {
        replay_with(record, |_, _, _, _| {}).map(|_| ())
    }
}

pub(crate) fn replay_with<F>(record: &GameRecord, mut observer: F) -> Result<Game, ReplayError>
where F: FnMut(&Game, &ActionRecord, Action, &Reaction) {
    if record.schema_version != RECORD_SCHEMA_VERSION {
        return Err(game_error(
            record,
            format!("unsupported schema version {}", record.schema_version),
        ));
    }
    verify_state_hash(record, "initial", &record.initial_state, &record.initial_state_sha256)?;
    let mut replay = record.initial_state.parse::<Game>().map_err(|message| {
        game_error(record, format!("initial state cannot be parsed: {message}"))
    })?;
    if replay.to_string() != record.initial_state {
        return Err(game_error(record, "initial state is not canonical"));
    }

    let mut action_counts = ActionCountsBySide::default();
    for (index, action_record) in record.actions.iter().enumerate() {
        let action_index =
            u64::try_from(index).map_err(|_| game_error(record, "action index exceeds u64"))?;
        verify_action(
            record,
            &mut replay,
            &mut action_counts,
            action_index,
            action_record,
            &mut observer,
        )?;
    }

    verify_state_hash(record, "final", &record.final_state, &record.final_state_sha256)?;
    let replayed_final_state = replay.to_string();
    if replayed_final_state != record.final_state {
        return Err(game_error(record, "final state differs from replayed actions"));
    }
    if GameResultRecord::from(replay.result()) != record.final_game_result {
        return Err(game_error(record, "final game result differs from replayed state"));
    }
    if action_counts != record.action_counts {
        return Err(game_error(record, "action counts differ from replayed actions"));
    }
    verify_termination(record, &replay, &action_counts)?;
    Ok(replay)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    Game { game_id: u64, message: String },
    Action { game_id: u64, action_index: u64, message: String },
}

impl Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Game { game_id, message } => {
                write!(formatter, "game {game_id} replay verification failed: {message}")
            },
            Self::Action { game_id, action_index, message } => write!(
                formatter,
                "game {game_id} action {action_index} replay verification failed: {message}"
            ),
        }
    }
}

impl Error for ReplayError {}

fn verify_action<F>(
    record: &GameRecord, replay: &mut Game, action_counts: &mut ActionCountsBySide,
    action_index: u64, stored: &ActionRecord, observer: &mut F,
) -> Result<(), ReplayError>
where
    F: FnMut(&Game, &ActionRecord, Action, &Reaction),
{
    if stored.action_index != action_index {
        return Err(action_error(
            record,
            action_index,
            format!("stored action index is {}", stored.action_index),
        ));
    }
    let player = player_from_record(stored.player);
    if replay.player() != player {
        return Err(action_error(record, action_index, "player differs from replay state"));
    }
    let phase = phase_from_record(stored.phase);
    if replay.phase() != phase {
        return Err(action_error(record, action_index, "phase differs from replay state"));
    }
    if !stored.score.is_finite() {
        return Err(action_error(record, action_index, "score is not finite"));
    }
    if stored.candidate_rank == 0 {
        return Err(action_error(record, action_index, "candidate rank must be nonzero"));
    }
    verify_legal_action_count(record, replay, action_index, stored.legal_action_count)?;

    let action = action_from_data(&stored.action)
        .map_err(|message| action_error(record, action_index, message))?;
    let reaction = replay.try_action(action).map_err(|message| {
        action_error(record, action_index, format!("action is not legal: {message}"))
    })?;
    let (notation, reaction_notation) = {
        let resolver = NotationResolver::new(replay);
        (resolver.fmt_action(&action), resolver.fmt_reaction(Ok(reaction.clone())))
    };
    if stored.notation != notation {
        return Err(action_error(record, action_index, "action notation differs from replay"));
    }
    if stored.reaction != ReactionRecord::from(&reaction) {
        return Err(action_error(record, action_index, "reaction differs from replay"));
    }
    if stored.reaction_notation != reaction_notation {
        return Err(action_error(record, action_index, "reaction notation differs from replay"));
    }

    observer(replay, stored, action, &reaction);
    let applied_reaction = replay.action(action).map_err(|message| {
        action_error(record, action_index, format!("validated action failed execution: {message}"))
    })?;
    if applied_reaction != reaction {
        return Err(action_error(
            record,
            action_index,
            "validation and execution reactions differ",
        ));
    }
    action_counts.record(player, phase, action);
    let state_after_sha256 = state_sha256(&replay.to_string());
    if stored.state_after_sha256 != state_after_sha256 {
        return Err(action_error(
            record,
            action_index,
            "post-action state hash differs from replay",
        ));
    }
    Ok(())
}

fn verify_legal_action_count(
    record: &GameRecord, replay: &Game, action_index: u64, stored: Option<u64>,
) -> Result<(), ReplayError> {
    match replay.phase() {
        Phase::Place => {
            if stored.is_some() {
                Err(action_error(record, action_index, "placement action has a legal-action count"))
            } else {
                Ok(())
            }
        },
        Phase::Move => {
            let Some(stored) = stored else {
                return Err(action_error(
                    record,
                    action_index,
                    "movement action has no legal-action count",
                ));
            };
            let mut legal_actions = Vec::new();
            legal_movement_actions(replay, &mut legal_actions);
            let expected = u64::try_from(legal_actions.len()).map_err(|_| {
                action_error(record, action_index, "legal-action count exceeds u64")
            })?;
            if stored == expected {
                Ok(())
            } else {
                Err(action_error(
                    record,
                    action_index,
                    format!("legal-action count is {stored}, expected {expected}"),
                ))
            }
        },
    }
}

fn verify_state_hash(
    record: &GameRecord, label: &str, state: &str, stored_hash: &str,
) -> Result<(), ReplayError> {
    if stored_hash == state_sha256(state) {
        Ok(())
    } else {
        Err(game_error(record, format!("{label} state hash is invalid")))
    }
}

fn verify_termination(
    record: &GameRecord, replay: &Game, action_counts: &ActionCountsBySide,
) -> Result<(), ReplayError> {
    match &record.termination {
        TerminationRecord::Completed { result } => {
            let result = game_result_from_record(*result);
            if result == GameResult::Unfinished || replay.result() != result {
                return Err(game_error(
                    record,
                    "completed termination differs from final replay state",
                ));
            }
        },
        TerminationRecord::ActionLimit { limit } => {
            if *limit == 0
                || replay.result() != GameResult::Unfinished
                || action_counts.total_actions() != u64::from(*limit)
            {
                return Err(game_error(
                    record,
                    "action-limit termination differs from final replay state or counts",
                ));
            }
        },
        TerminationRecord::AgentFailure { player, phase, .. } => {
            if replay.result() != GameResult::Unfinished
                || replay.player() != player_from_record(*player)
                || replay.phase() != phase_from_record(*phase)
            {
                return Err(game_error(
                    record,
                    "agent-failure termination differs from final replay state",
                ));
            }
        },
    }
    Ok(())
}

fn action_from_data(data: &ActionData) -> Result<Action, String> {
    Ok(match data {
        ActionData::Place { piece, to } => {
            Action::Place(Place { piece: piece_from_record(piece)?, to: position_from_record(*to) })
        },
        ActionData::Move { from, to } => {
            Action::Move(Move { from: position_from_record(*from), to: position_from_record(*to) })
        },
        ActionData::Capture { from, to } => Action::Capture(Move {
            from: position_from_record(*from),
            to: position_from_record(*to),
        }),
        ActionData::Push { from, to } => {
            Action::Push(Move { from: position_from_record(*from), to: position_from_record(*to) })
        },
        ActionData::Pull { from, to } => {
            Action::Pull(Move { from: position_from_record(*from), to: position_from_record(*to) })
        },
        ActionData::Draw { from, to } => {
            Action::Draw(Move { from: position_from_record(*from), to: position_from_record(*to) })
        },
        ActionData::Pass { player } => Action::Pass(player_from_record(*player)),
        ActionData::Resign { at } => {
            let at = position_from_record(*at);
            Action::Resign(at.0, at.1)
        },
    })
}

fn piece_from_record(record: &PieceRecord) -> Result<PieceId, String> {
    let player = player_from_record(record.player);
    let Some(piece) = Piece::lookup(record.name, player) else {
        return Err(format!(
            "action references unknown piece {:?} for player {:?}",
            record.name, record.player
        ));
    };
    Ok(piece.id())
}

const fn position_from_record(record: PositionRecord) -> (u8, u8) {
    (record.x, record.y)
}

const fn player_from_record(record: PlayerRecord) -> Player {
    match record {
        PlayerRecord::Red => Player::Red,
        PlayerRecord::Black => Player::Black,
    }
}

const fn phase_from_record(record: PhaseRecord) -> Phase {
    match record {
        PhaseRecord::Placement => Phase::Place,
        PhaseRecord::Movement => Phase::Move,
    }
}

const fn game_result_from_record(record: GameResultRecord) -> GameResult {
    match record {
        GameResultRecord::Unfinished => GameResult::Unfinished,
        GameResultRecord::RedWin => GameResult::RedWin,
        GameResultRecord::BlackWin => GameResult::BlackWin,
        GameResultRecord::Draw => GameResult::Draw,
    }
}

fn game_error(record: &GameRecord, message: impl Into<String>) -> ReplayError {
    ReplayError::Game { game_id: record.game_id, message: message.into() }
}

fn action_error(record: &GameRecord, action_index: u64, message: impl Into<String>) -> ReplayError {
    ReplayError::Action { game_id: record.game_id, action_index, message: message.into() }
}
