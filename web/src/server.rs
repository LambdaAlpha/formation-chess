use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use formation_chess_agent::ActionSelectionPolicy;
use formation_chess_agent::ActionSelector;
use formation_chess_agent::MinAgent;
use formation_chess_agent::MinConfig;
use formation_chess_agent::analyze_agent;
use formation_chess_agent::play_agent_turn;
use formation_chess_core::action::Reaction;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Player;
use rust_embed::RustEmbed;

use crate::protocol::*;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PlacementMode {
    Standard,
    Random,
}

struct PlayerRuntime {
    control: ApiControl,
    strength: ApiAgentStrength,
    agent: MinAgent,
    agent_display_name: String,
    selector: ActionSelector,
}

impl PlayerRuntime {
    fn new(setting: ApiControllerSetting) -> Self {
        Self::with_placement_mode(setting, PlacementMode::Standard)
    }

    fn with_placement_mode(setting: ApiControllerSetting, placement_mode: PlacementMode) -> Self {
        let control = setting.control();
        let strength = setting.strength();
        let agent = MinAgent::new(min_config(strength, placement_mode))
            .expect("web Min profiles must remain valid");
        let agent_display_name = format!("Min AI {}", agent.config().versioned_id());
        Self {
            control,
            strength,
            agent,
            agent_display_name,
            selector: ActionSelector::new(ActionSelectionPolicy::Best),
        }
    }

    fn info(&self) -> ApiControllerInfo {
        ApiControllerInfo {
            control: self.control,
            strength: self.strength,
            agent: self.agent_display_name.clone(),
        }
    }
}

fn min_config(strength: ApiAgentStrength, placement_mode: PlacementMode) -> MinConfig {
    let mut config = MinConfig::best();
    match strength {
        ApiAgentStrength::VeryWeak => {
            "web-very-weak".clone_into(&mut config.config_id);
            config.movement_search.max_depth = non_zero_u8(2);
            config.movement_search.max_nodes = non_zero_u32(90);
            config.movement_search.opponent_width = non_zero_u8(3);
            config.movement_search.response_width = non_zero_u8(2);
            config.evaluation.movement_weights.vital_safety = 360;
            config.evaluation.movement_weights.effective_abilities = 80;
            config.evaluation.movement_weights.formation_effects = 60;
            config.evaluation.movement_weights.control = 80;
            config.evaluation.movement_weights.mobility = 100;
            config.evaluation.movement_weights.action_effects = 260;
            config.evaluation.movement_weights.material = 80;
            config.evaluation.movement_weights.interactions = 80;
        },
        ApiAgentStrength::Weak => {
            "web-weak".clone_into(&mut config.config_id);
            config.movement_search.max_depth = non_zero_u8(2);
            config.movement_search.max_nodes = non_zero_u32(450);
            config.movement_search.opponent_width = non_zero_u8(4);
            config.movement_search.response_width = non_zero_u8(3);
        },
        ApiAgentStrength::Medium => {
            "web-medium".clone_into(&mut config.config_id);
            config.movement_search.max_nodes = non_zero_u32(750);
            config.movement_search.opponent_width = non_zero_u8(6);
            config.movement_search.response_width = non_zero_u8(4);
        },
    }

    if placement_mode == PlacementMode::Random {
        match strength {
            ApiAgentStrength::VeryWeak => {
                config.placement_search.max_nodes = non_zero_u32(1_500);
                config.placement_search.root_width = non_zero_u8(12);
                config.placement_search.opponent_width = non_zero_u8(4);
                config.placement_search.response_width = non_zero_u8(2);
            },
            ApiAgentStrength::Weak => {
                config.placement_search.max_nodes = non_zero_u32(3_000);
                config.placement_search.root_width = non_zero_u8(20);
                config.placement_search.opponent_width = non_zero_u8(6);
                config.placement_search.response_width = non_zero_u8(3);
            },
            ApiAgentStrength::Medium => {},
        }
        config.config_id.push_str("-random");
    }

    config
}

