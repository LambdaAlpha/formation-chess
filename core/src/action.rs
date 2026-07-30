use std::str::FromStr;

use crate::piece::Piece;
use crate::piece::Player;

/// A player action, expressed in 0-based board coordinates.
#[derive(Debug, Copy, Clone)]
pub enum Action {
    /// Place a piece from a pool onto the board.
    Place(Place),
    /// Move to an empty point.
    Move(Move),
    /// Move onto an occupied point, capturing the piece there.
    Capture(Move),
    /// Move onto an occupied point, shoving the piece there one step
    /// farther.
    Push(Move),
    /// Move onto an opponent's vital piece to draw the game.
    Draw(Move),
    /// Move to an empty point and leave a white piece at the original
    /// position. Only pieces with CONTROL_WHITE ability may do this.
    Leave(Move),
    /// Skip the turn without moving.
    Pass(Player),
    /// Concede: the opponent wins immediately.
    Resign(Player),
}

/// The successful outcome of an [`Action`]: the piece changes it caused and
/// the game result afterwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Reaction {
    pub changes: Vec<PositionChange>,
    pub game_result: GameResult,
}

/// The new content of a single point: a piece, or None for empty.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PositionChange {
    pub at: (u8, u8),
    pub piece: Option<Piece>,
}

/// A piece arriving at `to` from outside the board.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Place {
    pub piece: Piece,
    pub to: (u8, u8),
}

/// A movement between two board points.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Move {
    pub from: (u8, u8),
    pub to: (u8, u8),
}

/// The persistent result of a game. Anything other than `Unfinished` means
/// the game is over and no further actions are accepted.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum GameResult {
    Unfinished,
    RedWin,
    BlackWin,
    Draw,
}

impl std::fmt::Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GameResult::Unfinished => "未分",
            GameResult::RedWin => "红胜",
            GameResult::BlackWin => "黑胜",
            GameResult::Draw => "和棋",
        })
    }
}

impl std::fmt::Debug for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl FromStr for GameResult {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "未分" => Ok(GameResult::Unfinished),
            "红胜" => Ok(GameResult::RedWin),
            "黑胜" => Ok(GameResult::BlackWin),
            "和棋" => Ok(GameResult::Draw),
            _ => Err(format!("unknown result: {s}")),
        }
    }
}
