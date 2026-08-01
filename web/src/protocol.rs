use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Color;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPieceRef {
    pub name: char,
    pub color: String,
}

impl ApiPieceRef {
    pub fn from_piece(piece: Piece) -> Self {
        Self { name: piece.name, color: color_to_str(piece.color) }
    }

    pub fn to_piece(&self) -> Result<Piece, String> {
        let color = parse_color(&self.color)?;
        Piece::lookup(self.name, color)
            .ok_or_else(|| format!("unknown piece: {} {:?}", self.name, self.color))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiPiece {
    pub name: char,
    pub color: String,
    pub formation: u8,
}

impl ApiPiece {
    pub fn from_piece(piece: Piece) -> Self {
        Self {
            name: piece.name,
            color: color_to_str(piece.color),
            formation: piece.formation.points,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiBoard {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<Vec<Option<ApiPiece>>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiControl {
    Human,
    Agent,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ApiControllerSettings {
    #[serde(default = "default_red_control")]
    pub red: ApiControl,
    #[serde(default = "default_black_control")]
    pub black: ApiControl,
}

impl Default for ApiControllerSettings {
    fn default() -> Self {
        Self { red: default_red_control(), black: default_black_control() }
    }
}

fn default_red_control() -> ApiControl {
    ApiControl::Human
}

fn default_black_control() -> ApiControl {
    ApiControl::Agent
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiControllerInfo {
    pub control: ApiControl,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiControllers {
    pub red: ApiControllerInfo,
    pub black: ApiControllerInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiCurrentController {
    pub side: String,
    pub control: ApiControl,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiState {
    pub revision: u64,
    pub player: String,
    pub phase: String,
    pub result: String,
    pub controllers: ApiControllers,
    pub current_controller: ApiCurrentController,
    pub board: ApiBoard,
    pub red_pool: Vec<ApiPiece>,
    pub black_pool: Vec<ApiPiece>,
    pub white_pool: u8,
    pub can_human_act: bool,
    pub can_agent_step: bool,
    pub can_undo: bool,
}

impl ApiState {
    pub fn from_game(
        game: &Game, revision: u64, can_undo: bool, controllers: ApiControllers,
    ) -> Self {
        let board = game.board();
        let cells = (0 .. board.height())
            .map(|y| {
                (0 .. board.width()).map(|x| board.get((x, y)).map(ApiPiece::from_piece)).collect()
            })
            .collect();
        let current = match game.player() {
            Player::Red => &controllers.red,
            Player::Black => &controllers.black,
        };
        let current_controller = ApiCurrentController {
            side: player_to_str(game.player()),
            control: current.control,
            agent: current.agent.clone(),
        };
        let unfinished = game.result() == GameResult::Unfinished;
        let current_control = current_controller.control;

        Self {
            revision,
            player: player_to_str(game.player()),
            phase: phase_to_str(game.phase()),
            result: result_to_str(game.result()),
            controllers,
            current_controller,
            board: ApiBoard { width: board.width(), height: board.height(), cells },
            red_pool: game.red_pool().iter().copied().map(ApiPiece::from_piece).collect(),
            black_pool: game.black_pool().iter().copied().map(ApiPiece::from_piece).collect(),
            white_pool: game.white_pool(),
            can_human_act: unfinished && current_control == ApiControl::Human,
            can_agent_step: unfinished && current_control == ApiControl::Agent,
            can_undo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiAction {
    Place { piece: ApiPieceRef, to: [u8; 2] },
    Move { from: [u8; 2], to: [u8; 2] },
    Capture { from: [u8; 2], to: [u8; 2] },
    Push { from: [u8; 2], to: [u8; 2] },
    Draw { from: [u8; 2], to: [u8; 2] },
    Divide { from: [u8; 2], to: [u8; 2] },
    Pass,
    Resign,
}

impl ApiAction {
    pub fn from_action(action: Action) -> Self {
        match action {
            Action::Place(place) => {
                Self::Place { piece: ApiPieceRef::from_piece(place.piece), to: pair(place.to) }
            },
            Action::Move(move_) => Self::Move { from: pair(move_.from), to: pair(move_.to) },
            Action::Capture(move_) => Self::Capture { from: pair(move_.from), to: pair(move_.to) },
            Action::Push(move_) => Self::Push { from: pair(move_.from), to: pair(move_.to) },
            Action::Draw(move_) => Self::Draw { from: pair(move_.from), to: pair(move_.to) },
            Action::Divide(move_) => Self::Divide { from: pair(move_.from), to: pair(move_.to) },
            Action::Pass(_) => Self::Pass,
            Action::Resign(_) => Self::Resign,
        }
    }

    pub fn to_action(&self, current_player: Player) -> Result<Action, String> {
        match self {
            Self::Place { piece, to } => {
                Ok(Action::Place(Place { piece: piece.to_piece()?, to: tuple(*to) }))
            },
            Self::Move { from, to } => {
                Ok(Action::Move(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Capture { from, to } => {
                Ok(Action::Capture(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Push { from, to } => {
                Ok(Action::Push(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Draw { from, to } => {
                Ok(Action::Draw(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Divide { from, to } => {
                Ok(Action::Divide(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Pass => Ok(Action::Pass(current_player)),
            Self::Resign => Ok(Action::Resign(current_player)),
        }
    }
}

fn pair((x, y): (u8, u8)) -> [u8; 2] {
    [x, y]
}

fn tuple([x, y]: [u8; 2]) -> (u8, u8) {
    (x, y)
}

#[derive(Debug, Deserialize)]
pub struct ApiActionRequest {
    pub revision: u64,
    pub side: String,
    pub action: ApiAction,
}

#[derive(Debug, Serialize)]
pub struct ApiActionResponse {
    #[serde(flatten)]
    pub state: ApiState,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiLegalActionsRequest {
    pub revision: u64,
    pub side: String,
    pub from: [u8; 2],
}

#[derive(Debug, Serialize)]
pub struct ApiLegalActionsResponse {
    pub revision: u64,
    pub side: String,
    pub actions: Vec<ApiAction>,
}

#[derive(Debug, Deserialize)]
pub struct ApiAgentAnalyzeRequest {
    pub revision: u64,
    pub side: String,
    pub top_k: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiAgentCandidate {
    pub action: ApiAction,
    pub notation: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct ApiAgentAnalyzeResponse {
    pub revision: u64,
    pub side: String,
    pub agent: String,
    pub candidates: Vec<ApiAgentCandidate>,
}

#[derive(Debug, Deserialize)]
pub struct ApiAgentStepRequest {
    pub revision: u64,
    pub side: String,
}

#[derive(Debug, Serialize)]
pub struct ApiAgentStepResponse {
    #[serde(flatten)]
    pub state: ApiState,
    pub played: Option<ApiAgentCandidate>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiUndoRequest {
    pub revision: u64,
    pub side: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiNewBoard {
    pub width: u8,
    pub height: u8,
    #[serde(default)]
    pub cells: Option<Vec<Vec<Option<ApiPieceRef>>>>,
}

#[derive(Debug, Deserialize)]
pub struct ApiNewRequest {
    pub revision: u64,
    pub side: String,
    #[serde(default)]
    pub controllers: ApiControllerSettings,
    #[serde(default)]
    pub random_placement: bool,
    #[serde(default)]
    pub notation: Option<String>,
    #[serde(default)]
    pub board: Option<ApiNewBoard>,
    #[serde(default)]
    pub red_pool: Vec<String>,
    #[serde(default)]
    pub black_pool: Vec<String>,
    #[serde(default)]
    pub white_pool: u8,
    #[serde(default)]
    pub player: Option<String>,
}

impl ApiNewRequest {
    pub fn to_game_config(&self) -> Result<GameConfig, String> {
        if let Some(notation) = &self.notation {
            let config =
                notation.parse().map_err(|error: String| format!("invalid notation: {error}"))?;
            return Ok(config);
        }

        let api_board = self
            .board
            .as_ref()
            .ok_or_else(|| "board is required when notation is not provided".to_owned())?;

        if api_board.width == 0 || api_board.width > 16 {
            return Err(format!("board width must be 1..=16, got {}", api_board.width));
        }
        if api_board.height == 0 || api_board.height > 16 {
            return Err(format!("board height must be 1..=16, got {}", api_board.height));
        }

        let mut board = Board::new(api_board.width, api_board.height);
        let has_cells = api_board.cells.is_some();
        if let Some(cells) = &api_board.cells {
            for (y, row) in cells.iter().enumerate() {
                let y = y as u8;
                if y >= api_board.height {
                    break;
                }
                for (x, cell) in row.iter().enumerate() {
                    let x = x as u8;
                    if x >= api_board.width {
                        break;
                    }
                    if let Some(piece) = cell {
                        board[(x, y)] = Some(piece.to_piece()?);
                    }
                }
            }
        }

        let use_standard_pools =
            !has_cells && self.red_pool.is_empty() && self.black_pool.is_empty();
        let red_pool = if use_standard_pools {
            Piece::RED_PLAYER_PIECES.to_vec()
        } else {
            parse_pool(&self.red_pool, Color::Red)?
        };
        let black_pool = if use_standard_pools {
            Piece::BLACK_PLAYER_PIECES.to_vec()
        } else {
            parse_pool(&self.black_pool, Color::Black)?
        };
        let player = match self.player.as_deref() {
            None | Some("Red") => Player::Red,
            Some("Black") => Player::Black,
            Some(value) => return Err(format!("invalid player: {value}")),
        };

        Ok(GameConfig {
            player,
            board,
            red_pool,
            black_pool,
            white: Piece::WHITE,
            white_pool: self.white_pool,
            result: GameResult::Unfinished,
        })
    }
}

fn parse_pool(names: &[String], color: Color) -> Result<Vec<Piece>, String> {
    names
        .iter()
        .map(|name| {
            let piece_name =
                name.chars().next().ok_or_else(|| format!("empty piece name: {name}"))?;
            Piece::lookup(piece_name, color)
                .ok_or_else(|| format!("unknown {} piece: {name}", color_to_str(color)))
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct ApiRulesResponse {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub fn color_to_str(color: Color) -> String {
    match color {
        Color::Red => "Red".to_owned(),
        Color::Black => "Black".to_owned(),
        Color::White => "White".to_owned(),
    }
}

pub fn parse_color(value: &str) -> Result<Color, String> {
    match value {
        "Red" => Ok(Color::Red),
        "Black" => Ok(Color::Black),
        "White" => Ok(Color::White),
        _ => Err(format!("unknown color: {value}")),
    }
}

pub fn player_to_str(player: Player) -> String {
    match player {
        Player::Red => "Red".to_owned(),
        Player::Black => "Black".to_owned(),
    }
}

pub fn parse_player(value: &str) -> Result<Player, String> {
    match value {
        "Red" => Ok(Player::Red),
        "Black" => Ok(Player::Black),
        _ => Err(format!("unknown side: {value}")),
    }
}

pub fn phase_to_str(phase: Phase) -> String {
    match phase {
        Phase::Place => "placement".to_owned(),
        Phase::Move => "movement".to_owned(),
    }
}

pub fn result_to_str(result: GameResult) -> String {
    match result {
        GameResult::Unfinished => "Unfinished".to_owned(),
        GameResult::RedWin => "RedWin".to_owned(),
        GameResult::BlackWin => "BlackWin".to_owned(),
        GameResult::Draw => "Draw".to_owned(),
    }
}