fn non_zero_u8(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("web Min profile value must be non-zero")
}

fn non_zero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("web Min profile value must be non-zero")
}

struct PreparedAgentAnalysis {
    game: Game,
    revision: u64,
    side: String,
    agent: MinAgent,
    agent_display_name: String,
    top_k: NonZeroU8,
}

impl PreparedAgentAnalysis {
    fn run(mut self) -> Result<ApiAgentAnalyzeResponse, String> {
        let analysis = analyze_agent(&self.game, &mut self.agent, self.top_k)
            .map_err(|error| error.to_string())?;
        let resolver = NotationResolver::new(&self.game);
        let candidates = analysis
            .candidates
            .into_iter()
            .map(|candidate| ApiAgentCandidate {
                action: ApiAction::from_action(candidate.action),
                notation: resolver.fmt_action(&candidate.action),
                score: candidate.score,
            })
            .collect();

        Ok(ApiAgentAnalyzeResponse {
            revision: self.revision,
            side: self.side,
            agent: self.agent_display_name,
            candidates,
        })
    }
}

struct HistoryEntry {
    reaction: Reaction,
    control: ApiControl,
}

struct GameSession {
    game: Game,
    history: Vec<HistoryEntry>,
    revision: u64,
    red: PlayerRuntime,
    black: PlayerRuntime,
}

impl GameSession {
    fn standard() -> Self {
        let game = Game::new(GameConfig::default()).expect("default config should be valid");
        Self::new(game, ApiControllerSettings::default(), 0)
    }

    fn new(game: Game, controllers: ApiControllerSettings, revision: u64) -> Self {
        Self {
            game,
            history: Vec::new(),
            revision,
            red: PlayerRuntime::new(controllers.red),
            black: PlayerRuntime::new(controllers.black),
        }
    }

    fn state(&self) -> ApiState {
        let controllers = ApiControllers { red: self.red.info(), black: self.black.info() };
        ApiState::from_game(&self.game, self.revision, !self.history.is_empty(), controllers)
    }

    fn control(&self, player: Player) -> ApiControl {
        match player {
            Player::Red => self.red.control,
            Player::Black => self.black.control,
        }
    }

    fn validate_request(&self, revision: u64, side: &str) -> Result<Player, String> {
        if revision != self.revision {
            return Err(format!("棋局已更新：请求版本为 {revision}，当前版本为 {}", self.revision));
        }
        let player = parse_player(side)?;
        if player != self.game.player() {
            return Err(format!(
                "当前轮到 {} 方，不是 {} 方",
                player_to_str(self.game.player()),
                side
            ));
        }
        Ok(player)
    }

    fn apply_human_action(&mut self, request: &ApiActionRequest) -> Result<(), String> {
        let player = self.validate_request(request.revision, &request.side)?;
        if self.control(player) != ApiControl::Human {
            return Err(format!("{} 方当前由 AI 控制", request.side));
        }
        let action = request.action.to_action()?;
        let reaction = self.game.action(action)?;
        self.history.push(HistoryEntry { reaction, control: ApiControl::Human });
        self.revision += 1;
        Ok(())
    }

    fn prepare_analysis(
        &self, request: &ApiAgentAnalyzeRequest,
    ) -> Result<PreparedAgentAnalysis, String> {
        let player = self.validate_request(request.revision, &request.side)?;
        let top_k = NonZeroU8::new(request.top_k)
            .ok_or_else(|| "top_k must be greater than zero".to_owned())?;
        let runtime = match player {
            Player::Red => &self.red,
            Player::Black => &self.black,
        };

        Ok(PreparedAgentAnalysis {
            game: self.game.clone(),
            revision: self.revision,
            side: player_to_str(player),
            agent: runtime.agent.clone(),
            agent_display_name: runtime.agent_display_name.clone(),
            top_k,
        })
    }

