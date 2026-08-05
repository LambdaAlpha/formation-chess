use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;
use std::ops::Index;
use std::ops::IndexMut;
use std::str::FromStr;

use crate::ability::Ability;
use crate::action::Action;
use crate::action::Move;
use crate::action::PositionChange;
use crate::action::PositionChanges;
use crate::chinese_num::fmt_num;
use crate::piece::Color;
use crate::piece::Piece;
use crate::piece::PieceId;
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

    fn change(&self, at: (u8, u8), new: Option<Piece>) -> PositionChange {
        PositionChange { at, old: self[at], new }
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
    pub fn local(&self, (x, y): (u8, u8)) -> Local {
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
        let local = self.local((x, y));
        let mut piece = local.center?;
        piece.take_effect(&local.neighbors);
        Some(piece)
    }

    /// Occupied points and their pieces, in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = ((u8, u8), Piece)> + '_ {
        self.pieces
            .iter()
            .enumerate()
            .filter_map(move |(index, piece)| (*piece).map(|piece| (self.position(index), piece)))
    }
    /// Find the unique point holding `piece`.
    ///
    /// * `Ok(pos)` — exactly one copy at `pos`.
    /// * `Err(false)` — no copy on the board.
    /// * `Err(true)` — more than one copy; the caller must identify the
    ///   piece by coordinates.
    pub fn find_unique(&self, piece: PieceId) -> Result<(u8, u8), bool> {
        let mut pos = None;
        for (i, cell) in self.pieces.iter().enumerate() {
            if cell.is_some_and(|candidate| candidate.id() == piece) {
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
    pub fn place(&mut self, piece: Piece, to: (u8, u8)) -> Result<PositionChanges, String> {
        let changes = self.try_place(piece, to)?;
        self.apply(changes.as_slice());
        Ok(changes)
    }

    /// Validate a placement without modifying the board. Checks bounds,
    /// vacancy, and the half-board rule (red bottom, black top).
    pub fn try_place(&self, piece: Piece, to: (u8, u8)) -> Result<PositionChanges, String> {
        self.check_placement_target(to)?;
        let half = self.height / 2;
        let midpoint = self.height.div_ceil(2);
        if piece.color == Color::Red && to.1 < midpoint {
            return Err("red pieces can only be placed in the bottom half".into());
        }
        if piece.color == Color::Black && to.1 >= half {
            return Err("black pieces can only be placed in the top half".into());
        }
        Ok(PositionChanges::one(self.change(to, Some(piece))))
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
    pub fn move_(&mut self, move_: Move) -> Result<PositionChanges, String> {
        let changes = self.try_move(move_.from, move_.to)?;
        self.apply(changes.as_slice());
        Ok(changes)
    }

    /// Validate a simple move to an empty point without modifying the board.
    /// Checks movement ability, distance, and path blocking.
    pub fn try_move(&self, from: (u8, u8), to: (u8, u8)) -> Result<PositionChanges, String> {
        let _ = self.try_move_to(from, to)?;
        if self[to].is_some() {
            return Err(format!("cannot move onto occupied destination ({},{})", to.0, to.1));
        }
        Ok(PositionChanges::two(self.change(from, None), self.change(to, self[from])))
    }

    /// Validate a divide action without modifying the board. Checks movement,
    /// DIVIDE ability, and that the destination is empty. Returns
    /// changes that place a white piece at `from` and the moving piece at
    /// `to`.
    pub fn try_divide(
        &self, from: (u8, u8), to: (u8, u8), white: Piece,
    ) -> Result<PositionChanges, String> {
        let piece = self.try_move_to(from, to)?;
        if !piece.ability.has(Ability::DIVIDE) {
            return Err("only pieces with DIVIDE ability can divide forces".into());
        }
        if self[to].is_some() {
            return Err(format!("cannot move onto occupied destination ({},{})", to.0, to.1));
        }
        let origin = self.change(from, Some(white));
        let destination = self.change(to, self[from]);
        if origin.old == origin.new {
            return Ok(PositionChanges::one(destination));
        }
        Ok(PositionChanges::two(origin, destination))
    }

    /// Execute a capture (normal or jump, including mutual destruction).
    pub fn capture(&mut self, move_: Move) -> Result<PositionChanges, String> {
        let changes = self.try_capture(move_.from, move_.to)?;
        self.apply(changes.as_slice());
        Ok(changes)
    }

    /// Validate a capture without modifying the board. Checks movement,
    /// path blocking, capture ability, mutual-destruction effects,
    /// and capture demotion.
    pub fn try_capture(&self, from: (u8, u8), to: (u8, u8)) -> Result<PositionChanges, String> {
        let piece = self.try_move_to(from, to)?;
        let Some(target) = self.effective(to) else {
            return Err(format!(
                "destination ({},{}) is empty, capture requires an occupied point",
                to.0, to.1
            ));
        };
        if !piece.can_capture(target) {
            return Err(format!("cannot capture {} at ({},{})", target, to.0, to.1));
        }
        // Capture demotion: if either piece has demotion ability and the
        // pushed target is valid, demote to push instead.
        if (piece.ability.has(Ability::PUSH_ON_CAPTURE_UNBLOCKED)
            || target.ability.has(Ability::PUSHED_ON_CAPTURE_UNBLOCKED))
            && let Some(pt) = self.pushed_target(from, to)
        {
            return Ok(self.push_result(from, to, pt));
        }
        Ok(self.capture_result(piece, from, to, target))
    }

    /// Execute a push (escalates to capture when blocked).
    pub fn push(&mut self, move_: Move) -> Result<PositionChanges, String> {
        let changes = self.try_push(move_.from, move_.to)?;
        self.apply(changes.as_slice());
        Ok(changes)
    }

    /// Validate a push without modifying the board. Checks movement,
    /// path blocking, push ability, and the pushed piece's landing.
    /// A blocked push may escalate to capture if either piece has the
    /// escalation ability.
    pub fn try_push(&self, from: (u8, u8), to: (u8, u8)) -> Result<PositionChanges, String> {
        let piece = self.try_move_to(from, to)?;
        let Some(target) = self.effective(to) else {
            return Err(format!(
                "destination ({},{}) is empty, push requires an occupied point",
                to.0, to.1
            ));
        };
        if !piece.can_push(target) {
            return Err(format!("cannot push {} at ({},{})", target, to.0, to.1));
        }
        if let Some(pt) = self.pushed_target(from, to) {
            return Ok(self.push_result(from, to, pt));
        }
        // Push is blocked. Escalate to capture if either piece has
        // escalation ability; otherwise fail.
        if piece.ability.has(Ability::CAPTURE_ON_PUSH_BLOCKED)
            || target.ability.has(Ability::CAPTURED_ON_PUSH_BLOCKED)
        {
            Ok(self.capture_result(piece, from, to, target))
        } else {
            Err(format!("push blocked at ({},{}) and no escalation ability", to.0, to.1))
        }
    }

    /// Validate a draw without modifying the board. Checks movement,
    /// path blocking, that the mover has DRAW ability, and that the target
    /// is a vital piece. A piece with DRAW may move onto an opponent's vital
    /// piece to end the game in a draw without needing capture or push
    /// abilities.
    pub fn try_draw(&self, from: (u8, u8), to: (u8, u8)) -> Result<PositionChanges, String> {
        let piece = self.try_move_to(from, to)?;
        let Some(target) = self.get(to) else {
            return Err(format!(
                "destination ({},{}) is empty, draw requires an occupied point",
                to.0, to.1
            ));
        };
        if !piece.ability.has(Ability::DRAW) {
            return Err("only pieces with DRAW ability can draw".into());
        }
        if !target.ability.has(Ability::VITAL) {
            return Err(format!("{} at ({},{}) is not a vital piece", target, to.0, to.1));
        }
        let departure = self.change(from, None);
        let arrival = self.change(to, self[from]);
        if arrival.old == arrival.new {
            return Ok(PositionChanges::one(departure));
        }
        Ok(PositionChanges::two(departure, arrival))
    }

    /// Shared pre-checks: bounds validation, piece lookup, movement ability,
    /// path check. Returns the effective piece at `from` on success.
    fn try_move_to(&self, from: (u8, u8), to: (u8, u8)) -> Result<Piece, String> {
        if !self.in_bounds(from) {
            return Err(format!("({},{}) is outside the board", from.0, from.1));
        }
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        let Some(piece) = self.effective(from) else {
            return Err(format!("no piece at ({},{})", from.0, from.1));
        };
        if !Self::can_move(piece, from, to) {
            return Err(format!(
                "piece {piece} at ({},{}) cannot move to ({},{})",
                from.0, from.1, to.0, to.1
            ));
        }
        if !Self::path_passable(self, from, to) {
            return Err("path blocked, cannot reach destination".into());
        }
        Ok(piece)
    }

    /// Whether the single step from `from` to `to` is clear. For
    /// cross/diagonal steps there are no intermediate points — always true.
    /// For L-shaped (knight) steps, checks that the leg-blocking point is
    /// empty.
    fn step_passable(&self, from: (u8, u8), to: (u8, u8)) -> bool {
        let dx = from.0.abs_diff(to.0);
        let dy = from.1.abs_diff(to.1);
        if dx <= 1 && dy <= 1 {
            return true;
        }
        if dx == 1 && dy == 2 {
            let sy: i8 = (to.1 as i8 - from.1 as i8).signum();
            self[(from.0, (from.1 as i8 + sy) as u8)].is_none()
        } else if dx == 2 && dy == 1 {
            let sx: i8 = (to.0 as i8 - from.0 as i8).signum();
            self[((from.0 as i8 + sx) as u8, from.1)].is_none()
        } else {
            panic!("unsupported direction for single step")
        }
    }

    /// Whether the path between `from` and `to` is free of pieces. Returns
    /// true for adjacent points and points connected by a clean line.
    fn path_passable(&self, from: (u8, u8), to: (u8, u8)) -> bool {
        let dx = from.0.abs_diff(to.0);
        let dy = from.1.abs_diff(to.1);
        let sx: i8 = (to.0 as i8 - from.0 as i8).signum();
        let sy: i8 = (to.1 as i8 - from.1 as i8).signum();
        if dx == 0 {
            let (a, b) = if from.1 < to.1 { (from.1 + 1, to.1) } else { (to.1 + 1, from.1) };
            for y in a .. b {
                if self[(from.0, y)].is_some() {
                    return false;
                }
            }
        } else if dy == 0 {
            let (a, b) = if from.0 < to.0 { (from.0 + 1, to.0) } else { (to.0 + 1, from.0) };
            for x in a .. b {
                if self[(x, from.1)].is_some() {
                    return false;
                }
            }
        } else if dx == dy {
            let (mut x, mut y) = (from.0 as i8 + sx, from.1 as i8 + sy);
            for _ in 1 .. dx {
                if self[(x as u8, y as u8)].is_some() {
                    return false;
                }
                x += sx;
                y += sy;
            }
        } else if dx * 2 == dy {
            let (mut px, mut py) = (from.0 as i8, from.1 as i8);
            let steps = dx.min(dy) as i8;
            for i in 0 .. steps {
                if i > 0 && self[(px as u8, py as u8)].is_some() {
                    return false;
                }
                let lx = px;
                let ly = py + sy;
                if self[(lx as u8, ly as u8)].is_some() {
                    return false;
                }
                px += sx;
                py += sy * 2;
            }
        } else if dx == dy * 2 {
            let (mut px, mut py) = (from.0 as i8, from.1 as i8);
            let steps = dx.min(dy) as i8;
            for i in 0 .. steps {
                if i > 0 && self[(px as u8, py as u8)].is_some() {
                    return false;
                }
                let lx = px + sx;
                let ly = py;
                if self[(lx as u8, ly as u8)].is_some() {
                    return false;
                }
                px += sx * 2;
                py += sy;
            }
        } else {
            panic!("unsupported direction");
        }
        true
    }

    /// Where the pushed piece would land, continuing one step along the push
    /// direction. The pushed piece's own direction abilities are irrelevant:
    /// the shove supplies the movement. Returns None if the pushed piece
    /// cannot make that step: the landing point is off the board or occupied,
    /// or the pushed piece's path there is blocked (for L-shaped pushes this
    /// is the pushed piece's leg).
    fn pushed_target(&self, from: (u8, u8), to: (u8, u8)) -> Option<(u8, u8)> {
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
        // The push always moves the target a single step: adjacent for
        // cross/diagonal, one knight-step for L-shaped. `step_passable`
        // suffices — cross/diagonal steps have no intermediate points,
        // and L-shaped steps only need the leg checked.
        if !self.step_passable(to, pt) {
            return None;
        }
        Some(pt)
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
    /// distance only. Path blocking is checked separately by
    /// [`Self::path_passable`].
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
        if is_cross && !piece.ability.has(Ability::DIRECTION_CROSS) {
            return false;
        }
        if is_diagonal && !piece.ability.has(Ability::DIRECTION_DIAGONAL) {
            return false;
        }
        if is_shape_L && !piece.ability.has(Ability::DIRECTION_SHAPE_L) {
            return false;
        }
        if piece.ability.has(Ability::ANY_DISTANCE) {
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

    fn push_result(&self, from: (u8, u8), to: (u8, u8), pushed_to: (u8, u8)) -> PositionChanges {
        let departure = self.change(from, None);
        let arrival = self.change(to, self[from]);
        let pushed = self.change(pushed_to, self[to]);
        if arrival.old == arrival.new {
            return PositionChanges::two(departure, pushed);
        }
        PositionChanges::three(departure, arrival, pushed)
    }
    /// Compute the changes of a successful capture, including
    /// mutual‑destruction effects.
    fn capture_result(
        &self, piece: Piece, from: (u8, u8), to: (u8, u8), target: Piece,
    ) -> PositionChanges {
        let departure = self.change(from, None);
        if piece.ability.has(Ability::CAPTURED_ON_CAPTURE)
            || target.ability.has(Ability::CAPTURE_ON_CAPTURED)
        {
            return PositionChanges::two(departure, self.change(to, None));
        }

        let arrival = self.change(to, self[from]);
        if arrival.old == arrival.new {
            return PositionChanges::one(departure);
        }
        PositionChanges::two(departure, arrival)
    }

    /// Append all legal actions for the piece at `from` to `actions`. The piece
    /// must be present on the board. Actions are [`Action::Move`],
    /// [`Action::Capture`], [`Action::Push`], [`Action::Draw`], and
    /// [`Action::Divide`]; placement, pass, and resign are the caller's concern.
    ///
    /// `player` filters draw actions: only draws targeting the opponent's
    /// colored pieces are appended.
    pub fn valid_moves(
        &self, player: Player, from: (u8, u8), has_white: bool, actions: &mut Vec<Action>,
    ) {
        let Some(piece) = self.effective(from) else {
            return;
        };
        let max = if piece.ability.has(Ability::ANY_DISTANCE) {
            self.width.max(self.height) as i8
        } else {
            1
        };
        if piece.ability.has(Ability::DIRECTION_CROSS) {
            for (dx, dy) in [(0i8, -1), (0, 1), (-1, 0), (1, 0)] {
                self.enumerate_line(player, piece, from, dx, dy, max, has_white, actions);
            }
        }
        if piece.ability.has(Ability::DIRECTION_DIAGONAL) {
            for (dx, dy) in [(-1i8, -1), (1, -1), (-1, 1), (1, 1)] {
                self.enumerate_line(player, piece, from, dx, dy, max, has_white, actions);
            }
        }
        if piece.ability.has(Ability::DIRECTION_SHAPE_L) {
            for (dx, dy) in
                [(1i8, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1)]
            {
                self.enumerate_line(player, piece, from, dx, dy, max, has_white, actions);
            }
        }
    }

    /// Scan from `from` along `(dx, dy)`, adding legal [`Action`]s for each
    /// reachable cell. Path blocking is checked per step segment.
    #[expect(clippy::too_many_arguments)]
    fn enumerate_line(
        &self, player: Player, piece: Piece, from: (u8, u8), dx: i8, dy: i8, max_steps: i8,
        has_white: bool, actions: &mut Vec<Action>,
    ) {
        let mut origin = from;
        for _ in 0 .. max_steps {
            let nx = origin.0 as i8 + dx;
            let ny = origin.1 as i8 + dy;
            if nx < 0 || ny < 0 || nx as u8 >= self.width || ny as u8 >= self.height {
                break;
            }
            let to = (nx as u8, ny as u8);
            if !self.step_passable(origin, to) {
                break;
            }
            if let Some(target) = self.effective(to) {
                self.enumerate_action(player, piece, from, to, target, actions);
                break;
            }
            actions.push(Action::Move(Move { from, to }));
            if has_white && piece.ability.has(Ability::DIVIDE) {
                actions.push(Action::Divide(Move { from, to }));
            }
            origin = to;
        }
    }

    /// Add every capture, push, and draw action legal against `target`.
    /// Path blocking is already handled by [`Self::enumerate_line`].
    fn enumerate_action(
        &self, player: Player, piece: Piece, from: (u8, u8), to: (u8, u8), target: Piece,
        actions: &mut Vec<Action>,
    ) {
        let move_ = Move { from, to };
        if piece.can_capture(target) {
            actions.push(Action::Capture(move_));
        }
        if piece.can_push(target) {
            let push_blocked = self.pushed_target(from, to).is_none();
            if !push_blocked
                || piece.ability.has(Ability::CAPTURE_ON_PUSH_BLOCKED)
                || target.ability.has(Ability::CAPTURED_ON_PUSH_BLOCKED)
            {
                actions.push(Action::Push(move_));
            }
        }
        let opponent_color = match player {
            Player::Red => Color::Black,
            Player::Black => Color::Red,
        };
        if piece.ability.has(Ability::DRAW)
            && target.ability.has(Ability::VITAL)
            && target.color == opponent_color
        {
            actions.push(Action::Draw(move_));
        }
    }

    /// Write the new side of position changes onto the board. Panics when a change is
    /// outside the board.
    pub fn apply(&mut self, changes: &[PositionChange]) {
        for change in changes {
            self[change.at] = change.new;
        }
    }

    /// Write the old side of position changes back onto the board. Panics when
    /// a change is outside the board.
    pub fn undo(&mut self, changes: &[PositionChange]) {
        for change in changes {
            self[change.at] = change.old;
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
                if pos.new.is_none() && change.new.is_some() {
                    pos.new = change.new;
                }
            } else {
                result.push(*change);
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
    let Some(inner) = rest.strip_prefix('[') else {
        return Err(format!("row must be bracketed: {line}"));
    };
    let Some(inner) = inner.strip_suffix(']') else {
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
