use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;
use std::ops::Index;
use std::ops::IndexMut;
use std::str::FromStr;

use crate::ability::Ability;
use crate::action::Move;
use crate::action::PieceChange;
use crate::action::Place;
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

/// The outcome of a move on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveOutcome {
    pub changes: Vec<PieceChange>,
    /// Number of pieces removed from the board by capture.
    pub captured: u8,
}

/// The new content of a single point: a piece, or None for empty.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PositionChange {
    pub at: (u8, u8),
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

    /// Move a piece from→to. Clears from.
    fn move_(&mut self, from: (u8, u8), to: (u8, u8)) {
        self[to] = self[from];
        self[from] = None;
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
    pub fn find_vital(&self, color: Color) -> Option<((u8, u8), Piece)> {
        self.pieces.iter().enumerate().find_map(|(i, cell)| {
            cell.filter(|p| p.color == color && p.ability.has_ability(Ability::VITAL))
                .map(|p| (self.position(i), p))
        })
    }

    /// Number of pieces of `color` with the VITAL ability.
    pub fn vital_count(&self, color: Color) -> usize {
        self.pieces
            .iter()
            .flatten()
            .filter(|p| p.color == color && p.ability.has_ability(Ability::VITAL))
            .count()
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
        pos.ok_or(false)
    }

    /// Whether from→to lies on a horizontal or vertical line. Note:
    /// `from == to` satisfies all three direction predicates; callers must
    /// exclude it first.
    pub fn is_direction_cross(from: (u8, u8), to: (u8, u8)) -> bool {
        from.0 == to.0 || from.1 == to.1
    }

    /// Whether from→to lies on a diagonal line. See the `from == to` note
    /// on [`Self::is_direction_cross`].
    pub fn is_direction_diagonal(from: (u8, u8), to: (u8, u8)) -> bool {
        from.0.abs_diff(to.0) == from.1.abs_diff(to.1)
    }

    /// Whether from→to lies on a knight line (1:2 slope, including chained
    /// knight moves). See the `from == to` note on
    /// [`Self::is_direction_cross`].
    #[expect(non_snake_case)]
    pub fn is_direction_shape_L(from: (u8, u8), to: (u8, u8)) -> bool {
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
    pub fn can_move(piece: &Piece, from: (u8, u8), to: (u8, u8)) -> bool {
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

    /// Place a red or black piece onto an empty point in its own half
    /// (the placement-phase rule: red in the bottom half, black in the top
    /// half, the center row of odd-height boards in neither). On error,
    /// self is unchanged.
    pub fn place(&mut self, piece: Piece, to: (u8, u8)) -> Result<Vec<PieceChange>, String> {
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        if self[to].is_some() {
            return Err(format!("destination ({},{}) is already occupied", to.0, to.1));
        }
        let half = self.height / 2;
        let midpoint = self.height.div_ceil(2);
        if piece.color == Color::Red && to.1 < midpoint {
            return Err("red pieces can only be placed in the bottom half".into());
        }
        if piece.color == Color::Black && to.1 >= half {
            return Err("black pieces can only be placed in the top half".into());
        }
        self[to] = Some(piece);
        Ok(vec![PieceChange::Place(Place { piece, to })])
    }

    /// Place a white piece on an empty point covered by the formation of a
    /// piece with CONTROL_WHITE that the given player controls. Raw abilities
    /// suffice: no formation modifies CONTROL_WHITE, and control granted by
    /// formations does not extend to placing white pieces. On error, self is
    /// unchanged.
    pub fn place_white(
        &mut self, white: Piece, to: (u8, u8), player: Player,
    ) -> Result<Vec<PieceChange>, String> {
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        if self[to].is_some() {
            return Err(format!("destination ({},{}) is already occupied", to.0, to.1));
        }
        let has_control = self.local(to.0, to.1).neighbors.iter().any(|n| {
            n.piece.is_some_and(|piece| {
                piece.formation.contains(-n.dx, -n.dy)
                    && piece.ability.has_ability(Ability::CONTROL_WHITE)
                    && piece.can_controlled_by(player)
            })
        });
        if !has_control {
            return Err(format!(
                "({},{}) is not covered by any piece with CONTROL_WHITE controlled by player {}",
                to.0, to.1, player
            ));
        }
        self[to] = Some(white);
        Ok(vec![PieceChange::Place(Place { piece: white, to })])
    }

    /// Execute a simple move to an empty point (no capture or push intent).
    /// Validates movement and pass-through. On error, self is unchanged.
    pub fn move_to(&mut self, move_: Move) -> Result<MoveOutcome, String> {
        let info = self.move_info(move_)?;
        if self[info.to].is_some() {
            return Err(format!(
                "cannot move onto occupied destination ({},{})",
                info.to.0, info.to.1
            ));
        }
        self.try_move(info.from, info.to, &info.path)
    }

    /// Execute a push: the mover attempts to shove the piece at the
    /// destination. Validates movement, pass-through, push ability, and the
    /// pushed piece's own path. Escalates to capture when the push is
    /// blocked. On error, self is unchanged.
    pub fn move_push(&mut self, move_: Move) -> Result<MoveOutcome, String> {
        let info = self.move_info(move_)?;
        let target = self.effective(info.to).ok_or_else(|| {
            format!(
                "destination ({},{}) is empty, push requires an occupied point",
                info.to.0, info.to.1
            )
        })?;
        self.try_push(info.from, info.to, &info.mover, &target, &info.path)
    }

    /// Execute a capture: the mover attempts to remove the piece at the
    /// destination. Validates movement, pass-through, and capture ability.
    /// On error, self is unchanged.
    pub fn move_capture(&mut self, move_: Move) -> Result<MoveOutcome, String> {
        let info = self.move_info(move_)?;
        let target = self.effective(info.to).ok_or_else(|| {
            format!(
                "destination ({},{}) is empty, capture requires an occupied point",
                info.to.0, info.to.1
            )
        })?;
        self.try_capture(info.from, info.to, &info.mover, &target, &info.path)
    }

    /// Shared pre-checks: bounds validation, piece lookup, movement ability,
    /// path computation.
    fn move_info(&self, move_: Move) -> Result<MoveInfo, String> {
        let from = move_.from;
        let to = move_.to;
        if !self.in_bounds(from) {
            return Err(format!("({},{}) is outside the board", from.0, from.1));
        }
        if !self.in_bounds(to) {
            return Err(format!("({},{}) is outside the board", to.0, to.1));
        }
        let mover =
            self.effective(from).ok_or_else(|| format!("no piece at ({},{})", from.0, from.1))?;
        if !Self::can_move(&mover, from, to) {
            return Err(format!(
                "piece {mover} at ({},{}) cannot move to ({},{})",
                from.0, from.1, to.0, to.1
            ));
        }
        let path = MovePath::new(self, &mover, from, to);
        Ok(MoveInfo { from, to, mover, path })
    }

    fn try_move(
        &mut self, from: (u8, u8), to: (u8, u8), path: &MovePath,
    ) -> Result<MoveOutcome, String> {
        if path.unpassable > 0 {
            return Err("path blocked, cannot reach empty destination".into());
        }
        self.move_(from, to);
        Ok(MoveOutcome { changes: vec![PieceChange::Move(Move { from, to })], captured: 0 })
    }

    fn try_capture(
        &mut self, from: (u8, u8), to: (u8, u8), mover: &Piece, target: &Piece, path: &MovePath,
    ) -> Result<MoveOutcome, String> {
        // Normal capture requires a fully passable path. Jump capture requires
        // exactly one piece on the path (checked by can_jump_capture), no
        // matter whether that piece is passable.
        let normal_capture = path.unpassable == 0 && mover.can_capture(target);
        let jump_capture = mover.can_jump_capture(target, path.pieces);
        if !normal_capture && !jump_capture {
            return Err(format!("cannot capture {} at ({},{})", target, to.0, to.1));
        }
        Ok(self.execute_capture(from, to, mover, target))
    }

    /// Execute a capture: remove target, move attacker, apply
    /// mutual-destruction effects.
    fn execute_capture(
        &mut self, from: (u8, u8), to: (u8, u8), mover: &Piece, target: &Piece,
    ) -> MoveOutcome {
        self[to] = None;
        self.move_(from, to);
        if mover.ability.has_ability(Ability::CAPTURED_ON_CAPTURE)
            || target.ability.has_ability(Ability::CAPTURE_ON_CAPTURED)
        {
            self[to] = None;
            MoveOutcome {
                changes: vec![PieceChange::Remove(from.0, from.1), PieceChange::Remove(to.0, to.1)],
                captured: 2,
            }
        } else {
            MoveOutcome { changes: vec![PieceChange::Move(Move { from, to })], captured: 1 }
        }
    }

    fn try_push(
        &mut self, from: (u8, u8), to: (u8, u8), mover: &Piece, target: &Piece, path: &MovePath,
    ) -> Result<MoveOutcome, String> {
        if path.unpassable > 0 {
            return Err("cannot push through blocking pieces on path".into());
        }
        if !mover.can_push(target) {
            return Err(format!("cannot push {} at ({},{})", target, to.0, to.1));
        }
        if let Some(pt) = self.pushed_target(from, to, target) {
            self.move_(to, pt);
            self.move_(from, to);
            return Ok(MoveOutcome {
                changes: vec![
                    PieceChange::Move(Move { from, to }),
                    PieceChange::Move(Move { from: to, to: pt }),
                ],
                captured: 0,
            });
        }
        if mover.color != target.color && target.ability.has_ability(Ability::CAPTURED) {
            return Ok(self.execute_capture(from, to, mover, target));
        }
        Err(format!("push blocked and cannot capture {} at ({},{})", target, to.0, to.1))
    }

    /// Where the pushed piece would land, continuing one step along the push
    /// direction. The pushed piece's own direction abilities are irrelevant:
    /// the shove supplies the movement. Returns None if the pushed piece
    /// cannot make that step: the landing point is off the board or occupied,
    /// or the pushed piece cannot traverse its own path there (the same
    /// pass-through rules as a normal move; for L-shaped pushes this is the
    /// pushed piece's leg).
    fn pushed_target(&self, from: (u8, u8), to: (u8, u8), pushed: &Piece) -> Option<(u8, u8)> {
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

    /// Write position-based changes (produced by [`Self::resolve_changes`])
    /// onto the board. Panics when a change lies outside the board.
    pub fn apply(&mut self, changes: &[PositionChange]) {
        for change in changes {
            self[change.at] = change.piece;
        }
    }

    /// Convert piece-based changes into position-based changes. `self` must
    /// be the board as it stood when the action was executed. Points that a
    /// piece left or was removed from become empty unless another piece
    /// arrived there, so cyclic changes (e.g. two pieces swapping points)
    /// resolve correctly.
    pub fn resolve_changes(&self, changes: &[PieceChange]) -> Result<Vec<PositionChange>, String> {
        let mut vacated: Vec<(u8, u8)> = Vec::new();
        let mut occupied: Vec<((u8, u8), Piece)> = Vec::new();
        for change in changes {
            match *change {
                PieceChange::Move(Move { from, to }) => {
                    let piece = self
                        .get(from)
                        .ok_or_else(|| format!("no piece at ({},{})", from.0, from.1))?;
                    if !self.in_bounds(to) {
                        return Err(format!("({},{}) is outside the board", to.0, to.1));
                    }
                    vacated.push(from);
                    occupied.push((to, piece));
                },
                PieceChange::Place(Place { piece, to }) => {
                    if !self.in_bounds(to) {
                        return Err(format!("({},{}) is outside the board", to.0, to.1));
                    }
                    occupied.push((to, piece));
                },
                PieceChange::Remove(x, y) => {
                    if self.get((x, y)).is_none() {
                        return Err(format!("no piece at ({x},{y})"));
                    }
                    vacated.push((x, y));
                },
            }
        }
        let mut result: Vec<PositionChange> = Vec::new();
        for (i, &(at, piece)) in occupied.iter().enumerate() {
            if occupied[.. i].iter().any(|&(other, _)| other == at) {
                return Err(format!("conflicting changes at ({},{})", at.0, at.1));
            }
            result.push(PositionChange { at, piece: Some(piece) });
        }
        for at in vacated {
            if !occupied.iter().any(|&(other, _)| other == at) {
                result.push(PositionChange { at, piece: None });
            }
        }
        result.sort_by_key(|c| c.at);
        result.dedup();
        Ok(result)
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
    fn new(board: &Board, mover: &Piece, from: (u8, u8), to: (u8, u8)) -> Self {
        let mut pieces: u8 = 0;
        let mut unpassable: u8 = 0;
        for pos in Self::path_positions(from, to) {
            if let Some(blocker) = board.effective(pos) {
                pieces += 1;
                if !mover.can_pass(&blocker) {
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
    write!(f, "零[")?;
    for x in 0 .. board.width() {
        if x > 0 {
            write!(f, " ")?;
        }
        let n = fmt_num(x + 1);
        write!(f, "{n}{n}")?;
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
    let header = lines.next().ok_or("missing column header row")?.trim();
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
        if *cell != format!("{n}{n}") {
            return Err(format!("column header cell {x} must be {n}{n}, got {cell}"));
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
        let row = cells
            .iter()
            .map(|c| parse_board_piece(c))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("row {}: {e}", rows.len()))?;
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
    let rest = line
        .strip_prefix(label)
        .ok_or_else(|| format!("row must start with label {label}: {line}"))?;
    let inner = rest
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| format!("row must be bracketed: {line}"))?;
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(' ')
        .map(|c| if c.is_empty() { Err(format!("empty cell in row: {line}")) } else { Ok(c) })
        .collect()
}

/// Parse a single board cell text: `一一` (empty) or a color-prefixed piece.
fn parse_board_piece(s: &str) -> Result<Option<Piece>, String> {
    if s == "一一" {
        return Ok(None);
    }
    s.parse().map(Some)
}
