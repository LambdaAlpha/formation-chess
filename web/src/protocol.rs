use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Color;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiPieceRef {
    pub name: char,
    pub color: String,
}

impl ApiPieceRef {
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
        ApiPiece {
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

#[derive(Debug, Clone, Serialize)]
pub struct ApiState {
    pub player: String,
    pub result: String,
    pub board: ApiBoard,
    pub red_pool: Vec<ApiPiece>,
    pub black_pool: Vec<ApiPiece>,
    pub white_pool: u8,
    pub can_undo: bool,
}

impl ApiState {
    pub fn from_game(game: &Game, can_undo: bool) -> Self {
        let board = game.board();
        let cells: Vec<Vec<Option<ApiPiece>>> = (0 .. board.height())
            .map(|y| {
                (0 .. board.width()).map(|x| board.get((x, y)).map(ApiPiece::from_piece)).collect()
            })
            .collect();

        ApiState {
            player: player_to_str(game.player()),
            result: result_to_str(game.result()),
            board: ApiBoard { width: board.width(), height: board.height(), cells },
            red_pool: game.red_pool().iter().copied().map(ApiPiece::from_piece).collect(),
            black_pool: game.black_pool().iter().copied().map(ApiPiece::from_piece).collect(),
            white_pool: game.white_pool(),
            can_undo,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiActionResponse {
    #[serde(flatten)]
    pub state: ApiState,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiActionRequest {
    Place { piece: ApiPieceRef, to: [u8; 2] },
    Move { from: [u8; 2], to: [u8; 2] },
    Capture { from: [u8; 2], to: [u8; 2] },
    Push { from: [u8; 2], to: [u8; 2] },
    Draw { from: [u8; 2], to: [u8; 2] },
    Leave { from: [u8; 2], to: [u8; 2] },
    Pass,
    Resign,
}

impl ApiActionRequest {
    pub fn to_action(&self, current_player: Player) -> Result<Action, String> {
        match self {
            ApiActionRequest::Place { piece, to } => {
                Ok(Action::Place(Place { piece: piece.to_piece()?, to: (to[0], to[1]) }))
            },
            ApiActionRequest::Move { from, to } => {
                Ok(Action::Move(Move { from: (from[0], from[1]), to: (to[0], to[1]) }))
            },
            ApiActionRequest::Capture { from, to } => {
                Ok(Action::Capture(Move { from: (from[0], from[1]), to: (to[0], to[1]) }))
            },
            ApiActionRequest::Push { from, to } => {
                Ok(Action::Push(Move { from: (from[0], from[1]), to: (to[0], to[1]) }))
            },
            ApiActionRequest::Draw { from, to } => {
                Ok(Action::Draw(Move { from: (from[0], from[1]), to: (to[0], to[1]) }))
            },
            ApiActionRequest::Leave { from, to } => {
                Ok(Action::Leave(Move { from: (from[0], from[1]), to: (to[0], to[1]) }))
            },
            ApiActionRequest::Pass => Ok(Action::Pass(current_player)),
            ApiActionRequest::Resign => Ok(Action::Resign(current_player)),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiMoveHint {
    #[serde(rename = "type")]
    pub action_type: String,
    pub to: [u8; 2],
}

#[derive(Debug, Serialize)]
pub struct ApiHintsResponse {
    pub moves: Option<Vec<ApiMoveHint>>,
    pub placements: Option<Vec<[u8; 2]>>,
}

#[derive(Debug, Deserialize)]
pub struct ApiHintsRequest {
    pub x: u8,
    pub y: u8,
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
            let config: GameConfig =
                notation.parse().map_err(|e: String| format!("invalid notation: {e}"))?;
            return Ok(config);
        }

        let api_board = self
            .board
            .as_ref()
            .ok_or_else(|| "board is required when notation is not provided".to_string())?;

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
                    if let Some(piece_ref) = cell {
                        board[(x, y)] = Some(piece_ref.to_piece()?);
                    }
                }
            }
        }

        let red_pool = if !has_cells && self.red_pool.is_empty() && self.black_pool.is_empty() {
            Piece::RED_PLAYER_PIECES.to_vec()
        } else {
            self.red_pool
                .iter()
                .map(|name| {
                    let c =
                        name.chars().next().ok_or_else(|| format!("empty piece name: {name}"))?;
                    Piece::lookup(c, Color::Red).ok_or_else(|| format!("unknown red piece: {name}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let black_pool = if !has_cells && self.red_pool.is_empty() && self.black_pool.is_empty() {
            Piece::BLACK_PLAYER_PIECES.to_vec()
        } else {
            self.black_pool
                .iter()
                .map(|name| {
                    let c =
                        name.chars().next().ok_or_else(|| format!("empty piece name: {name}"))?;
                    Piece::lookup(c, Color::Black)
                        .ok_or_else(|| format!("unknown black piece: {name}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let player = match self.player.as_deref() {
            None | Some("Red") => Player::Red,
            Some("Black") => Player::Black,
            Some(s) => return Err(format!("invalid player: {s}")),
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
        Color::Red => "Red".into(),
        Color::Black => "Black".into(),
        Color::White => "White".into(),
    }
}

pub fn parse_color(s: &str) -> Result<Color, String> {
    match s {
        "Red" => Ok(Color::Red),
        "Black" => Ok(Color::Black),
        "White" => Ok(Color::White),
        _ => Err(format!("unknown color: {s}")),
    }
}

pub fn player_to_str(player: Player) -> String {
    match player {
        Player::Red => "Red".into(),
        Player::Black => "Black".into(),
    }
}

pub fn result_to_str(result: GameResult) -> String {
    match result {
        GameResult::Unfinished => "Unfinished".into(),
        GameResult::RedWin => "RedWin".into(),
        GameResult::BlackWin => "BlackWin".into(),
        GameResult::Draw => "Draw".into(),
    }
}

pub fn action_type_str(action: &Action) -> &'static str {
    match action {
        Action::Move(_) => "move",
        Action::Capture(_) => "capture",
        Action::Push(_) => "push",
        Action::Draw(_) => "draw",
        Action::Leave(_) => "leave",
        _ => "move",
    }
}
