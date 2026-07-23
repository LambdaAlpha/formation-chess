use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;
use std::ops::Index;
use std::ops::IndexMut;
use std::str::FromStr;

use crate::ability::Ability;
use crate::action::Action;
use crate::action::Move;
use crate::action::Place;
use crate::action::PositionChange;
use crate::chinese_num::fmt_num;
use crate::piece::Color;
use crate::piece::Piece;
use crate::piece::Player;

/// A rectangular grid of points (at most 16×16), each empty or holding one
/// piece. Coordinates are 0-based `(x, y)` with `(0, 0)` at the top left.
#[derive(Clone)]
pub struct Board {
    width: u8,
    height: u8,
    pieces: Vec<Option<Piece>>,
}

/// The center piece plus its eight neighbors, each with relative position.
pub struct Local {
    pub center: Option<Piece>,
    pub neighbors: [Neighbor; 8],
}

/// A neighbor of a board point, carrying its relative offset from the center.
#[derive(Debug, Copy, Clone)]
pub struct Neighbor {
    pub dx: i8,
    pub dy: i8,
    pub piece: Option<Piece>,
}

struct MoveInfo {
    mover: Piece,
    from: (u8, u8),
    to: (u8, u8),
    path: MovePath,
}

/// Piece counts on the open interval between a move's endpoints.
struct MovePath {
    /// Pieces on the path, passable or not.
    pieces: u8,
    /// The subset of `pieces` the mover cannot pass through.
    unpassable: u8,
}

impl Board {
    /// An empty board. Panics when a dimension exceeds 16.
    pub fn new(width: u8, height: u8) -> Self {
        assert!(width <= 16 && height <= 16, "board dimensions must be <= 16");
        Self { width, height, pieces: vec![None; (width * height) as usize] }
    }