    fn play_agent_step(
        &mut self, request: &ApiAgentStepRequest,
    ) -> Result<ApiAgentCandidate, String> {
        let player = self.validate_request(request.revision, &request.side)?;
        if self.control(player) != ApiControl::Agent {
            return Err(format!("{} 方当前由人类控制", request.side));
        }

        let snapshot = self.game.clone();
        let resolver = NotationResolver::new(&snapshot);
        let turn = match player {
            Player::Red => {
                play_agent_turn(&mut self.game, &mut self.red.agent, &mut self.red.selector)
            },
            Player::Black => {
                play_agent_turn(&mut self.game, &mut self.black.agent, &mut self.black.selector)
            },
        }
        .map_err(|error| error.to_string())?;
        let played = ApiAgentCandidate {
            action: ApiAction::from_action(turn.action),
            notation: resolver.fmt_action(&turn.action),
            score: turn.score,
        };
        self.history.push(HistoryEntry { reaction: turn.reaction, control: ApiControl::Agent });
        self.revision += 1;
        Ok(played)
    }

    fn undo(&mut self, request: &ApiUndoRequest) -> Result<(), String> {
        self.validate_request(request.revision, &request.side)?;
        let history_len = self.history.len();
        if history_len == 0 {
            return Err("没有可悔棋的操作".to_owned());
        }

        let undo_count = if history_len >= 2
            && self.history[history_len - 1].control == ApiControl::Agent
            && self.history[history_len - 2].control == ApiControl::Human
        {
            2
        } else {
            1
        };
        for _ in 0 .. undo_count {
            let entry = self.history.pop().expect("undo count is bounded by history length");
            self.game.undo(entry.reaction);
        }
        self.revision += 1;
        Ok(())
    }
}

struct AppState {
    session: Mutex<GameSession>,
    rules_text: String,
}

type SharedState = Arc<AppState>;
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiError>)>;

pub fn build_app() -> Router {
    let rules_text = String::from_utf8(
        Assets::get("rules.zh-Hans.md").expect("rules.zh-Hans.md not embedded").data.into_owned(),
    )
    .expect("rules.zh-Hans.md is not valid UTF-8");
    let state = Arc::new(AppState { session: Mutex::new(GameSession::standard()), rules_text });

    Router::new()
        .route("/", get(index_handler))
        .route("/api/state", get(state_handler))
        .route("/api/action", post(action_handler))
        .route("/api/agent/analyze", post(agent_analyze_handler))
        .route("/api/agent/step", post(agent_step_handler))
        .route("/api/new", post(new_handler))
        .route("/api/undo", post(undo_handler))
        .route("/api/rules", get(rules_handler))
        .route("/{*path}", get(asset_handler))
        .with_state(state)
}

async fn index_handler() -> impl IntoResponse {
    serve_asset("index.html")
}

async fn asset_handler(Path(path): Path<String>) -> impl IntoResponse {
    serve_asset(&path)
}

fn serve_asset(path: &str) -> Response {
    match Assets::get(path) {
        Some(file) => Response::builder()
            .header(header::CONTENT_TYPE, mime_type(path))
            .body(Body::from(file.data.into_owned()))
            .unwrap(),
        None => {
            Response::builder().status(StatusCode::NOT_FOUND).body(Body::from("not found")).unwrap()
        },
    }
}

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

async fn state_handler(State(state): State<SharedState>) -> Json<ApiState> {
    let session = state.session.lock().unwrap();
    Json(session.state())
}

async fn action_handler(
    State(state): State<SharedState>, Json(request): Json<ApiActionRequest>,
) -> Json<ApiActionResponse> {
    let mut session = state.session.lock().unwrap();
    let error = session.apply_human_action(&request).err();
    Json(ApiActionResponse { state: session.state(), error })
}

