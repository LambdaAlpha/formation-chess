use std::cmp::Ordering;
use std::fmt::Display;
use std::num::NonZeroU16;
use std::num::NonZeroU32;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Place;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::Agent;
use crate::AgentError;
use crate::AgentInput;
use crate::PlacementArea;
use crate::ScoredAction;
use crate::placement_area;

/// Schema version of the serialized MCTS configuration.
pub const MCTS_CONFIG_SCHEMA_VERSION: u16 = 1;
/// Version of the canonical text hashed for configuration identity.
pub const MCTS_CONFIG_HASH_FORMAT_VERSION: u16 = 1;
/// Hash algorithm used by [`MctsConfig::sha256`].
pub const MCTS_CONFIG_HASH_ALGORITHM: &str = "sha256";
/// Identifier of the bundled pure UCT baseline.
pub const MCTS_BASELINE_CONFIG_ID: &str = "baseline";
/// Version of the bundled pure UCT baseline.
pub const MCTS_BASELINE_CONFIG_VERSION: u16 = 1;
/// Hard maximum number of UCT iterations per analysis.
pub const MCTS_MAX_ITERATIONS: u32 = 1_000_000;
/// Hard maximum simulated actions from an analyzed root.
pub const MCTS_MAX_SIMULATION_ACTIONS: u16 = 128;
/// Default pure UCT iteration budget.
pub const MCTS_BASELINE_ITERATIONS: u32 = 128;
/// Default UCT exploration constant selected by release Arena comparisons.
pub const MCTS_BASELINE_EXPLORATION: f32 = 0.7;

/// Reproducible pure UCT configuration.
///
/// The search has no action priors and no static evaluator. Selection uses
/// UCT, expansion order is seeded and uniform, rollouts choose uniformly from
/// all legal actions, terminal rewards are `-1`, `0`, and `1`, and a rollout
/// reaching `max_simulation_actions` receives the neutral value `0`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MctsConfig {
    pub schema_version: u16,
    pub config_id: String,
    pub config_version: NonZeroU16,
    pub iterations: NonZeroU32,
    pub max_simulation_actions: NonZeroU16,
    pub exploration: f32,
}

impl MctsConfig {
    /// Current source-defined pure UCT baseline.
    pub fn baseline() -> Self {
        Self {
            schema_version: MCTS_CONFIG_SCHEMA_VERSION,
            config_id: MCTS_BASELINE_CONFIG_ID.to_owned(),
            config_version: non_zero_u16(MCTS_BASELINE_CONFIG_VERSION),
            iterations: non_zero_u32(MCTS_BASELINE_ITERATIONS),
            max_simulation_actions: non_zero_u16(MCTS_MAX_SIMULATION_ACTIONS),
            exploration: MCTS_BASELINE_EXPLORATION,
        }
    }

    /// Stable versioned label such as `baseline-v1`.
    pub fn versioned_id(&self) -> String {
        format!("{}-v{}", self.config_id, self.config_version)
    }