    /// Number of columns.
    pub fn width(&self) -> u8 {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> u8 {
        self.height
    }

    /// Whether (x,y) lies on the board.
    pub fn in_bounds(&self, pos: (u8, u8)) -> bool {
        pos.0 < self.width && pos.1 < self.height
    }

    /// The piece at `pos`, or None when `pos` is empty or outside the board.
    pub fn get(&self, pos: (u8, u8)) -> Option<Piece> {
        if self.in_bounds(pos) { self[pos] } else { None }
    }

    fn index(&self, x: u8, y: u8) -> usize {
        assert!(x < self.width && y < self.height, "position out of bounds");
        y as usize * self.width as usize + x as usize
    }

    /// The (x,y) coordinates of a linear index into `pieces`.
    fn position(&self, index: usize) -> (u8, u8) {
        assert!(index < self.pieces.len(), "index out of bounds");
        ((index % self.width as usize) as u8, (index / self.width as usize) as u8)
    }

    pub(crate) fn validate_halves(&self) -> Result<(), String> {
        let height = self.height();
        let half = height / 2;
        let midpoint = height.div_ceil(2);
        for y in 0 .. midpoint {
            for x in 0 .. self.width() {
                let Some(piece) = self[(x, y)] else { continue };
                if piece.color != Color::Red {
                    continue;
                }
                return Err(format!(
                    "red piece {piece} at ({x},{y}) must be in the bottom half during placement"
                ));
            }
        }
        for y in half .. height {
            for x in 0 .. self.width() {
                let Some(piece) = self[(x, y)] else { continue };
                if piece.color != Color::Black {
                    continue;
                }
                return Err(format!(
                    "black piece {piece} at ({x},{y}) must be in the top half during placement"
                ));
            }
        }
        Ok(())
    }

    /// Returns the center piece at (x,y) and its eight neighbors, each with
    /// its relative offset from the center.
    pub fn local(&self, x: u8, y: u8) -> Local {
        let at = |dx: i8, dy: i8| {
            let nx = x as i8 + dx;
            let ny = y as i8 + dy;
            if nx >= 0 && ny >= 0 && (nx as u8) < self.width && (ny as u8) < self.height {
                self.pieces[self.index(nx as u8, ny as u8)]
            } else {
                None
            }
        };
        Local {
            center: at(0, 0),
            neighbors: [
                Neighbor { dx: -1, dy: -1, piece: at(-1, -1) },
                Neighbor { dx: 0, dy: -1, piece: at(0, -1) },
                Neighbor { dx: 1, dy: -1, piece: at(1, -1) },
                Neighbor { dx: -1, dy: 0, piece: at(-1, 0) },
                Neighbor { dx: 1, dy: 0, piece: at(1, 0) },
                Neighbor { dx: -1, dy: 1, piece: at(-1, 1) },
                Neighbor { dx: 0, dy: 1, piece: at(0, 1) },
                Neighbor { dx: 1, dy: 1, piece: at(1, 1) },
            ],
        }
    }

    /// The piece at (x,y) with formation effects from its neighbors applied.
    /// Returns None when (x,y) is empty or outside the board.
    pub fn effective(&self, (x, y): (u8, u8)) -> Option<Piece> {
        if !self.in_bounds((x, y)) {
            return None;
        }
        let local = self.local(x, y);
        let mut piece = local.center?;
        piece.take_effect(&local.neighbors);
        Some(piece)
    }

    /// The first piece of `color` with the VITAL ability, and its position.
    /// VITAL is never modified by formation effects, so raw abilities suffice.
    pub fn find_vital(&self, color: Color) -> Option<Place> {
        for (i, cell) in self.pieces.iter().enumerate() {
            let Some(piece) = cell else {
                continue;
            };
            if piece.color == color && piece.ability.has_ability(Ability::VITAL) {
                return Some(Place { to: self.position(i), piece: *piece });
            }
        }
        None
    }

    /// Number of pieces of `color` with the VITAL ability.
    pub fn vital_count(&self, color: Color) -> usize {
        let mut count = 0;
        for cell in &self.pieces {
            let Some(p) = cell else {
                continue;
            };
            if p.color == color && p.ability.has_ability(Ability::VITAL) {
                count += 1;
            }
        }
        count
    }

    /// Find the unique point holding `piece`.
    ///
    /// * `Ok(pos)` — exactly one copy at `pos`.
    /// * `Err(false)` — no copy on the board.
    /// * `Err(true)` — more than one copy; the caller must identify the
    ///   piece by coordinates.
    pub fn find_unique(&self, piece: Piece) -> Result<(u8, u8), bool> {
        let mut pos = None;
        for (i, cell) in self.pieces.iter().enumerate() {
            if *cell == Some(piece) {
                if pos.is_some() {
                    return Err(true);
                }
                pos = Some(self.position(i));
            }
        }
        match pos {
            Some(p) => Ok(p),
            None => Err(false),
        }
    }

    /// Place a red or black piece onto an empty point in its own half.
    pub fn place(&mut self, piece: Piece, to: (u8, u8)) -> Result<Vec<PositionChange>, String> {
        let changes = self.try_place(piece, to)?;
        self.apply(&changes);
        Ok(changes)
    }

    /// Validate a placement without modifying the board. Checks bounds,
    /// vacancy, and the half-board rule (red bottom, black top).
    pub fn try_place(&self, piece: Piece, to: (u8, u8)) -> Result<Vec<PositionChange>, String> {
        self.check_placement_target(to)?;
        let half = self.height / 2;
        let midpoint = self.height.div_ceil(2);
        if piece.color == Color::Red && to.1 < midpoint {
            return Err("red pieces can only be placed in the bottom half".into());
        }
        if piece.color == Color::Black && to.1 >= half {
            return Err("black pieces can only be placed in the top half".into());
        }
        Ok(vec![PositionChange { at: to, piece: Some(piece) }])
    }

    /// Place a white piece on an empty point covered by a CONTROL_WHITE
    /// formation the given player commands.
    pub fn place_white(
        &mut self, white: Piece, to: (u8, u8), player: Player,
    ) -> Result<Vec<PositionChange>, String> {
        let changes = self.try_place_white(white, to, player)?;
        self.apply(&changes);
        Ok(changes)
    }

    /// Validate a white-piece placement without modifying the board.
    /// The target must be empty and covered by a CONTROL_WHITE piece
    /// that the given player commands.
    pub fn try_place_white(
        &self, white: Piece, to: (u8, u8), player: Player,
    ) -> Result<Vec<PositionChange>, String> {
        self.check_placement_target(to)?;
        let mut has_control = false;
        for n in &self.local(to.0, to.1).neighbors {
            let Some(piece) = n.piece else {
                continue;
            };
            if piece.formation.contains(-n.dx, -n.dy)
                && piece.ability.has_ability(Ability::CONTROL_WHITE)
                && piece.can_controlled_by(player)
            {
                has_control = true;
                break;
            }
        }
        if !has_control {
            return Err(format!(
                "({},{}) is not covered by any piece with CONTROL_WHITE controlled by player {}",
                to.0, to.1, player
            ));
        }
        Ok(vec![PositionChange { at: to, piece: Some(white) }])
    }

    /// Verify `to` is in bounds and empty, for placement.
    fn check_placement_target(&self, to: (u8, u8)) -> Result<(), String> {
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        if self[to].is_some() {
            return Err(format!("destination ({},{}) is already occupied", to.0, to.1));
        }
        Ok(())
    }

    /// Execute a simple move to an empty point (no capture or push intent).
    pub fn move_(&mut self, move_: Move) -> Result<Vec<PositionChange>, String> {
        let changes = self.try_move(move_.from, move_.to)?;
        self.apply(&changes);
        Ok(changes)
    }

    /// Validate a simple move to an empty point without modifying the board.
    /// Checks movement ability, distance, and pass-through.
    pub fn try_move(&self, from: (u8, u8), to: (u8, u8)) -> Result<Vec<PositionChange>, String> {
        let info = self.try_move_to(from, to)?;
        if self[info.to].is_some() {
            return Err(format!(
                "cannot move onto occupied destination ({},{})",
                info.to.0, info.to.1
            ));
        }
        if info.path.unpassable > 0 {
            return Err("path blocked, cannot reach empty destination".into());
        }
        Ok(vec![PositionChange { at: info.from, piece: None }, PositionChange {
            at: info.to,
            piece: Some(info.mover),
        }])
    }

    /// Execute a capture (normal or jump, including mutual destruction).
    pub fn capture(&mut self, move_: Move) -> Result<Vec<PositionChange>, String> {
        let changes = self.try_capture(move_.from, move_.to)?;
        self.apply(&changes);
        Ok(changes)
    }

    /// Validate a capture without modifying the board. Checks movement,
    /// pass-through, capture/jump-capture ability, and mutual-destruction
    /// effects.
    pub fn try_capture(&self, from: (u8, u8), to: (u8, u8)) -> Result<Vec<PositionChange>, String> {
        let info = self.try_move_to(from, to)?;
        let Some(target) = self.effective(info.to) else {
            return Err(format!(
                "destination ({},{}) is empty, capture requires an occupied point",
                info.to.0, info.to.1
            ));
        };
        let normal_capture = info.path.unpassable == 0 && info.mover.can_capture(target);
        let jump_capture = info.mover.can_jump_capture(target, info.path.pieces);
        if !normal_capture && !jump_capture {
            return Err(format!("cannot capture {} at ({},{})", target, info.to.0, info.to.1));
        }
        Ok(Self::capture_result(info.mover, target, info.from, info.to))
    }

    /// Execute a push (escalates to capture when blocked).
    pub fn push(&mut self, move_: Move) -> Result<Vec<PositionChange>, String> {
        let changes = self.try_push(move_.from, move_.to)?;
        self.apply(&changes);
        Ok(changes)
    }

    /// Validate a push without modifying the board. Checks movement,
    /// pass-through, push ability, and the pushed piece's landing.
    /// A blocked push may escalate to capture.
    pub fn try_push(&self, from: (u8, u8), to: (u8, u8)) -> Result<Vec<PositionChange>, String> {
        let info = self.try_move_to(from, to)?;
        let Some(target) = self.effective(info.to) else {
            return Err(format!(
                "destination ({},{}) is empty, push requires an occupied point",
                info.to.0, info.to.1
            ));
        };
        if info.path.unpassable > 0 {
            return Err("cannot push through blocking pieces on path".into());
        }
        if !info.mover.can_push(target) {
            return Err(format!("cannot push {} at ({},{})", target, info.to.0, info.to.1));
        }
        if let Some(pt) = self.pushed_target(info.from, info.to, target) {
            return Ok(vec![
                PositionChange { at: info.from, piece: None },
                PositionChange { at: info.to, piece: Some(info.mover) },
                PositionChange { at: pt, piece: Some(target) },
            ]);
        }
        if info.mover.color != target.color && target.ability.has_ability(Ability::CAPTURED) {
            return Ok(Self::capture_result(info.mover, target, info.from, info.to));
        }
        Err(format!("push blocked and cannot capture {} at ({},{})", target, info.to.0, info.to.1))
    }

    /// Shared pre-checks: bounds validation, piece lookup, movement ability,
    /// path computation.
    fn try_move_to(&self, from: (u8, u8), to: (u8, u8)) -> Result<MoveInfo, String> {
        if !self.in_bounds(from) {
            return Err(format!("({},{}) is outside the board", from.0, from.1));
        }
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        let Some(mover) = self.effective(from) else {
            return Err(format!("no piece at ({},{})", from.0, from.1));
        };
        if !Self::can_move(mover, from, to) {
            return Err(format!(
                "piece {mover} at ({},{}) cannot move to ({},{})",
                from.0, from.1, to.0, to.1
            ));
        }
        let path = MovePath::new(self, mover, from, to);
        Ok(MoveInfo { from, to, mover, path })
    }

    /// Whether from→to lies on a horizontal or vertical line. Note:
    /// `from == to` satisfies all three direction predicates; callers must
    /// exclude it first.
    fn is_direction_cross(from: (u8, u8), to: (u8, u8)) -> bool {
        from.0 == to.0 || from.1 == to.1
    }

    /// Whether from→to lies on a diagonal line. See the `from == to` note
    /// on [`Self::is_direction_cross`].
    fn is_direction_diagonal(from: (u8, u8), to: (u8, u8)) -> bool {
        from.0.abs_diff(to.0) == from.1.abs_diff(to.1)
    }

    /// Whether from→to lies on a knight line (1:2 slope, including chained
    /// knight moves). See the `from == to` note on
    /// [`Self::is_direction_cross`].
    #[expect(non_snake_case)]
    fn is_direction_shape_L(from: (u8, u8), to: (u8, u8)) -> bool {
        if from.0.abs_diff(to.0) * 2 == from.1.abs_diff(to.1) {
            return true;
        }
        if from.0.abs_diff(to.0) == from.1.abs_diff(to.1) * 2 {
            return true;
        }
        false
    }

    /// Whether a piece can move from→to, checking direction ability and
    /// distance only. Pieces on the path are ignored; pass-through rules
    /// are handled by the `move_*` methods.
    fn can_move(piece: Piece, from: (u8, u8), to: (u8, u8)) -> bool {
        if from == to {
            return false;
        }
        let is_cross = Self::is_direction_cross(from, to);
        let is_diagonal = Self::is_direction_diagonal(from, to);
        #[expect(non_snake_case)]
        let is_shape_L = Self::is_direction_shape_L(from, to);
        if !is_cross && !is_diagonal && !is_shape_L {
            return false;
        }
        if is_cross && !piece.ability.has_ability(Ability::DIRECTION_CROSS) {
            return false;
        }
        if is_diagonal && !piece.ability.has_ability(Ability::DIRECTION_DIAGONAL) {
            return false;
        }
        if is_shape_L && !piece.ability.has_ability(Ability::DIRECTION_SHAPE_L) {
            return false;
        }
        if piece.ability.has_ability(Ability::ANY_DISTANCE) {
            return true;
        }
        let dx = from.0.abs_diff(to.0);
        let dy = from.1.abs_diff(to.1);
        if is_cross {
            dx + dy == 1
        } else if is_diagonal {
            dx == 1
        } else if is_shape_L {
            dx.min(dy) == 1
        } else {
            false
        }
    }

    /// Compute the changes of a successful capture, including
    /// mutual‑destruction effects.
    fn capture_result(
        mover: Piece, target: Piece, from: (u8, u8), to: (u8, u8),
    ) -> Vec<PositionChange> {
        if mover.ability.has_ability(Ability::CAPTURED_ON_CAPTURE)
            || target.ability.has_ability(Ability::CAPTURE_ON_CAPTURED)
        {
            vec![PositionChange { at: from, piece: None }, PositionChange { at: to, piece: None }]
        } else {
            vec![PositionChange { at: from, piece: None }, PositionChange {
                at: to,
                piece: Some(mover),
            }]
        }
    }

    pub fn valid_white_placements(&self, player: Player) -> Vec<(u8, u8)> {
        let Some(place) = self.find_control_white(player) else {
            return vec![];
        };
        let mut targets = Vec::new();
        for dy in -1i8 ..= 1 {
            for dx in -1i8 ..= 1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if !place.piece.formation.contains(dx, dy) {
                    continue;
                }
                let tx = place.to.0 as i8 + dx;
                let ty = place.to.1 as i8 + dy;
                if tx < 0 || ty < 0 {
                    continue;
                }
                let to = (tx as u8, ty as u8);
                if !self.in_bounds(to) || self[to].is_some() {
                    continue;
                }
                targets.push(to);
            }
        }
        targets
    }

