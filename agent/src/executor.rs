use std::num::NonZeroU8;
use std::time::Duration;
use std::time::Instant;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

use crate::Agent;
use crate::AgentError;
use crate::AgentInput;
use crate::ScoredAction;
use crate::legal_movement_actions;
use crate::placement_area;

/// A validated, read-only agent analysis for the current position.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentAnalysis {
    pub player: Player,
    pub candidates: Vec<ScoredAction>,
    /// Time spent inside Agent::analyze. Framework enumeration and output
    /// validation are deliberately excluded.
    pub decision_time: Duration,
    /// Number of supplied movement candidates, or None for placement.
    pub legal_action_count: Option<usize>,
}

/// The observable result of one successfully executed agent turn.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentTurn {
    pub player: Player,
    pub action: Action,
    pub score: f32,
    pub reaction: Reaction,
    pub decision_time: Duration,
    pub legal_action_count: Option<usize>,
}

/// Ask an agent for up to top_k ranked candidates without changing the game.
///
/// The framework prepares phase-specific input and verifies that the returned
/// list is nonempty, bounded, sorted, unique, finite-scored, and legal.
pub fn analyze_agent(
    game: &Game, agent: &mut dyn Agent, top_k: NonZeroU8,
) -> Result<AgentAnalysis, AgentError> {
    if game.result() != GameResult::Unfinished {
        return Err(AgentError::GameState(format!("game result is {}", game.result())));
    }

    let player = game.player();
    let (candidates, decision_time, legal_action_count) = match game.phase() {
        Phase::Place => {
            let area = placement_area(game).ok_or_else(|| {
                AgentError::GameState("placement input is unavailable".to_owned())
            })?;
            let input = AgentInput::Placement { area };
            let started = Instant::now();
            let candidates = agent.analyze(game, input, top_k)?;
            let decision_time = started.elapsed();
            validate_analysis(game, input, top_k, &candidates)?;
            (candidates, decision_time, None)
        },
        Phase::Move => {
            let legal_actions = legal_movement_actions(game);
            let legal_action_count = legal_actions.len();
            let input = AgentInput::Movement { legal_actions: &legal_actions };
            let started = Instant::now();
            let candidates = agent.analyze(game, input, top_k)?;
            let decision_time = started.elapsed();
            validate_analysis(game, input, top_k, &candidates)?;
            (candidates, decision_time, Some(legal_action_count))
        },
    };

    Ok(AgentAnalysis { player, candidates, decision_time, legal_action_count })
}

/// Analyze one candidate and execute the best result through Game::action.
pub fn play_agent_turn(game: &mut Game, agent: &mut dyn Agent) -> Result<AgentTurn, AgentError> {
    let analysis = analyze_agent(game, agent, NonZeroU8::MIN)?;
    let selected = analysis.candidates[0];
    let reaction = game.action(selected.action).map_err(|message| {
        AgentError::InvalidAnalysis(format!(
            "validated action {:?} failed execution: {message}",
            selected.action
        ))
    })?;

    Ok(AgentTurn {
        player: analysis.player,
        action: selected.action,
        score: selected.score,
        reaction,
        decision_time: analysis.decision_time,
        legal_action_count: analysis.legal_action_count,
    })
}

fn validate_analysis(
    game: &Game, input: AgentInput<'_>, top_k: NonZeroU8, candidates: &[ScoredAction],
) -> Result<(), AgentError> {
    if candidates.is_empty() {
        return Err(AgentError::InvalidAnalysis("candidate list is empty".to_owned()));
    }
    if candidates.len() > usize::from(top_k.get()) {
        return Err(AgentError::InvalidAnalysis(format!(
            "returned {} candidates for top_k {}",
            candidates.len(),
            top_k.get()
        )));
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if !candidate.score.is_finite() {
            return Err(AgentError::InvalidAnalysis(format!(
                "candidate {index} has non-finite score {}",
                candidate.score
            )));
        }
        if index > 0 && candidates[index - 1].score < candidate.score {
            return Err(AgentError::InvalidAnalysis(format!(
                "candidate scores are not descending at indices {} and {index}",
                index - 1
            )));
        }
        if candidates[.. index].iter().any(|previous| previous.action == candidate.action) {
            return Err(AgentError::InvalidAnalysis(format!(
                "candidate {index} duplicates action {:?}",
                candidate.action
            )));
        }

        validate_candidate(game, input, index, *candidate)?;
    }

    Ok(())
}

fn validate_candidate(
    game: &Game, input: AgentInput<'_>, index: usize, candidate: ScoredAction,
) -> Result<(), AgentError> {
    match input {
        AgentInput::Placement { area } => {
            let Action::Place(place) = candidate.action else {
                return Err(AgentError::InvalidAnalysis(format!(
                    "candidate {index} is not a placement action: {:?}",
                    candidate.action
                )));
            };
            if !area.contains(place.to) {
                return Err(AgentError::InvalidAnalysis(format!(
                    "candidate {index} places outside the supplied area: {:?}",
                    candidate.action
                )));
            }
            game.try_action(candidate.action).map_err(|message| {
                AgentError::InvalidAnalysis(format!(
                    "candidate {index} is not a legal placement: {message}"
                ))
            })?;
        },
        AgentInput::Movement { legal_actions } => {
            if !legal_actions.contains(&candidate.action) {
                return Err(AgentError::InvalidAnalysis(format!(
                    "candidate {index} was not present in the supplied legal-action list: {:?}",
                    candidate.action
                )));
            }
        },
    }

    Ok(())
}