    /// Validate the supported pure UCT configuration contract.
    pub fn validate(&self) -> Result<(), MctsConfigError> {
        if self.schema_version != MCTS_CONFIG_SCHEMA_VERSION {
            return Err(MctsConfigError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                supported: MCTS_CONFIG_SCHEMA_VERSION,
            });
        }
        if !is_valid_config_id(&self.config_id) {
            return Err(MctsConfigError::InvalidConfigId(self.config_id.clone()));
        }
        if self.iterations.get() > MCTS_MAX_ITERATIONS {
            return Err(MctsConfigError::ValueAboveMaximum {
                field: "iterations",
                actual: u64::from(self.iterations.get()),
                maximum: u64::from(MCTS_MAX_ITERATIONS),
            });
        }
        if self.max_simulation_actions.get() > MCTS_MAX_SIMULATION_ACTIONS {
            return Err(MctsConfigError::ValueAboveMaximum {
                field: "max_simulation_actions",
                actual: u64::from(self.max_simulation_actions.get()),
                maximum: u64::from(MCTS_MAX_SIMULATION_ACTIONS),
            });
        }
        if !self.exploration.is_finite() || self.exploration <= 0.0 {
            return Err(MctsConfigError::InvalidExploration(self.exploration.to_bits()));
        }
        Ok(())
    }

    /// Stable canonical text covered by [`Self::sha256`].
    pub fn canonical_text(&self) -> Result<String, MctsConfigError> {
        self.validate()?;
        Ok(format!(
            concat!(
                "formation-chess-mcts-config\n",
                "hash_format_version={}\n",
                "schema_version={}\n",
                "config_id={}\n",
                "config_version={}\n",
                "iterations={}\n",
                "max_simulation_actions={}\n",
                "exploration_bits={:08x}\n",
            ),
            MCTS_CONFIG_HASH_FORMAT_VERSION,
            self.schema_version,
            self.config_id,
            self.config_version,
            self.iterations,
            self.max_simulation_actions,
            self.exploration.to_bits(),
        ))
    }

    /// SHA-256 of [`Self::canonical_text`].
    pub fn sha256(&self) -> Result<String, MctsConfigError> {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let digest = Sha256::digest(self.canonical_text()?.as_bytes());
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            output.push(HEX[usize::from(byte >> 4)] as char);
            output.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        Ok(output)
    }
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self::baseline()
    }
}

/// An MCTS configuration that cannot be used safely or reproduced correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MctsConfigError {
    UnsupportedSchemaVersion { actual: u16, supported: u16 },
    InvalidConfigId(String),
    ValueAboveMaximum { field: &'static str, actual: u64, maximum: u64 },
    InvalidExploration(u32),
}

impl Display for MctsConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { actual, supported } => write!(
                formatter,
                "unsupported MCTS config schema version {actual}; supported version is {supported}"
            ),
            Self::InvalidConfigId(id) => {
                write!(formatter, "invalid MCTS config ID {id:?}; expected [a-z][a-z0-9-]{{0,31}}")
            },
            Self::ValueAboveMaximum { field, actual, maximum } => {
                write!(formatter, "MCTS config field {field} is {actual}; maximum is {maximum}")
            },
            Self::InvalidExploration(bits) => {
                let exploration = f32::from_bits(*bits);
                write!(formatter, "MCTS exploration must be finite and positive, got {exploration}")
            },
        }
    }
}

impl std::error::Error for MctsConfigError {}

/// Technical statistics from the most recent analysis.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MctsStats {
    pub iterations: u32,
    pub nodes: usize,
    pub root_actions: usize,
    pub expanded_root_actions: usize,
    pub terminal_rollouts: u32,
    pub cutoff_rollouts: u32,
    pub simulated_actions: u64,
}

/// Seeded pure UCT agent with uniform random rollouts.
#[derive(Debug)]
pub struct MctsAgent {
    config: MctsConfig,
    rng: StdRng,
    name: String,
    last_stats: Option<MctsStats>,
}

impl MctsAgent {
    /// Construct an agent seeded from the process random-number source.
    pub fn new(config: MctsConfig) -> Result<Self, MctsConfigError> {
        config.validate()?;
        Ok(Self::from_rng(config, rand::make_rng()))
    }

    /// Construct a reproducible agent from an explicit seed.
    pub fn with_seed(config: MctsConfig, seed: u64) -> Result<Self, MctsConfigError> {
        config.validate()?;
        Ok(Self::from_rng(config, StdRng::seed_from_u64(seed)))
    }

    /// Construct the bundled pure UCT baseline.
    pub fn baseline() -> Self {
        Self::new(MctsConfig::baseline()).expect("built-in MCTS baseline config must remain valid")
    }

    /// Complete immutable configuration used by this agent.
    pub fn config(&self) -> &MctsConfig {
        &self.config
    }

    /// Technical statistics from the most recent successful analysis.
    pub fn last_stats(&self) -> Option<MctsStats> {
        self.last_stats
    }

    fn from_rng(config: MctsConfig, rng: StdRng) -> Self {
        let name = format!("MCTS {}", config.versioned_id());
        Self { config, rng, name, last_stats: None }
    }