    /// The first piece with the CONTROL_WHITE ability that `player`
    /// commands, paired with its position. CONTROL_WHITE is never modified
    /// by formation effects, so raw abilities suffice.
    fn find_control_white(&self, player: Player) -> Option<Place> {
        for (i, cell) in self.pieces.iter().enumerate() {
            let Some(piece) = cell else {
                continue;
            };
            if piece.ability.has_ability(Ability::CONTROL_WHITE) && piece.can_controlled_by(player)
            {
                return Some(Place { to: self.position(i), piece: *piece });
            }
        }
        None
    }

    /// Enumerate all legal actions for the piece at `from`. The piece must
    /// be present on the board. Actions are [`Action::Move`],
    /// [`Action::Capture`], and [`Action::Push`]; placement, pass, and
    /// resign are the caller's concern.
    pub fn valid_moves(&self, from: (u8, u8)) -> Vec<Action> {
        let Some(piece) = self.effective(from) else {
            return vec![];
        };
        let max_steps = if piece.ability.has_ability(Ability::ANY_DISTANCE) {
            self.width.max(self.height) as i8
        } else {
            1
        };
        let mut actions = Vec::new();
        if piece.ability.has_ability(Ability::DIRECTION_CROSS) {
            for (dx, dy) in [(0i8, -1), (0, 1), (-1, 0), (1, 0)] {
                self.enumerate_line(piece, from, dx, dy, max_steps, &mut actions);
            }
        }
        if piece.ability.has_ability(Ability::DIRECTION_DIAGONAL) {
            for (dx, dy) in [(-1i8, -1), (1, -1), (-1, 1), (1, 1)] {
                self.enumerate_line(piece, from, dx, dy, max_steps, &mut actions);
            }
        }
        if piece.ability.has_ability(Ability::DIRECTION_SHAPE_L) {
            for (dx, dy) in
                [(1i8, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1)]
            {
                self.enumerate_line(piece, from, dx, dy, max_steps, &mut actions);
            }
        }
        actions
    }