async fn agent_analyze_handler(
    State(state): State<SharedState>, Json(request): Json<ApiAgentAnalyzeRequest>,
) -> ApiResult<ApiAgentAnalyzeResponse> {
    let prepared = {
        let session = state.session.lock().unwrap();
        session.prepare_analysis(&request)
    }
    .map_err(|error| api_error(StatusCode::UNPROCESSABLE_ENTITY, error))?;

    tokio::task::spawn_blocking(move || prepared.run())
        .await
        .map_err(|error| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("AI analysis task failed: {error}"),
            )
        })?
        .map(Json)
        .map_err(|error| api_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

async fn agent_step_handler(
    State(state): State<SharedState>, Json(request): Json<ApiAgentStepRequest>,
) -> Json<ApiAgentStepResponse> {
    let mut session = state.session.lock().unwrap();
    let result = session.play_agent_step(&request);
    let (played, error) = match result {
        Ok(played) => (Some(played), None),
        Err(error) => (None, Some(error)),
    };
    Json(ApiAgentStepResponse { state: session.state(), played, error })
}

async fn new_handler(
    State(state): State<SharedState>, Json(request): Json<ApiNewRequest>,
) -> ApiResult<ApiState> {
    let mut session = state.session.lock().unwrap();
    session
        .validate_request(request.revision, &request.side)
        .map_err(|error| api_error(StatusCode::CONFLICT, error))?;
    if request.random_placement && request.notation.is_some() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "random placement cannot be combined with notation".to_owned(),
        ));
    }

    let config =
        request.to_game_config().map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    let mut game = Game::new(config).map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    let placement_mode =
        if request.random_placement { PlacementMode::Random } else { PlacementMode::Standard };
    let mut red = PlayerRuntime::with_placement_mode(request.controllers.red, placement_mode);
    let mut black = PlayerRuntime::with_placement_mode(request.controllers.black, placement_mode);
    if request.random_placement {
        complete_random_placement(&mut game, &mut red, &mut black)
            .map_err(|error| api_error(StatusCode::UNPROCESSABLE_ENTITY, error))?;
    }

    let revision = session.revision + 1;
    *session = GameSession { game, history: Vec::new(), revision, red, black };
    Ok(Json(session.state()))
}

fn complete_random_placement(
    game: &mut Game, red: &mut PlayerRuntime, black: &mut PlayerRuntime,
) -> Result<(), String> {
    let policy = ActionSelectionPolicy::standard_score_softmax();
    let mut red_selector = ActionSelector::new(policy);
    let mut black_selector = ActionSelector::new(policy);
    let placement_count = game.red_pool().len() + game.black_pool().len();
    for _ in 0 .. placement_count {
        if game.phase() == Phase::Move {
            return Ok(());
        }
        let result = match game.player() {
            Player::Red => play_agent_turn(game, &mut red.agent, &mut red_selector),
            Player::Black => play_agent_turn(game, &mut black.agent, &mut black_selector),
        };
        result.map_err(|error| error.to_string())?;
    }

    if game.phase() == Phase::Move {
        Ok(())
    } else {
        Err("agents did not finish the placement phase".to_owned())
    }
}

async fn undo_handler(
    State(state): State<SharedState>, Json(request): Json<ApiUndoRequest>,
) -> Json<ApiActionResponse> {
    let mut session = state.session.lock().unwrap();
    let error = session.undo(&request).err();
    Json(ApiActionResponse { state: session.state(), error })
}

async fn rules_handler(State(state): State<SharedState>) -> Json<ApiRulesResponse> {
    Json(ApiRulesResponse { text: state.rules_text.clone() })
}

fn api_error(status: StatusCode, error: String) -> (StatusCode, Json<ApiError>) {
    (status, Json(ApiError { error }))
}

#[cfg(test)]
mod tests {
    use formation_chess_core::action::GameResult;
    use formation_chess_core::action::PoolChange;
    use formation_chess_core::board::Board;
    use formation_chess_core::piece::Piece;

    use super::*;

