use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::Move;
use formation_chess_core::action::Place;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::PieceId;
use formation_chess_core::piece::Player;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPieceRef {
    pub name: char,
    pub player: String,
}

impl ApiPieceRef {
    pub fn from_id(piece: PieceId) -> Self {
        Self { name: piece.name, player: player_to_str(piece.player) }
    }

    pub fn to_id(&self) -> Result<PieceId, String> {
        let player = parse_player(&self.player)?;
        let Some(piece) = Piece::lookup(self.name, player) else {
            return Err(format!("unknown piece: {} {:?}", self.name, self.player));
        };
        Ok(piece.id())
    }

    pub fn to_piece(&self) -> Result<Piece, String> {
        let player = parse_player(&self.player)?;
        let Some(piece) = Piece::lookup(self.name, player) else {
            return Err(format!("unknown piece: {} {:?}", self.name, self.player));
        };
        Ok(piece)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiPiece {
    pub name: char,
    pub player: String,
    pub formation: u8,
}

impl ApiPiece {
    pub fn from_piece(piece: Piece) -> Self {
        Self {
            name: piece.name,
            player: player_to_str(piece.player),
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

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiAgentStrength {
    VeryWeak,
    Weak,
    #[default]
    Medium,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ApiControllerConfig {
    pub control: ApiControl,
    #[serde(default)]
    pub strength: ApiAgentStrength,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(untagged)]
pub enum ApiControllerSetting {
    Control(ApiControl),
    Config(ApiControllerConfig),
}

impl ApiControllerSetting {
    pub fn new(control: ApiControl, strength: ApiAgentStrength) -> Self {
        Self::Config(ApiControllerConfig { control, strength })
    }

    pub fn control(self) -> ApiControl {
        match self {
            Self::Control(control) => control,
            Self::Config(config) => config.control,
        }
    }

    pub fn strength(self) -> ApiAgentStrength {
        match self {
            Self::Control(_) => ApiAgentStrength::default(),
            Self::Config(config) => config.strength,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ApiControllerSettings {
    #[serde(default = "default_red_controller")]
    pub red: ApiControllerSetting,
    #[serde(default = "default_black_controller")]
    pub black: ApiControllerSetting,
}

impl Default for ApiControllerSettings {
    fn default() -> Self {
        Self { red: default_red_controller(), black: default_black_controller() }
    }
}

fn default_red_controller() -> ApiControllerSetting {
    ApiControllerSetting::new(ApiControl::Human, ApiAgentStrength::Medium)
}

fn default_black_controller() -> ApiControllerSetting {
    ApiControllerSetting::new(ApiControl::Agent, ApiAgentStrength::Medium)
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiControllerInfo {
    pub control: ApiControl,
    pub strength: ApiAgentStrength,
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
    pub strength: ApiAgentStrength,
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
    pub legal_actions: Vec<ApiAction>,
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
            strength: current.strength,
            agent: current.agent.clone(),
        };
        let unfinished = game.result() == GameResult::Unfinished;
        let current_control = current_controller.control;
        let mut legal_actions = Vec::new();
        game.all_valid_moves(&mut legal_actions);
        let legal_actions = legal_actions.into_iter().map(ApiAction::from_action).collect();

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
            legal_actions,
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
    Pull { from: [u8; 2], to: [u8; 2] },
    Draw { from: [u8; 2], to: [u8; 2] },
    Resign { at: [u8; 2] },
}

impl ApiAction {
    pub fn from_action(action: Action) -> Self {
        match action {
            Action::Place(place) => {
                Self::Place { piece: ApiPieceRef::from_id(place.piece), to: pair(place.to) }
            },
            Action::Move(move_) => Self::Move { from: pair(move_.from), to: pair(move_.to) },
            Action::Capture(move_) => Self::Capture { from: pair(move_.from), to: pair(move_.to) },
            Action::Push(move_) => Self::Push { from: pair(move_.from), to: pair(move_.to) },
            Action::Pull(move_) => Self::Pull { from: pair(move_.from), to: pair(move_.to) },
            Action::Draw(move_) => Self::Draw { from: pair(move_.from), to: pair(move_.to) },
            Action::Resign(x, y) => Self::Resign { at: [x, y] },
        }
    }

    pub fn to_action(&self) -> Result<Action, String> {
        match self {
            Self::Place { piece, to } => {
                Ok(Action::Place(Place { piece: piece.to_id()?, to: tuple(*to) }))
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
            Self::Pull { from, to } => {
                Ok(Action::Pull(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Draw { from, to } => {
                Ok(Action::Draw(Move { from: tuple(*from), to: tuple(*to) }))
            },
            Self::Resign { at } => Ok(Action::Resign(at[0], at[1])),
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
            parse_pool(&self.red_pool, Player::Red)?
        };
        let black_pool = if use_standard_pools {
            Piece::BLACK_PLAYER_PIECES.to_vec()
        } else {
            parse_pool(&self.black_pool, Player::Black)?
        };
        let player = match self.player.as_deref() {
            None | Some("Red") => Player::Red,
            Some("Black") => Player::Black,
            Some(value) => return Err(format!("invalid player: {value}")),
        };

        Ok(GameConfig { player, board, red_pool, black_pool, result: GameResult::Unfinished })
    }
}

fn parse_pool(names: &[String], player: Player) -> Result<Vec<Piece>, String> {
    names
        .iter()
        .map(|name| {
            let piece_name =
                name.chars().next().ok_or_else(|| format!("empty piece name: {name}"))?;
            Piece::lookup(piece_name, player)
                .ok_or_else(|| format!("unknown {} piece: {name}", player_to_str(player)))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn controller_settings_accept_legacy_and_strength_forms() {
        let legacy: ApiControllerSettings = serde_json::from_value(json!({
            "red": "human",
            "black": "agent",
        }))
        .expect("legacy controller settings");
        assert_eq!(legacy.red.control(), ApiControl::Human);
        assert_eq!(legacy.red.strength(), ApiAgentStrength::Medium);
        assert_eq!(legacy.black.control(), ApiControl::Agent);
        assert_eq!(legacy.black.strength(), ApiAgentStrength::Medium);

        let configured: ApiControllerSettings = serde_json::from_value(json!({
            "red": { "control": "agent", "strength": "very_weak" },
            "black": { "control": "agent", "strength": "weak" },
        }))
        .expect("strength controller settings");
        assert_eq!(configured.red.control(), ApiControl::Agent);
        assert_eq!(configured.red.strength(), ApiAgentStrength::VeryWeak);
        assert_eq!(configured.black.control(), ApiControl::Agent);
        assert_eq!(configured.black.strength(), ApiAgentStrength::Weak);
    }

    #[test]
    fn action_protocol_round_trips_pull_and_targeted_resign() {
        let pull = Action::Pull(Move { from: (2, 2), to: (2, 1) });
        let api_pull = ApiAction::from_action(pull);
        assert_eq!(
            serde_json::to_value(&api_pull).expect("serialize pull"),
            json!({
                "type": "pull",
                "from": [2, 2],
                "to": [2, 1],
            })
        );
        assert_eq!(api_pull.to_action().expect("decode pull"), pull);

        let resign = Action::Resign(0, 4);
        let api_resign = ApiAction::from_action(resign);
        assert_eq!(
            serde_json::to_value(&api_resign).expect("serialize resign"),
            json!({
                "type": "resign",
                "at": [0, 4],
            })
        );
        assert_eq!(api_resign.to_action().expect("decode resign"), resign);
    }

    #[test]
    fn piece_protocol_uses_player_identity_and_rejects_white() {
        let piece = ApiPieceRef::from_id(Piece::RED_WIND.id());
        assert_eq!(piece.name, '风');
        assert_eq!(piece.player, "Red");
        assert_eq!(piece.to_id().expect("decode red wind"), Piece::RED_WIND.id());

        let value = serde_json::to_value(&piece).expect("serialize piece reference");
        assert_eq!(value, json!({ "name": "风", "player": "Red" }));
        assert!(value.get("color").is_none(), "legacy color field must be absent");

        let white = ApiPieceRef { name: '风', player: "White".to_owned() };
        assert_eq!(
            white.to_piece().expect_err("white pieces must be rejected"),
            "unknown side: White"
        );
    }
}
