use std::num::NonZeroU8;
use std::time::Duration;
use std::time::Instant;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

use crate::ActionSelector;
use crate::Agent;
use crate::AgentError;
use crate::AgentInput;
use crate::ScoredAction;
use crate::legal_movement_actions;
use crate::placement_area;

/// Owned phase-specific data prepared for one immutable game position.
#[derive(Debug, Clone)]
pub enum PreparedInput {
    /// Compact placement geometry; the borrowed game supplies pools and occupancy.
    Placement { area: crate::PlacementArea },
    /// The exact legal movement actions supplied to the agent.
    Movement { legal_actions: Vec<Action> },
}

/// A prepared agent turn tied to the exact game position that produced it.
///
/// The immutable game borrow prevents the position from changing while the
/// prepared movement-action list is inspected or passed to an agent.
#[derive(Debug)]
pub struct PreparedTurn<'game> {
    game: &'game Game,
    player: Player,
    input: PreparedInput,
}

impl PreparedTurn<'_> {
    /// The immutable game position used to prepare this turn.
    pub fn game(&self) -> &Game {
        self.game
    }

    /// The player whose turn was prepared.
    pub fn player(&self) -> Player {
        self.player
    }

    /// The phase encoded by the prepared input.
    pub fn phase(&self) -> Phase {
        match &self.input {
            PreparedInput::Placement { .. } => Phase::Place,
            PreparedInput::Movement { .. } => Phase::Move,
        }
    }

    /// Inspect the owned phase-specific input.
    pub fn input(&self) -> &PreparedInput {
        &self.input
    }

    /// Number of enumerated movement actions, or None for placement.
    pub fn legal_action_count(&self) -> Option<usize> {
        match &self.input {
            PreparedInput::Placement { .. } => None,
            PreparedInput::Movement { legal_actions } => Some(legal_actions.len()),
        }
    }

    fn agent_input(&self) -> AgentInput<'_> {
        match &self.input {
            PreparedInput::Placement { area } => AgentInput::Placement { area: *area },
            PreparedInput::Movement { legal_actions } => AgentInput::Movement { legal_actions },
        }
    }
}

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
    /// One-based rank of the selected candidate in the validated analysis.
    pub candidate_rank: NonZeroU8,
    pub reaction: Reaction,
    pub decision_time: Duration,
    pub legal_action_count: Option<usize>,
}

/// Prepare phase-specific data for the current unfinished game position.
///
/// Movement actions are enumerated exactly once and owned by the returned
/// value. Placement retains only the compact geometric area.
pub fn prepare_turn(game: &Game) -> Result<PreparedTurn<'_>, AgentError> {
    if game.result() != GameResult::Unfinished {
        return Err(AgentError::GameState(format!("game result is {}", game.result())));
    }

    let input = match game.phase() {
        Phase::Place => {
            let area = placement_area(game).ok_or_else(|| {
                AgentError::GameState("placement input is unavailable".to_owned())
            })?;
            PreparedInput::Placement { area }
        },
        Phase::Move => {
            let mut legal_actions = Vec::with_capacity(128);
            legal_movement_actions(game, &mut legal_actions);
            PreparedInput::Movement { legal_actions }
        },
    };

    Ok(PreparedTurn { game, player: game.player(), input })
}

/// Ask an agent to analyze one previously prepared game position.
///
/// The returned list is verified to be nonempty, bounded, sorted, unique,
/// finite-scored, and legal against the prepared input.
pub fn analyze_prepared(
    prepared: &PreparedTurn<'_>, agent: &mut dyn Agent, top_k: NonZeroU8,
) -> Result<AgentAnalysis, AgentError> {
    let input = prepared.agent_input();
    let started = Instant::now();
    let candidates = agent.analyze(prepared.game(), input, top_k)?;
    let decision_time = started.elapsed();
    validate_analysis(prepared.game(), input, top_k, &candidates)?;

    Ok(AgentAnalysis {
        player: prepared.player(),
        candidates,
        decision_time,
        legal_action_count: prepared.legal_action_count(),
    })
}

/// Prepare the current position and ask an agent for up to top_k candidates.
pub fn analyze_agent(
    game: &Game, agent: &mut dyn Agent, top_k: NonZeroU8,
) -> Result<AgentAnalysis, AgentError> {
    let prepared = prepare_turn(game)?;
    analyze_prepared(&prepared, agent, top_k)
}

/// Analyze candidates, select one through the supplied policy, and execute it.
pub fn play_agent_turn(
    game: &mut Game, agent: &mut dyn Agent, selector: &mut ActionSelector,
) -> Result<AgentTurn, AgentError> {
    let analysis = analyze_agent(game, agent, selector.top_k())?;
    let selected_index = selector.select_index(analysis.candidates.len());
    let selected = analysis.candidates[selected_index];
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
        candidate_rank: NonZeroU8::new(
            u8::try_from(selected_index + 1).expect("validated candidate rank must fit NonZeroU8"),
        )
        .expect("selected candidate rank must be nonzero"),
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
