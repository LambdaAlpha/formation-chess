use std::error::Error;
use std::fmt::Display;
use std::num::NonZeroU8;
use std::num::NonZeroU32;

use formation_chess_agent::ActionSelectionPolicy;
use formation_chess_agent::ActionSelector;
use formation_chess_agent::AgentError;
use formation_chess_agent::play_agent_turn;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

use crate::AgentDescriptor;
use crate::AgentFactory;
use crate::GamePlan;
use crate::Matchup;
use crate::ParticipantId;

/// Hard maximum total actions accepted by Arena for one game.
pub const MAX_GAME_ACTIONS: u32 = 128;

/// Safety limits applied while executing one game.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GameRunConfig {
    /// Stop an unfinished game before requesting another action once this many
    /// placement or movement actions have completed. Values above
    /// [`MAX_GAME_ACTIONS`] are clamped.
    pub max_actions: NonZeroU32,
    /// Policy used by both seats to select one analyzed candidate.
    pub action_selection: ActionSelectionPolicy,
}

impl GameRunConfig {
    /// Construct a deterministic run configuration.
    pub const fn new(max_actions: NonZeroU32) -> Self {
        let max_actions = cap_actions(max_actions);
        Self { max_actions, action_selection: ActionSelectionPolicy::Best }
    }

    /// Construct a run configuration with an explicit action-selection policy.
    pub const fn with_action_selection(
        max_actions: NonZeroU32, action_selection: ActionSelectionPolicy,
    ) -> Self {
        let max_actions = cap_actions(max_actions);
        Self { max_actions, action_selection }
    }
}

const fn cap_actions(max_actions: NonZeroU32) -> NonZeroU32 {
    let value = max_actions.get();
    let capped = if value > MAX_GAME_ACTIONS { MAX_GAME_ACTIONS } else { value };
    NonZeroU32::new(capped).expect("Arena action cap must be nonzero")
}

/// Why a game run stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTermination {
    /// The core rules engine produced a decided result.
    Completed { result: GameResult },
    /// The game remained unfinished after the configured total-action limit.
    ActionLimit { limit: NonZeroU32 },
    /// The current agent failed or returned an invalid analysis.
    AgentFailure { player: Player, phase: Phase, error: AgentError },
}

/// One successfully executed action, excluding technical timing data.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedAction {
    pub player: Player,
    pub phase: Phase,
    pub action: Action,
    pub score: f32,
    /// One-based rank of the selected candidate.
    pub candidate_rank: NonZeroU8,
    pub reaction: Reaction,
    /// Number of enumerated legal movement actions, or None during placement.
    pub legal_action_count: Option<usize>,
}

/// Complete in-memory result of one planned game.
#[derive(Debug, Clone)]
pub struct GameRun {
    pub plan: GamePlan,
    pub red_agent: AgentDescriptor,
    pub black_agent: AgentDescriptor,
    pub initial_game: Game,
    pub final_game: Game,
    pub actions: Vec<ExecutedAction>,
    pub termination: GameTermination,
}

/// A game plan that does not belong to the runner's matchup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameRunError {
    DuplicateSeat(ParticipantId),
    UnknownParticipant(ParticipantId),
}

impl Display for GameRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateSeat(participant) => {
                write!(formatter, "game plan assigns participant {participant} to both colors")
            },
            Self::UnknownParticipant(participant) => {
                write!(formatter, "game plan references unknown participant {participant}")
            },
        }
    }
}

impl Error for GameRunError {}

/// Executes plans for one matchup using fresh seeded agents for every game.
pub struct MatchRunner<'factory> {
    matchup: Matchup,
    participant_a: &'factory dyn AgentFactory,
    participant_b: &'factory dyn AgentFactory,
    config: GameRunConfig,
}

impl<'factory> MatchRunner<'factory> {
    pub fn new(
        matchup: Matchup, participant_a: &'factory dyn AgentFactory,
        participant_b: &'factory dyn AgentFactory, config: GameRunConfig,
    ) -> Self {
        Self { matchup, participant_a, participant_b, config }
    }

    pub fn matchup(&self) -> &Matchup {
        &self.matchup
    }

    pub fn config(&self) -> GameRunConfig {
        self.config
    }

    pub(crate) fn participant_descriptors(&self) -> (AgentDescriptor, AgentDescriptor) {
        (self.participant_a.descriptor(), self.participant_b.descriptor())
    }

    /// Execute one plan from an explicitly supplied initial game position.
    ///
    /// The caller may derive randomized scenarios from `plan.scenario_seed`
    /// before calling this method. Agent failures are retained in
    /// [`GameTermination`] so later persistence can record them; only malformed
    /// plan identities are returned as [`GameRunError`].
    pub fn run(&self, plan: GamePlan, mut game: Game) -> Result<GameRun, GameRunError> {
        if plan.red == plan.black {
            return Err(GameRunError::DuplicateSeat(plan.red));
        }
        let red_factory = self.factory_for(&plan.red)?;
        let black_factory = self.factory_for(&plan.black)?;
        let red_agent = red_factory.descriptor();
        let black_agent = black_factory.descriptor();
        let mut red_instance = red_factory.create(plan.red_agent_seed);
        let mut black_instance = black_factory.create(plan.black_agent_seed);
        let mut red_selector =
            ActionSelector::with_seed(self.config.action_selection, plan.red_agent_seed);
        let mut black_selector =
            ActionSelector::with_seed(self.config.action_selection, plan.black_agent_seed);
        let initial_game = game.clone();
        let mut actions = Vec::new();
        let mut action_count = 0_u32;

        let termination = loop {
            let result = game.result();
            if result != GameResult::Unfinished {
                break GameTermination::Completed { result };
            }

            if action_count >= self.config.max_actions.get() {
                break GameTermination::ActionLimit { limit: self.config.max_actions };
            }

            let phase = game.phase();
            let player = game.player();
            let (instance, selector) = match player {
                Player::Red => (red_instance.as_mut(), &mut red_selector),
                Player::Black => (black_instance.as_mut(), &mut black_selector),
            };
            let agent_turn = match play_agent_turn(&mut game, instance, selector) {
                Ok(agent_turn) => agent_turn,
                Err(error) => {
                    break GameTermination::AgentFailure { player, phase, error };
                },
            };

            action_count += 1;
            actions.push(ExecutedAction {
                player: agent_turn.player,
                phase,
                action: agent_turn.action,
                score: agent_turn.score,
                candidate_rank: agent_turn.candidate_rank,
                reaction: agent_turn.reaction,
                legal_action_count: agent_turn.legal_action_count,
            });
        };

        Ok(GameRun {
            plan,
            red_agent,
            black_agent,
            initial_game,
            final_game: game,
            actions,
            termination,
        })
    }

    fn factory_for(
        &self, participant: &ParticipantId,
    ) -> Result<&'factory dyn AgentFactory, GameRunError> {
        if participant == self.matchup.participant_a() {
            Ok(self.participant_a)
        } else if participant == self.matchup.participant_b() {
            Ok(self.participant_b)
        } else {
            Err(GameRunError::UnknownParticipant(participant.clone()))
        }
    }
}