    /// Scan from `from` along `(dx, dy)`, adding legal [`Action`]s for each
    /// reachable cell.  Uses [`MovePath`] for per‑step path validation so
    /// that leg‑blocking and pass‑through rules stay in one place.
    fn enumerate_line(
        &self, mover: Piece, from: (u8, u8), dx: i8, dy: i8, max_steps: i8,
        actions: &mut Vec<Action>,
    ) {
        let mut origin = from;
        let mut blocked = false;
        let mut path_pieces: u8 = 0;

        for _ in 0 .. max_steps {
            let nx = origin.0 as i8 + dx;
            let ny = origin.1 as i8 + dy;
            if nx < 0 || ny < 0 || nx as u8 >= self.width || ny as u8 >= self.height {
                break;
            }
            let to = (nx as u8, ny as u8);

            let step_path = MovePath::new(self, mover, origin, to);
            path_pieces += step_path.pieces;
            if step_path.unpassable > 0 {
                blocked = true;
            }

            if self[to].is_none() {
                if blocked {
                    break;
                }
                actions.push(Action::Move(Move { from, to }));
            } else if let Some(target) = self.effective(to) {
                let move_ = Move { from, to };
                self.enumerate_action(mover, move_, target, blocked, path_pieces, actions);
                path_pieces += 1;
                if !mover.can_pass(target) {
                    if blocked {
                        break;
                    }
                    blocked = true;
                }
            }

            origin = to;
        }
    }

