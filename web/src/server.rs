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
use formation_chess_core::action::Action;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use rust_embed::RustEmbed;

use crate::protocol::*;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

struct AppState {
    game: Mutex<Game>,
    rules_text: String,
}

type SharedState = Arc<AppState>;

pub fn build_app() -> Router {
    let game = Game::new(GameConfig::default()).expect("default config should be valid");
    let rules_text = String::from_utf8(
        Assets::get("rules.zh-Hans.md").expect("rules.zh-Hans.md not embedded").data.into_owned(),
    )
    .expect("rules.zh-Hans.md is not valid UTF-8");
    let state = Arc::new(AppState { game: Mutex::new(game), rules_text });

    Router::new()
        .route("/", get(index_handler))
        .route("/api/state", get(state_handler))
        .route("/api/action", post(action_handler))
        .route("/api/hints", post(hints_handler))
        .route("/api/new", post(new_handler))
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
    let game = state.game.lock().unwrap();
    Json(ApiState::from_game(&game))
}

async fn action_handler(
    State(state): State<SharedState>, Json(request): Json<ApiActionRequest>,
) -> Json<ApiActionResponse> {
    let mut game = state.game.lock().unwrap();
    let player = game.player();
    let action = match request.to_action(player) {
        Ok(a) => a,
        Err(e) => {
            return Json(ApiActionResponse { state: ApiState::from_game(&game), error: Some(e) });
        },
    };
    match game.action(action) {
        Ok(_) => Json(ApiActionResponse { state: ApiState::from_game(&game), error: None }),
        Err(e) => Json(ApiActionResponse { state: ApiState::from_game(&game), error: Some(e) }),
    }
}

async fn hints_handler(
    State(state): State<SharedState>, Json(request): Json<ApiHintsRequest>,
) -> Json<ApiHintsResponse> {
    let game = state.game.lock().unwrap();
    let response = match request {
        ApiHintsRequest::Piece { x, y } => {
            let moves = game.valid_moves(x, y);
            let hints: Vec<ApiMoveHint> = moves
                .iter()
                .map(|a| {
                    let to = action_destination(a);
                    ApiMoveHint { action_type: action_type_str(a).into(), to: [to.0, to.1] }
                })
                .collect();
            ApiHintsResponse { moves: Some(hints), placements: None }
        },
        ApiHintsRequest::White { .. } => {
            let placements: Vec<[u8; 2]> =
                game.valid_white_placements().iter().map(|&(x, y)| [x, y]).collect();
            ApiHintsResponse { moves: None, placements: Some(placements) }
        },
    };
    drop(game);
    Json(response)
}

fn action_destination(action: &Action) -> (u8, u8) {
    match action {
        Action::Move(m) | Action::Capture(m) | Action::Push(m) | Action::Draw(m) => m.to,
        _ => unreachable!("valid_moves only returns move/capture/push/draw"),
    }
}

async fn new_handler(
    State(state): State<SharedState>, Json(request): Json<ApiNewRequest>,
) -> Result<Json<ApiState>, (StatusCode, Json<ApiError>)> {
    let config = request
        .to_game_config()
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e })))?;
    let game =
        Game::new(config).map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e })))?;
    let mut current = state.game.lock().unwrap();
    *current = game;
    let api_state = ApiState::from_game(&current);
    drop(current);
    Ok(Json(api_state))
}

async fn rules_handler(State(state): State<SharedState>) -> Json<ApiRulesResponse> {
    Json(ApiRulesResponse { text: state.rules_text.clone() })
}
