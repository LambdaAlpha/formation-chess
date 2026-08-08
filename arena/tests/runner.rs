use std::collections::BTreeMap;
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;

use formation_chess_agent::Agent;
use formation_chess_agent::AgentError;
use formation_chess_agent::AgentInput;
use formation_chess_agent::ScoredAction;
use formation_chess_arena::AgentDescriptor;
use formation_chess_arena::AgentFactory;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::GameRunError;
use formation_chess_arena::GameTermination;
use formation_chess_arena::MatchRunner;
use formation_chess_arena::Matchup;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

#[derive(Debug, Copy, Clone)]
enum Behavior {
    Pass,
    Draw,
    Fail,
}

struct TestAgent {
    name: &'static str,
    behavior: Behavior,
}

impl Agent for TestAgent {
    fn name(&self) -> &str {
        self.name
    }

    fn analyze(
        &mut self, _game: &Game, input: AgentInput<'_>, _top_k: NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        if matches!(self.behavior, Behavior::Fail) {
            return Err(AgentError::Decision("planned failure".to_owned()));
        }
        let AgentInput::Movement { legal_actions } = input else {
            return Err(AgentError::Decision("unexpected placement turn".to_owned()));
        };
        let action = legal_actions
            .iter()
            .copied()
            .find(|action| match self.behavior {
                Behavior::Pass => matches!(action, Action::Pass(_)),
                Behavior::Draw => matches!(action, Action::Draw(_)),
                Behavior::Fail => false,
            })
            .ok_or_else(|| AgentError::Decision("requested action is unavailable".to_owned()))?;
        Ok(vec![ScoredAction { action, score: 1.0 }])
    }
}

struct TestFactory {
    kind: &'static str,
    behavior: Behavior,
    seeds: Arc<Mutex<Vec<u64>>>,
}

impl TestFactory {
    fn new(kind: &'static str, behavior: Behavior) -> Self {
        Self { kind, behavior, seeds: Arc::new(Mutex::new(Vec::new())) }
    }

    fn seeds(&self) -> Vec<u64> {
        self.seeds.lock().expect("seed log lock").clone()
    }
}

impl AgentFactory for TestFactory {
    fn descriptor(&self) -> AgentDescriptor {
        AgentDescriptor {
            kind: self.kind.to_owned(),
            display_name: self.kind.to_owned(),
            implementation_version: "test".to_owned(),
            parameters: BTreeMap::new(),
        }
    }

    fn create(&self, seed: u64) -> Box<dyn Agent> {
        self.seeds.lock().expect("seed log lock").push(seed);
        Box::new(TestAgent { name: self.kind, behavior: self.behavior })
    }
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::new(value).expect("valid participant id")
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("nonzero value")
}

fn matchup() -> Matchup {
    Matchup::new(participant("agent_a"), participant("agent_b")).expect("distinct matchup")
}

fn movement_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(0, 4)] = Some(Piece::RED_GENERAL);
    board[(4, 0)] = Some(Piece::BLACK_GENERAL);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
    .expect("valid movement game")
}

fn draw_game() -> Game {
    let mut board = Board::new(5, 5);
    board[(1, 1)] = Some(Piece::RED_GENERAL);
    board[(0, 0)] = Some(Piece::RED_PAWN);
    board[(0, 1)] = Some(Piece::BLACK_GENERAL);
    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
    .expect("valid draw game")
}

#[test]
fn random_factories_execute_standard_placement_and_movement() {
    let matchup = matchup();
    let plan = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 37)
        .next()
        .expect("first fixed game");
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(33)));

    let run = runner.run(plan, Game::default()).expect("valid game plan");

    assert_eq!(run.actions.len(), 33);
    assert!(run.actions[.. 32].iter().all(|action| action.phase == Phase::Place));
    assert_eq!(run.actions[32].phase, Phase::Move);
    assert!(matches!(
        run.termination,
        GameTermination::Completed { .. } | GameTermination::ActionLimit { .. }
    ));
}

#[test]
fn total_action_limit_counts_placement_actions() {
    let matchup = matchup();
    let plan = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 41)
        .next()
        .expect("first fixed game");
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));

    let run = runner.run(plan, Game::default()).expect("valid game plan");

    assert_eq!(run.actions.len(), 1);
    assert_eq!(run.actions[0].phase, Phase::Place);
    assert_eq!(run.final_game.phase(), Phase::Place);
    assert_eq!(run.termination, GameTermination::ActionLimit { limit: nonzero(1) });
}