    /// Add every capture and push action legal against `target` at `move_.to`,
    /// given the accumulated `blocked` / `path_pieces` state for this cell.
    fn enumerate_action(
        &self, mover: Piece, move_: Move, target: Piece, blocked: bool, path_pieces: u8,
        actions: &mut Vec<Action>,
    ) {
        if target.color != mover.color {
            let mut captured = false;
            if !blocked {
                if mover.can_capture(target) {
                    actions.push(Action::Capture(move_));
                    captured = true;
                }
                if mover.can_push(target) {
                    if let Some(_pt) = self.pushed_target(move_.from, move_.to, target) {
                        actions.push(Action::Push(move_));
                    } else if !captured && target.ability.has_ability(Ability::CAPTURED) {
                        actions.push(Action::Capture(move_));
                        captured = true;
                    }
                }
            }
            if !captured && mover.can_jump_capture(target, path_pieces) {
                actions.push(Action::Capture(move_));
            }
        } else if !blocked
            && mover.can_push(target)
            && self.pushed_target(move_.from, move_.to, target).is_some()
        {
            actions.push(Action::Push(move_));
        }
    }

    /// Where the pushed piece would land, continuing one step along the push
    /// direction. The pushed piece's own direction abilities are irrelevant:
    /// the shove supplies the movement. Returns None if the pushed piece
    /// cannot make that step: the landing point is off the board or occupied,
    /// or the pushed piece cannot traverse its own path there (the same
    /// pass-through rules as a normal move; for L-shaped pushes this is the
    /// pushed piece's leg).
    fn pushed_target(&self, from: (u8, u8), to: (u8, u8), pushed: Piece) -> Option<(u8, u8)> {
        let dx: i8 = to.0 as i8 - from.0 as i8;
        let dy: i8 = to.1 as i8 - from.1 as i8;
        let sx = dx.signum();
        let sy = dy.signum();
        let adx = dx.unsigned_abs();
        let ady = dy.unsigned_abs();
        let (tsx, tsy) = if adx == 0 || ady == 0 || adx == ady {
            (sx, sy)
        } else if adx * 2 == ady {
            (sx, sy * 2)
        } else if adx == ady * 2 {
            (sx * 2, sy)
        } else {
            panic!("push_target called with invalid move vector ({dx},{dy})");
        };
        let tx = to.0 as i8 + tsx;
        let ty = to.1 as i8 + tsy;
        if tx < 0 || ty < 0 || tx as u8 >= self.width || ty as u8 >= self.height {
            return None;
        }
        let pt = (tx as u8, ty as u8);
        if self[pt].is_some() {
            return None;
        }
        // The pushed piece makes a regular one-step move from `to` to `pt`:
        // its path (the leg, for L-shaped pushes) must be passable by it.
        if MovePath::new(self, pushed, to, pt).unpassable > 0 {
            return None;
        }
        Some(pt)
    }

