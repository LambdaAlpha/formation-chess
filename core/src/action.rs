use std::str::FromStr;

use crate::piece::Piece;
use crate::piece::PieceId;
use crate::piece::Player;

/// A player action, expressed in 0-based board coordinates.
#[derive(Debug, Copy, Clone, PartialEq)]
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
    /// Move to an empty point and pull the piece one movement step behind
    /// the origin into the origin.
    Pull(Move),
    /// Exchange positions with an opponent's vital piece to draw the game.
    Draw(Move),
    /// Skip the turn without moving.
    Pass(Player),
    /// Concede the side owning the vital piece at the coordinate. The
    /// coordinate is ignored during the placement phase.
    Resign(u8, u8),
}

/// Maximum number of board points changed by one action.
pub const MAX_POSITION_CHANGES: usize = 3;

/// A reversible change to a single board point.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PositionChange {
    pub at: (u8, u8),
    /// The complete piece before the action, or None when the point was empty.
    pub old: Option<Piece>,
    /// The complete piece after the action, or None when the point became empty.
    pub new: Option<Piece>,
}

const EMPTY_POSITION_CHANGE: PositionChange = PositionChange { at: (0, 0), old: None, new: None };

/// Up to three reversible board changes stored without heap allocation.
///
/// Unused slots contain the `(0, 0), None, None` sentinel. The first sentinel
/// terminates the exposed slice.
#[derive(Copy, Clone, PartialEq)]
pub struct PositionChanges {
    changes: [PositionChange; MAX_POSITION_CHANGES],
}

impl PositionChanges {
    pub const fn empty() -> Self {
        Self { changes: [EMPTY_POSITION_CHANGE; MAX_POSITION_CHANGES] }
    }

    pub(crate) fn one(change: PositionChange) -> Self {
        debug_assert!(change.old != change.new, "position change must not be a no-op");
        Self { changes: [change, EMPTY_POSITION_CHANGE, EMPTY_POSITION_CHANGE] }
    }

    pub(crate) fn two(first: PositionChange, second: PositionChange) -> Self {
        debug_assert!(first.old != first.new, "position change must not be a no-op");
        debug_assert!(second.old != second.new, "position change must not be a no-op");
        Self { changes: [first, second, EMPTY_POSITION_CHANGE] }
    }

    pub(crate) fn three(
        first: PositionChange, second: PositionChange, third: PositionChange,
    ) -> Self {
        debug_assert!(first.old != first.new, "position change must not be a no-op");
        debug_assert!(second.old != second.new, "position change must not be a no-op");
        debug_assert!(third.old != third.new, "position change must not be a no-op");
        Self { changes: [first, second, third] }
    }

    pub fn try_from_slice(changes: &[PositionChange]) -> Result<Self, String> {
        if changes.len() > MAX_POSITION_CHANGES {
            return Err(format!("position changes exceed the maximum of {MAX_POSITION_CHANGES}"));
        }

        let mut result = Self::empty();
        for (index, &change) in changes.iter().enumerate() {
            if change.old == change.new {
                return Err(format!(
                    "position change at ({},{}) must change the position",
                    change.at.0, change.at.1
                ));
            }
            result.changes[index] = change;
        }
        Ok(result)
    }

    pub fn as_slice(&self) -> &[PositionChange] {
        let mut len = 0;
        while len < MAX_POSITION_CHANGES && self.changes[len] != EMPTY_POSITION_CHANGE {
            len += 1;
        }
        &self.changes[.. len]
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, PositionChange> {
        self.as_slice().iter()
    }
}

impl Default for PositionChanges {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Debug for PositionChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        list.entries(self.as_slice());
        list.finish()
    }
}

/// The successful outcome of an [`Action`]: reversible board and pool changes,
/// plus the game result afterwards.
#[derive(Debug, Clone, PartialEq)]
pub struct Reaction {
    pub changes: PositionChanges,
    pub pool_change: PoolChange,
    pub game_result: GameResult,
}

/// A reversible change to the current player's placement pool.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PoolChange {
    /// The placement pool was not changed.
    Unchanged,
    /// A piece was removed from the current player's pool.
    Removed { index: usize, piece: Piece },
}

/// A piece identity arriving at `to` from outside the board.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Place {
    pub piece: PieceId,
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

#[cfg(test)]
mod tests {
    use super::EMPTY_POSITION_CHANGE;
    use super::MAX_POSITION_CHANGES;
    use super::PositionChange;
    use super::PositionChanges;
    use crate::piece::Piece;

    #[test]
    fn position_changes_exposes_only_the_valid_prefix() {
        let first = PositionChange { at: (1, 2), old: None, new: Some(Piece::RED_ROOK) };
        let second = PositionChange { at: (2, 2), old: Some(Piece::BLACK_ROOK), new: None };
        let changes = PositionChanges::two(first, second);

        assert_eq!(changes.as_slice(), &[first, second]);
        assert_eq!(changes.len(), 2);
        assert!(!changes.is_empty());
        assert_eq!(changes.changes[2], EMPTY_POSITION_CHANGE);
    }

    #[test]
    fn empty_position_changes_exposes_an_empty_slice() {
        let changes = PositionChanges::empty();

        assert!(changes.as_slice().is_empty());
        assert!(changes.is_empty());
        assert_eq!(changes.changes, [EMPTY_POSITION_CHANGE; MAX_POSITION_CHANGES]);
    }

    #[test]
    fn position_changes_rejects_invalid_or_excess_changes() {
        let invalid = PositionChange { at: (1, 1), old: None, new: None };
        let valid = PositionChange { at: (1, 2), old: None, new: Some(Piece::RED_ROOK) };

        let invalid_error = PositionChanges::try_from_slice(&[invalid]).unwrap_err();
        assert!(invalid_error.contains("must change"), "unexpected error: {invalid_error}");

        let excess_error =
            PositionChanges::try_from_slice(&[valid; MAX_POSITION_CHANGES + 1]).unwrap_err();
        assert!(excess_error.contains("maximum of 3"), "unexpected error: {excess_error}");
    }
}