    #[test]
    fn state_identifies_the_current_side_when_agent_names_match() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
            black: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
        };
        let session = GameSession::new(game, controllers, 17);
        let state = session.state();

        let expected_agent = "Min AI web-medium-v1";
        assert_eq!(state.revision, 17, "revision should be preserved");
        assert_eq!(state.current_controller.side, "Red", "side must identify the runtime");
        assert_eq!(state.controllers.red.agent, expected_agent, "red agent should be versioned");
        assert_eq!(
            state.controllers.black.agent, expected_agent,
            "black agent should be versioned"
        );
        assert_eq!(state.current_controller.agent, expected_agent);
        assert!(state.legal_actions.is_empty(), "placement has no board actions");
        assert_eq!(session.red.selector.top_k(), NonZeroU8::MIN, "step must request top one");
        assert_eq!(session.black.selector.top_k(), NonZeroU8::MIN, "step must request top one");
    }

    #[test]
    fn different_strengths_create_distinct_agent_profiles() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::VeryWeak),
            black: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Weak),
        };
        let session = GameSession::new(game, controllers, 0);
        let state = session.state();

        assert_eq!(state.controllers.red.strength, ApiAgentStrength::VeryWeak);
        assert_eq!(state.controllers.black.strength, ApiAgentStrength::Weak);
        assert_eq!(session.red.agent.config().config_id, "web-very-weak");
        assert_eq!(session.black.agent.config().config_id, "web-weak");
        assert_eq!(session.red.agent.config().movement_search.max_nodes.get(), 90);
        assert_eq!(session.black.agent.config().movement_search.max_nodes.get(), 450);
        assert_eq!(session.red.agent.config().placement_search.max_nodes.get(), 6_000);
        assert_eq!(session.black.agent.config().placement_search.max_nodes.get(), 6_000);
        assert_ne!(state.controllers.red.agent, state.controllers.black.agent);
    }

    #[test]
    fn random_placement_uses_light_agent_profiles() {
        let red_setting = ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::VeryWeak);
        let black_setting = ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium);
        let red = PlayerRuntime::with_placement_mode(red_setting, PlacementMode::Random);
        let black = PlayerRuntime::with_placement_mode(black_setting, PlacementMode::Random);

        assert_eq!(red.agent.config().config_id, "web-very-weak-random");
        assert_eq!(red.agent.config().placement_search.max_nodes.get(), 1_500);
        assert_eq!(red.agent.config().movement_search.max_nodes.get(), 90);
        assert_eq!(black.agent.config().config_id, "web-medium-random");
        assert_eq!(black.agent.config().placement_search.max_nodes.get(), 6_000);
    }

    #[test]
    fn state_exposes_all_legal_actions() {
        let mut board = Board::new(5, 5);
        board[(0, 4)] = Some(Piece::RED_GENERAL);
        board[(4, 0)] = Some(Piece::BLACK_GENERAL);
        board[(2, 2)] = Some(Piece::RED_WIND);
        board[(2, 3)] = Some(Piece::RED_PAWN);
        let game = Game::new(GameConfig {
            player: Player::Red,
            board,
            red_pool: Vec::new(),
            black_pool: Vec::new(),
            result: GameResult::Unfinished,
        })
        .expect("valid movement game");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
            black: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
        };
        let session = GameSession::new(game, controllers, 3);
        let state = session.state();

        assert!(
            state
                .legal_actions
                .iter()
                .any(|action| matches!(action, ApiAction::Pull { from: [2, 2], to: [2, 1] }))
        );
        assert!(
            state
                .legal_actions
                .iter()
                .any(|action| matches!(action, ApiAction::Resign { at: [0, 4] }))
        );
        assert!(
            state
                .legal_actions
                .iter()
                .any(|action| { matches!(action, ApiAction::Move { from: [0, 4], .. }) }),
            "state should include actions from every selectable origin"
        );
    }

    #[test]
    fn targeted_resign_remains_available_during_placement() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
            black: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
        };
        let mut session = GameSession::new(game, controllers, 0);
        let request = ApiActionRequest {
            revision: 0,
            side: "Red".to_owned(),
            action: ApiAction::Resign { at: [0, 0] },
        };

        session.apply_human_action(&request).expect("placement resignation should succeed");
        assert_eq!(session.game.result(), GameResult::BlackWin);
    }

    #[test]
    #[ignore = "time-consuming"]
    fn min_best_agents_complete_standard_random_placement() {
        let mut game = Game::new(GameConfig::default()).expect("default game must be valid");
        let mut red = PlayerRuntime::with_placement_mode(
            ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
            PlacementMode::Random,
        );
        let mut black = PlayerRuntime::with_placement_mode(
            ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
            PlacementMode::Random,
        );

        complete_random_placement(&mut game, &mut red, &mut black)
            .expect("standard placement should complete");

        assert_eq!(game.phase(), Phase::Move, "placement should be complete");
        assert!(game.red_pool().is_empty(), "red pool should be empty");
        assert!(game.black_pool().is_empty(), "black pool should be empty");
    }

    #[test]
    fn prepared_analysis_remains_bound_to_its_original_revision() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
            black: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
        };
        let mut session = GameSession::new(game, controllers, 0);
        let request = ApiAgentAnalyzeRequest { revision: 0, side: "Red".to_owned(), top_k: 1 };
        let prepared = session.prepare_analysis(&request).expect("prepare red analysis snapshot");

        let action = ApiActionRequest {
            revision: 0,
            side: "Red".to_owned(),
            action: ApiAction::Place {
                piece: ApiPieceRef::from_id(Piece::RED_GENERAL.id()),
                to: [0, 5],
            },
        };
        session.apply_human_action(&action).expect("red placement should succeed");
        let response = prepared.run().expect("snapshot analysis should complete");

        assert_eq!(response.revision, 0, "analysis must retain its snapshot revision");
        assert_eq!(session.revision, 1, "the live session should advance independently");
        assert_eq!(response.side, "Red");
        assert_eq!(response.candidates.len(), 1);
    }

    #[test]
    fn undo_reverts_a_human_action_and_automatic_reply() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers = ApiControllerSettings {
            red: ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium),
            black: ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium),
        };
        let mut session = GameSession::new(game, controllers, 0);
        let action = ApiActionRequest {
            revision: 0,
            side: "Red".to_owned(),
            action: ApiAction::Place {
                piece: ApiPieceRef::from_id(Piece::RED_GENERAL.id()),
                to: [0, 5],
            },
        };
        session.apply_human_action(&action).expect("red placement should succeed");
        let step = ApiAgentStepRequest { revision: 1, side: "Black".to_owned() };
        session.play_agent_step(&step).expect("black reply should succeed");

        assert_eq!(session.game.red_pool().len(), 15, "red should have placed once");
        assert_eq!(session.game.black_pool().len(), 15, "black should have placed once");
        assert_eq!(session.history.len(), 2, "both reactions should be recorded");
        assert!(
            matches!(session.history[0].reaction.pool_change, PoolChange::Removed { .. }),
            "human placement reaction should be recorded"
        );
        assert!(
            matches!(session.history[1].reaction.pool_change, PoolChange::Removed { .. }),
            "agent placement reaction should be recorded"
        );

        let undo = ApiUndoRequest { revision: 2, side: "Red".to_owned() };
        session.undo(&undo).expect("paired undo should succeed");

        assert_eq!(session.game.red_pool().len(), 16, "red placement should be reverted");
        assert_eq!(session.game.black_pool().len(), 16, "black reply should be reverted");
        assert_eq!(session.game.board().iter().count(), 0, "board should be empty again");
        assert!(session.history.is_empty(), "undone reactions should leave history");
        assert_eq!(session.revision, 3, "undo should advance the revision");
    }
}