    /// Write position-based changes onto the board. Panics when a change
    /// lies outside the board.
    pub fn apply(&mut self, changes: &[PositionChange]) {
        for change in changes {
            self[change.at] = change.piece;
        }
    }

    /// Normalize a flat list of position changes: when a point appears as
    /// both vacated and occupied the occupant wins. The result is sorted by
    /// position. This handles cyclic changes (e.g. two pieces swapping
    /// points) correctly.
    pub fn normalize_changes(changes: &[PositionChange]) -> Vec<PositionChange> {
        let mut result: Vec<PositionChange> = Vec::new();
        for change in changes {
            if let Some(pos) = result.iter_mut().find(|c| c.at == change.at) {
                if pos.piece.is_none() && change.piece.is_some() {
                    pos.piece = change.piece;
                }
            } else {
                result.push(PositionChange { at: change.at, piece: change.piece });
            }
        }
        result.sort_by_key(|c| c.at);
        result
    }
}

/// Direct point access. Panics when (x,y) is outside the board; use
/// [`Board::get`] for checked access.
impl Index<(u8, u8)> for Board {
    type Output = Option<Piece>;
    fn index(&self, (x, y): (u8, u8)) -> &Self::Output {
        let i = self.index(x, y);
        &self.pieces[i]
    }
}

