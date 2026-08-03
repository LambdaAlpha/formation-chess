use std::num::NonZeroU8;

use formation_chess_core::action::Action;
use formation_chess_core::action::Place;
use formation_chess_core::game::Game;
use formation_chess_core::piece::Player;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::seq::IteratorRandom;

use crate::Agent;
use crate::AgentError;
use crate::AgentInput;
use crate::ScoredAction;

/// An agent that returns uniformly sampled candidates with equal scores.
///
/// Placement samples complete piece-position combinations through a lazy
/// iterator without materializing the Cartesian product. Movement samples
/// distinct entries from the supplied legal-action slice.
#[derive(Debug)]
pub struct RandomAgent {
    rng: StdRng,
}

impl RandomAgent {
    /// Create an agent seeded from the process random-number source.
    pub fn new() -> Self {
        Self { rng: rand::make_rng() }
    }

    /// Create a deterministic agent for replayable tests and simulations.
    ///
    /// Reproducibility is guaranteed for the same seed and dependency build;
    /// StdRng may change its algorithm in a future rand release.
    pub fn with_seed(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed) }
    }

    fn analyze_placements(
        &mut self, game: &Game, area: crate::PlacementArea, top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        let pool = match game.player() {
            Player::Red => game.red_pool(),
            Player::Black => game.black_pool(),
        };
        if pool.is_empty() {
            return Err(AgentError::Decision(format!(
                "{} has no pieces left to place",
                game.player()
            )));
        }

        let board = game.board();
        let candidates = pool
            .iter()
            .copied()
            .flat_map(|piece| {
                area.positions().filter(move |position| board.get(*position).is_none()).map(
                    move |to| ScoredAction {
                        action: Action::Place(Place { piece: piece.id(), to }),
                        score: 0.0,
                    },
                )
            })
            .sample(&mut self.rng, usize::from(top_k.get()));

        if candidates.is_empty() {
            return Err(AgentError::Decision("placement area has no empty point".to_owned()));
        }
        Ok(candidates)
    }

    fn analyze_movements(
        &mut self, legal_actions: &[Action], top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        if legal_actions.is_empty() {
            return Err(AgentError::Decision("movement action list is empty".to_owned()));
        }

        Ok(legal_actions
            .sample(&mut self.rng, usize::from(top_k.get()))
            .copied()
            .map(|action| ScoredAction { action, score: 0.0 })
            .collect())
    }
}

impl Default for RandomAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for RandomAgent {
    fn name(&self) -> &str {
        "Random"
    }

    fn analyze(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        match input {
            AgentInput::Placement { area } => self.analyze_placements(game, area, top_k),
            AgentInput::Movement { legal_actions } => self.analyze_movements(legal_actions, top_k),
        }
    }
}