#[test]
fn runner_caps_configured_action_limit_at_arena_maximum() {
    let matchup = matchup();
    let participant_a = RandomAgentFactory;
    let participant_b = RandomAgentFactory;
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(200)));

    assert_eq!(runner.config().max_actions.get(), 128);
}

#[test]
fn runner_maps_swapped_participants_to_factories_and_seeds() {
    let matchup = matchup();
    let plan = Schedule::new(matchup.clone(), ScheduleMode::Paired { pairs: nonzero(1) }, 11)
        .nth(1)
        .expect("second paired game");
    let participant_a = TestFactory::new("a", Behavior::Pass);
    let participant_b = TestFactory::new("b", Behavior::Pass);
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));

    let run = runner.run(plan.clone(), movement_game()).expect("valid game plan");

    assert_eq!(run.red_agent.kind, "b");
    assert_eq!(run.black_agent.kind, "a");
    assert_eq!(participant_a.seeds(), vec![plan.black_agent_seed]);
    assert_eq!(participant_b.seeds(), vec![plan.red_agent_seed]);
    assert_eq!(run.actions.len(), 1);
    assert_eq!(run.actions[0].player, Player::Red);
    assert_eq!(run.actions[0].phase, Phase::Move);
    assert_eq!(run.actions[0].action, Action::Pass(Player::Red));
    assert_eq!(run.actions[0].candidate_rank, NonZeroU8::MIN);
    assert!(run.actions[0].legal_action_count.is_some_and(|count| count > 0));
    assert_eq!(run.final_game.player(), Player::Black);
    assert_eq!(run.termination, GameTermination::ActionLimit { limit: nonzero(1) });
}

#[test]
fn runner_retains_natural_result_and_executed_action() {
    let matchup = matchup();
    let plan = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 23)
        .next()
        .expect("first fixed game");
    let participant_a = TestFactory::new("draw", Behavior::Draw);
    let participant_b = TestFactory::new("pass", Behavior::Pass);
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(5)));

    let run = runner.run(plan, draw_game()).expect("valid game plan");

    assert_eq!(run.actions.len(), 1);
    assert!(matches!(run.actions[0].action, Action::Draw(_)));
    assert_eq!(run.actions[0].reaction.game_result, GameResult::Draw);
    assert_eq!(run.final_game.result(), GameResult::Draw);
    assert_eq!(run.termination, GameTermination::Completed { result: GameResult::Draw });
}

#[test]
fn runner_retains_agent_failure_without_executing_an_action() {
    let matchup = matchup();
    let plan = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 29)
        .next()
        .expect("first fixed game");
    let participant_a = TestFactory::new("fail", Behavior::Fail);
    let participant_b = TestFactory::new("pass", Behavior::Pass);
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(5)));
    let initial_game = movement_game();
    let initial_state = initial_game.to_string();

    let run = runner.run(plan, initial_game).expect("valid game plan");

    assert!(run.actions.is_empty());
    assert_eq!(run.final_game.to_string(), initial_state);
    assert_eq!(run.termination, GameTermination::AgentFailure {
        player: Player::Red,
        phase: Phase::Move,
        error: AgentError::Decision("planned failure".to_owned()),
    });
}

#[test]
fn runner_rejects_a_plan_from_another_matchup_before_creating_agents() {
    let matchup = matchup();
    let mut plan = Schedule::new(matchup.clone(), ScheduleMode::Fixed { games: nonzero(1) }, 31)
        .next()
        .expect("first fixed game");
    plan.red = participant("outsider");
    let participant_a = TestFactory::new("a", Behavior::Pass);
    let participant_b = TestFactory::new("b", Behavior::Pass);
    let runner =
        MatchRunner::new(matchup, &participant_a, &participant_b, GameRunConfig::new(nonzero(1)));

    let error = runner.run(plan, movement_game()).expect_err("unknown participant must fail");

    assert_eq!(error, GameRunError::UnknownParticipant(participant("outsider")));
    assert!(participant_a.seeds().is_empty());
    assert!(participant_b.seeds().is_empty());
}