/// Direct point access. Panics when (x,y) is outside the board.
impl IndexMut<(u8, u8)> for Board {
    fn index_mut(&mut self, (x, y): (u8, u8)) -> &mut Self::Output {
        let i = self.index(x, y);
        &mut self.pieces[i]
    }
}

impl MovePath {
    fn new(board: &Board, mover: Piece, from: (u8, u8), to: (u8, u8)) -> Self {
        let mut pieces: u8 = 0;
        let mut unpassable: u8 = 0;
        for pos in Self::path_positions(from, to) {
            if let Some(blocker) = board.effective(pos) {
                pieces += 1;
                if !mover.can_pass(blocker) {
                    unpassable += 1;
                }
            }
        }
        Self { pieces, unpassable }
    }

    /// All positions between from and to (exclusive). For horse moves, includes
    /// leg-blocking corner positions and intermediate landing positions.
    fn path_positions(from: (u8, u8), to: (u8, u8)) -> Vec<(u8, u8)> {
        let mut positions = Vec::new();
        let dx = from.0.abs_diff(to.0);
        let dy = from.1.abs_diff(to.1);
        let sx: i8 = (to.0 as i8 - from.0 as i8).signum();
        let sy: i8 = (to.1 as i8 - from.1 as i8).signum();
        if dx == 0 {
            let (a, b) = if from.1 < to.1 { (from.1 + 1, to.1) } else { (to.1 + 1, from.1) };
            for y in a .. b {
                positions.push((from.0, y));
            }
        } else if dy == 0 {
            let (a, b) = if from.0 < to.0 { (from.0 + 1, to.0) } else { (to.0 + 1, from.0) };
            for x in a .. b {
                positions.push((x, from.1));
            }
        } else if dx == dy {
            let (mut x, mut y) = (from.0 as i8 + sx, from.1 as i8 + sy);
            for _ in 1 .. dx {
                positions.push((x as u8, y as u8));
                x += sx;
                y += sy;
            }
        } else if dx * 2 == dy {
            let (mut px, mut py) = (from.0 as i8, from.1 as i8);
            let steps = dx.min(dy) as i8;
            for i in 0 .. steps {
                if i > 0 {
                    positions.push((px as u8, py as u8));
                }
                let lx = px;
                let ly = py + sy;
                positions.push((lx as u8, ly as u8));
                px += sx;
                py += sy * 2;
            }
        } else if dx == dy * 2 {
            let (mut px, mut py) = (from.0 as i8, from.1 as i8);
            let steps = dx.min(dy) as i8;
            for i in 0 .. steps {
                if i > 0 {
                    positions.push((px as u8, py as u8));
                }
                let lx = px + sx;
                let ly = py;
                positions.push((lx as u8, ly as u8));
                px += sx * 2;
                py += sy;
            }
        } else {
            panic!("unsupported direction");
        }
        positions
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_board(f, self)
    }
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_board(f, self)
    }
}

