use std::ops::Range;

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::game::Game;
use formation_chess_core::game::Phase;
use formation_chess_core::piece::Player;

/// Compact geometric placement-phase input for an agent.
///
/// Occupied points remain in [`PlacementArea::positions`]. Agents inspect the
/// supplied [Game] for board occupancy and the current player's piece pool,
/// avoiding both duplicated state and materialized piece-position actions.
#[derive(Debug, Copy, Clone)]
pub struct PlacementArea {
    x_start: u8,
    x_end: u8,
    y_start: u8,
    y_end: u8,
}

impl PlacementArea {
    /// Columns in the placement half-board.
    pub fn x_range(&self) -> Range<u8> {
        self.x_start .. self.x_end
    }

    /// Rows in the placement half-board.
    pub fn y_range(&self) -> Range<u8> {
        self.y_start .. self.y_end
    }

    /// Whether a point belongs to the geometric placement half-board.
    pub fn contains(&self, (x, y): (u8, u8)) -> bool {
        self.x_start <= x && x < self.x_end && self.y_start <= y && y < self.y_end
    }

    /// All points in the geometric placement half-board, in column-major order.
    pub fn positions(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        (self.x_start .. self.x_end)
            .flat_map(move |x| (self.y_start .. self.y_end).map(move |y| (x, y)))
    }
}

/// Build the compact placement input for the current player.
///
/// Returns `None` outside an unfinished placement phase. On odd-height boards,
/// the middle row belongs to neither player, matching the core placement rule.
pub fn placement_area(game: &Game) -> Option<PlacementArea> {
    if game.phase() != Phase::Place || game.result() != GameResult::Unfinished {
        return None;
    }

    let board = game.board();
    let (y_start, y_end) = match game.player() {
        Player::Red => (board.height().div_ceil(2), board.height()),
        Player::Black => (0, board.height() / 2),
    };

    Some(PlacementArea { x_start: 0, x_end: board.width(), y_start, y_end })
}

/// Append movement-phase actions accepted by the core rules engine to `actions`.
///
/// Resignation is intentionally excluded because it is not a board move and
/// should be handled by an explicit higher-level policy. Passing is always the
/// final appended candidate in an unfinished movement phase.
pub fn legal_movement_actions(game: &Game, actions: &mut Vec<Action>) {
    if game.phase() != Phase::Move || game.result() != GameResult::Unfinished {
        return;
    }

    game.all_valid_moves(actions);
    actions.push(Action::Pass(game.player()));
}