    fn analyze_position(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: usize,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        let root_player = game.player();
        let mut root_actions = Vec::new();
        root_actions_for(game, input, &mut root_actions)?;
        root_actions.shuffle(&mut self.rng);
        let root_action_count = root_actions.len();
        let mut nodes = Vec::with_capacity(self.config.iterations.get() as usize + 1);
        nodes.push(Node::root(root_player, root_actions));
        let mut search_game = game.clone();
        let action_limit = usize::from(self.config.max_simulation_actions.get());
        let mut path = Vec::with_capacity(action_limit + 1);
        let mut reactions = Vec::with_capacity(action_limit);
        let mut rollout_actions = Vec::new();
        let mut stats = MctsStats {
            iterations: self.config.iterations.get(),
            nodes: 1,
            root_actions: root_action_count,
            expanded_root_actions: 0,
            terminal_rollouts: 0,
            cutoff_rollouts: 0,
            simulated_actions: 0,
        };

        for _ in 0 .. self.config.iterations.get() {
            let outcome = self.run_iteration(
                &mut search_game,
                &mut nodes,
                root_player,
                &mut path,
                &mut reactions,
                &mut rollout_actions,
            )?;
            if outcome.terminal {
                stats.terminal_rollouts += 1;
            } else {
                stats.cutoff_rollouts += 1;
            }
            stats.simulated_actions += u64::from(outcome.simulated_actions);
        }

        stats.nodes = nodes.len();
        stats.expanded_root_actions = nodes[0].children.len();
        let candidates = ranked_root_candidates(&nodes, top_k);
        if candidates.is_empty() {
            return Err(AgentError::Decision("MCTS did not expand a root action".to_owned()));
        }
        self.last_stats = Some(stats);
        Ok(candidates)
    }

    fn run_iteration(
        &mut self, game: &mut Game, nodes: &mut Vec<Node>, root_player: Player,
        path: &mut Vec<usize>, reactions: &mut Vec<Reaction>, rollout_actions: &mut Vec<Action>,
    ) -> Result<IterationOutcome, AgentError> {
        path.clear();
        path.push(0);
        let outcome =
            self.simulate_iteration(game, nodes, root_player, path, reactions, rollout_actions);
        while let Some(reaction) = reactions.pop() {
            game.undo(reaction);
        }
        let outcome = outcome?;
        for &node_index in path.iter() {
            let node = &mut nodes[node_index];
            node.visits += 1;
            node.value_sum += outcome.reward;
        }
        Ok(outcome)
    }

    fn simulate_iteration(
        &mut self, game: &mut Game, nodes: &mut Vec<Node>, root_player: Player,
        path: &mut Vec<usize>, reactions: &mut Vec<Reaction>, rollout_actions: &mut Vec<Action>,
    ) -> Result<IterationOutcome, AgentError> {
        let action_limit = self.config.max_simulation_actions.get();
        let mut node_index = 0;
        let mut simulated_actions = 0_u16;

        loop {
            let game_result = nodes[node_index].game_result;
            if game_result != GameResult::Unfinished {
                return Ok(IterationOutcome::terminal(
                    terminal_reward(game_result, root_player),
                    simulated_actions,
                ));
            }
            if simulated_actions >= action_limit {
                return Ok(IterationOutcome::cutoff(simulated_actions));
            }

            if nodes[node_index].unexpanded_actions.is_none() {
                let mut actions = Vec::new();
                legal_actions(game, &mut actions)?;
                actions.shuffle(&mut self.rng);
                nodes[node_index].unexpanded_actions = Some(actions);
            }

            let action = nodes[node_index]
                .unexpanded_actions
                .as_mut()
                .expect("initialized MCTS action list")
                .pop();
            if let Some(action) = action {
                let reaction = game.action(action).map_err(|message| {
                    AgentError::Decision(format!("generated MCTS action was rejected: {message}"))
                })?;
                reactions.push(reaction);
                simulated_actions += 1;
                let child_index = nodes.len();
                nodes.push(Node::child(action, game.player(), game.result()));
                nodes[node_index].children.push(child_index);
                path.push(child_index);
                return self.rollout(
                    game,
                    root_player,
                    simulated_actions,
                    reactions,
                    rollout_actions,
                );
            }

            let child_index =
                select_child(nodes, node_index, root_player, f64::from(self.config.exploration));
            let action = nodes[child_index]
                .action
                .expect("non-root MCTS node must retain its incoming action");
            let reaction = game.action(action).map_err(|message| {
                AgentError::Decision(format!("selected MCTS action was rejected: {message}"))
            })?;
            reactions.push(reaction);
            simulated_actions += 1;
            node_index = child_index;
            path.push(node_index);
        }
    }

