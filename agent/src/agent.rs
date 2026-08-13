use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::game::Game;

use crate::AgentError;
use crate::PlacementArea;

/// Phase-specific information prepared by the agent framework.
#[derive(Debug, Copy, Clone)]
pub enum AgentInput<'a> {
    /// Placement agents inspect the current player's pool and board through
    /// the supplied game, and receive only the compact geometric area.
    Placement { area: PlacementArea },
    /// Movement agents receive the exact legal-action list prepared by the
    /// framework, including controlled-leader Resign actions.
    Movement { legal_actions: &'a [Action] },
}

/// One complete action and its agent-defined score.
///
/// Scores are from the current player's perspective and higher is better.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ScoredAction {
    pub action: Action,
    pub score: f32,
}

/// A Formation Chess agent that ranks complete actions for the current state.
pub trait Agent: Send {
    fn name(&self) -> &str;

    /// Return at most top_k unique legal candidates, ordered from best to
    /// worst. Scores must be finite. Equal scores use list order as the
    /// agent's tie-break order.
    fn analyze(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError>;
}
