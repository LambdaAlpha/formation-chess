use std::num::NonZeroU8;
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
use formation_chess_agent::analyze_agent;
use formation_chess_agent::play_agent_turn;
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

struct PlayerRuntime {
    control: ApiControl,
    agent: MinAgent,
    agent_display_name: String,
    selector: ActionSelector,
}

impl PlayerRuntime {
    fn best(control: ApiControl) -> Self {
        let agent = MinAgent::best();
        let agent_display_name = format!("Min AI {}", agent.config().versioned_id());
        Self {
            control,
            agent,
            agent_display_name,
            selector: ActionSelector::new(ActionSelectionPolicy::Best),
        }
    }

    fn info(&self) -> ApiControllerInfo {
        ApiControllerInfo { control: self.control, agent: self.agent_display_name.clone() }
    }
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
        let resolver = NotationResolver::new(self.game.board(), self.game.phase());
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
    game: Game,
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
            red: PlayerRuntime::best(controllers.red),
            black: PlayerRuntime::best(controllers.black),
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
        let action = request.action.to_action(player)?;
        let snapshot = self.game.clone();
        self.game.action(action)?;
        self.history.push(HistoryEntry { game: snapshot, control: ApiControl::Human });
        self.revision += 1;
        Ok(())
    }

    fn legal_actions(
        &self, request: &ApiLegalActionsRequest,
    ) -> Result<ApiLegalActionsResponse, String> {
        let player = self.validate_request(request.revision, &request.side)?;
        let actions = self
            .game
            .valid_moves(request.from[0], request.from[1])
            .into_iter()
            .map(ApiAction::from_action)
            .collect();
        Ok(ApiLegalActionsResponse {
            revision: self.revision,
            side: player_to_str(player),
            actions,
        })
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
        let resolver = NotationResolver::new(snapshot.board(), snapshot.phase());
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
        self.history.push(HistoryEntry { game: snapshot, control: ApiControl::Agent });
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
        let target_index = history_len - undo_count;
        self.game = self.history[target_index].game.clone();
        self.history.truncate(target_index);
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
        .route("/api/legal-actions", post(legal_actions_handler))
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

async fn legal_actions_handler(
    State(state): State<SharedState>, Json(request): Json<ApiLegalActionsRequest>,
) -> ApiResult<ApiLegalActionsResponse> {
    let session = state.session.lock().unwrap();
    session
        .legal_actions(&request)
        .map(Json)
        .map_err(|error| api_error(StatusCode::CONFLICT, error))
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
    let mut red = PlayerRuntime::best(request.controllers.red);
    let mut black = PlayerRuntime::best(request.controllers.black);
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
    let policy = ActionSelectionPolicy::standard_rank_softmax();
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
    use formation_chess_core::piece::Piece;

    use super::*;

    #[test]
    fn state_identifies_the_current_side_when_agent_names_match() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers =
            ApiControllerSettings { red: ApiControl::Agent, black: ApiControl::Agent };
        let session = GameSession::new(game, controllers, 17);
        let state = session.state();

        let expected_agent = format!("Min AI {}", MinAgent::best().config().versioned_id());
        assert_eq!(state.revision, 17, "revision should be preserved");
        assert_eq!(state.current_controller.side, "Red", "side must identify the runtime");
        assert_eq!(state.controllers.red.agent, expected_agent, "red agent should be versioned");
        assert_eq!(
            state.controllers.black.agent, expected_agent,
            "black agent should be versioned"
        );
        assert_eq!(state.current_controller.agent, expected_agent);
        assert_eq!(session.red.selector.top_k(), NonZeroU8::MIN, "step must request top one");
        assert_eq!(session.black.selector.top_k(), NonZeroU8::MIN, "step must request top one");
    }

    #[test]
    fn min_best_agents_complete_standard_random_placement() {
        let mut game = Game::new(GameConfig::default()).expect("default game must be valid");
        let mut red = PlayerRuntime::best(ApiControl::Agent);
        let mut black = PlayerRuntime::best(ApiControl::Agent);

        complete_random_placement(&mut game, &mut red, &mut black)
            .expect("standard placement should complete");

        assert_eq!(game.phase(), Phase::Move, "placement should be complete");
        assert!(game.red_pool().is_empty(), "red pool should be empty");
        assert!(game.black_pool().is_empty(), "black pool should be empty");
    }

    #[test]
    fn prepared_analysis_remains_bound_to_its_original_revision() {
        let game = Game::new(GameConfig::default()).expect("default game must be valid");
        let controllers =
            ApiControllerSettings { red: ApiControl::Human, black: ApiControl::Agent };
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
        let controllers =
            ApiControllerSettings { red: ApiControl::Human, black: ApiControl::Agent };
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

        let undo = ApiUndoRequest { revision: 2, side: "Red".to_owned() };
        session.undo(&undo).expect("paired undo should succeed");

        assert_eq!(session.game.red_pool().len(), 16, "red placement should be reverted");
        assert_eq!(session.game.black_pool().len(), 16, "black reply should be reverted");
        assert_eq!(session.game.board().iter().count(), 0, "board should be empty again");
        assert_eq!(session.revision, 3, "undo should advance the revision");
    }
}