    fn rollout(
        &mut self, game: &mut Game, root_player: Player, mut simulated_actions: u16,
        reactions: &mut Vec<Reaction>, actions: &mut Vec<Action>,
    ) -> Result<IterationOutcome, AgentError> {
        let action_limit = self.config.max_simulation_actions.get();
        loop {
            let game_result = game.result();
            if game_result != GameResult::Unfinished {
                return Ok(IterationOutcome::terminal(
                    terminal_reward(game_result, root_player),
                    simulated_actions,
                ));
            }
            if simulated_actions >= action_limit {
                return Ok(IterationOutcome::cutoff(simulated_actions));
            }

            actions.clear();
            legal_actions(game, actions)?;
            let Some(action) = actions.choose(&mut self.rng).copied() else {
                return Err(AgentError::Decision("MCTS rollout has no legal action".to_owned()));
            };
            let reaction = game.action(action).map_err(|message| {
                AgentError::Decision(format!(
                    "generated MCTS rollout action was rejected: {message}"
                ))
            })?;
            reactions.push(reaction);
            simulated_actions += 1;
        }
    }
}

impl Default for MctsAgent {
    fn default() -> Self {
        Self::baseline()
    }
}

impl Agent for MctsAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn analyze(
        &mut self, game: &Game, input: AgentInput<'_>, top_k: std::num::NonZeroU8,
    ) -> Result<Vec<ScoredAction>, AgentError> {
        self.last_stats = None;
        self.analyze_position(game, input, usize::from(top_k.get()))
    }
}

#[derive(Debug)]
struct Node {
    action: Option<Action>,
    player: Player,
    game_result: GameResult,
    visits: u32,
    value_sum: f64,
    children: Vec<usize>,
    unexpanded_actions: Option<Vec<Action>>,
}

impl Node {
    fn root(player: Player, actions: Vec<Action>) -> Self {
        Self {
            action: None,
            player,
            game_result: GameResult::Unfinished,
            visits: 0,
            value_sum: 0.0,
            children: Vec::new(),
            unexpanded_actions: Some(actions),
        }
    }

    fn child(action: Action, player: Player, game_result: GameResult) -> Self {
        Self {
            action: Some(action),
            player,
            game_result,
            visits: 0,
            value_sum: 0.0,
            children: Vec::new(),
            unexpanded_actions: None,
        }
    }

    fn mean_value(&self) -> f64 {
        if self.visits == 0 { 0.0 } else { self.value_sum / f64::from(self.visits) }
    }
}

#[derive(Debug, Copy, Clone)]
struct IterationOutcome {
    reward: f64,
    terminal: bool,
    simulated_actions: u16,
}

impl IterationOutcome {
    fn terminal(reward: f64, simulated_actions: u16) -> Self {
        Self { reward, terminal: true, simulated_actions }
    }

    fn cutoff(simulated_actions: u16) -> Self {
        Self { reward: 0.0, terminal: false, simulated_actions }
    }
}

fn select_child(nodes: &[Node], node_index: usize, root_player: Player, exploration: f64) -> usize {
    let node = &nodes[node_index];
    let maximize_root = node.player == root_player;
    let parent_log = f64::from(node.visits.max(1)).ln();
    let mut best_child = node.children[0];
    let mut best_score = f64::NEG_INFINITY;

    for &child_index in &node.children {
        let child = &nodes[child_index];
        let mut exploitation = child.mean_value();
        if !maximize_root {
            exploitation = -exploitation;
        }
        let exploration_score = exploration * (parent_log / f64::from(child.visits)).sqrt();
        let score = exploitation + exploration_score;
        if score > best_score {
            best_score = score;
            best_child = child_index;
        }
    }
    best_child
}