fn fmt_board(f: &mut dyn Write, board: &Board) -> std::fmt::Result {
    write!(f, "{}[", fmt_num(0))?;
    for x in 0 .. board.width() {
        if x > 0 {
            write!(f, " ")?;
        }
        write!(f, "{}路", fmt_num(x + 1))?;
    }
    writeln!(f, "]")?;
    for y in 0 .. board.height() {
        write!(f, "{}[", fmt_num(y + 1))?;
        for x in 0 .. board.width() {
            if x > 0 {
                write!(f, " ")?;
            }
            match board[(x, y)] {
                Some(p) => write!(f, "{}", p)?,
                None => write!(f, "一一")?,
            }
        }
        writeln!(f, "]")?;
    }
    Ok(())
}

impl FromStr for Board {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        parse_board_from_lines(&mut s.lines())
    }
}

/// Parse a board grid (header row plus piece rows), consuming lines until
/// the first empty line or the end of input.
pub(crate) fn parse_board_from_lines(
    lines: &mut dyn Iterator<Item = &str>,
) -> Result<Board, String> {
    let Some(header) = lines.next() else {
        return Err("missing column header row".into());
    };
    let header = header.trim();
    let header_cells = bracket_cells(header, "零")?;
    let width = header_cells.len();
    if width == 0 {
        return Err("no board columns".into());
    }
    if width > 16 {
        return Err(format!("board has {width} columns, at most 16 supported"));
    }
    for (x, cell) in header_cells.iter().enumerate() {
        let n = fmt_num(x as u8 + 1);
        if !(cell.starts_with(n) && cell.ends_with("路") && cell.len() == n.len() + "路".len()) {
            return Err(format!("column header cell {x} must be {n}路, got {cell}"));
        }
    }

    let mut rows: Vec<Vec<Option<Piece>>> = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if rows.len() >= 16 {
            return Err("board has more than 16 rows, at most 16 supported".into());
        }
        let cells = bracket_cells(line, fmt_num(rows.len() as u8 + 1))?;
        if cells.len() != width {
            return Err(format!("row {} has {} cells, expected {width}", rows.len(), cells.len()));
        }
        let mut row = Vec::new();
        for c in cells {
            match parse_board_piece(c) {
                Ok(p) => row.push(p),
                Err(e) => return Err(format!("row {}: {e}", rows.len())),
            }
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("no board rows".into());
    }

    let mut board = Board::new(width as u8, rows.len() as u8);
    for (y, row) in rows.into_iter().enumerate() {
        for (x, cell) in row.into_iter().enumerate() {
            board[(x as u8, y as u8)] = cell;
        }
    }
    Ok(board)
}

/// Split a `label[cell cell ...]` row into cells, validating the row label
/// and bracket structure.
fn bracket_cells<'s>(line: &'s str, label: &str) -> Result<Vec<&'s str>, String> {
    let Some(rest) = line.strip_prefix(label) else {
        return Err(format!("row must start with label {label}: {line}"));
    };
    let Some(inner) = rest.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Err(format!("row must be bracketed: {line}"));
    };
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for c in inner.split(' ') {
        if c.is_empty() {
            return Err(format!("empty cell in row: {line}"));
        }
        result.push(c);
    }
    Ok(result)
}

/// Parse a single board cell text: `一一` (empty) or a color-prefixed piece.
fn parse_board_piece(s: &str) -> Result<Option<Piece>, String> {
    if s == "一一" {
        return Ok(None);
    }
    match s.parse() {
        Ok(p) => Ok(Some(p)),
        Err(e) => Err(e),
    }
}