fn ranked_root_candidates(nodes: &[Node], top_k: usize) -> Vec<ScoredAction> {
    let root = &nodes[0];
    let mut children = root.children.clone();
    children.sort_by(|left, right| compare_root_children(&nodes[*left], &nodes[*right]));
    children.truncate(top_k);

    let root_visits = root.visits.max(1) as f32;
    let mut candidates = Vec::with_capacity(children.len());
    for child_index in children {
        let child = &nodes[child_index];
        let action = child.action.expect("root child must retain its action");
        let score = child.visits as f32 / root_visits;
        candidates.push(ScoredAction { action, score });
    }
    candidates
}

fn compare_root_children(left: &Node, right: &Node) -> Ordering {
    right.visits.cmp(&left.visits).then_with(|| right.mean_value().total_cmp(&left.mean_value()))
}

fn root_actions_for(
    game: &Game, input: AgentInput<'_>, actions: &mut Vec<Action>,
) -> Result<(), AgentError> {
    match input {
        AgentInput::Placement { area } => placement_actions(game, area, actions),
        AgentInput::Movement { legal_actions } => {
            if legal_actions.is_empty() {
                return Err(AgentError::Decision(
                    "MCTS root has no legal movement action".to_owned(),
                ));
            }
            actions.extend_from_slice(legal_actions);
            Ok(())
        },
    }
}

fn legal_actions(game: &Game, actions: &mut Vec<Action>) -> Result<(), AgentError> {
    match game.phase() {
        Phase::Place => {
            let Some(area) = placement_area(game) else {
                return Err(AgentError::Decision("MCTS placement area is unavailable".to_owned()));
            };
            placement_actions(game, area, actions)
        },
        Phase::Move => {
            game.all_valid_moves(actions);
            actions.push(Action::Pass(game.player()));
            Ok(())
        },
    }
}

fn placement_actions(
    game: &Game, area: PlacementArea, actions: &mut Vec<Action>,
) -> Result<(), AgentError> {
    let pool = match game.player() {
        Player::Red => game.red_pool(),
        Player::Black => game.black_pool(),
    };
    if pool.is_empty() {
        return Err(AgentError::Decision(format!("{} has no pieces left to place", game.player())));
    }

    let mut piece_ids = Vec::<PieceId>::with_capacity(pool.len());
    for piece in pool {
        let piece_id = piece.id();
        if !piece_ids.contains(&piece_id) {
            piece_ids.push(piece_id);
        }
    }

    let mut positions = Vec::new();
    for position in area.positions() {
        if game.board().get(position).is_none() {
            positions.push(position);
        }
    }
    if positions.is_empty() {
        return Err(AgentError::Decision("placement area has no empty point".to_owned()));
    }

    let capacity = piece_ids.len().checked_mul(positions.len()).ok_or_else(|| {
        AgentError::Decision("MCTS placement candidate count overflowed usize".to_owned())
    })?;
    actions.reserve(capacity);
    for piece in piece_ids {
        for &to in &positions {
            actions.push(Action::Place(Place { piece, to }));
        }
    }
    Ok(())
}

fn terminal_reward(result: GameResult, root_player: Player) -> f64 {
    match (result, root_player) {
        (GameResult::RedWin, Player::Red) | (GameResult::BlackWin, Player::Black) => 1.0,
        (GameResult::BlackWin, Player::Red) | (GameResult::RedWin, Player::Black) => -1.0,
        (GameResult::Draw, _) => 0.0,
        (GameResult::Unfinished, _) => {
            unreachable!("terminal reward requires a finished game")
        },
    }
}

fn is_valid_config_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if !(1 ..= 32).contains(&bytes.len()) {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    for byte in &bytes[1 ..] {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-' {
            return false;
        }
    }
    true
}

fn non_zero_u16(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).expect("built-in MCTS u16 values must be non-zero")
}

fn non_zero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("built-in MCTS u32 values must be non-zero")
}
