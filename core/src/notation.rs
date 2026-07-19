use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use crate::action::Action;
use crate::action::GameResult;
use crate::action::Move;
use crate::action::PieceChange;
use crate::action::Place;
use crate::action::Reaction;
use crate::board::Board;
use crate::chinese_num::fmt_num;
use crate::chinese_num::parse_num;
use crate::piece::Color;
use crate::piece::Piece;
use crate::piece::Player;

/// Resolves between 1‑based notation types and 0‑based API types, using a
/// board for piece lookups and bounds checks. When formatting or parsing
/// the changes of an already-executed action, the resolver must hold the
/// board as it stood **before** that action.
pub struct NotationResolver<'a> {
    board: &'a Board,
}

/// An action as written in notation, e.g. `红车平五推` or `黑认负`.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionNotation {
    /// Placement with the `占` suffix.
    Place(PlaceNotation),
    /// Piece + position without a suffix: a move, or a placement when the
    /// piece is not on the board.
    Move(PiecePosition),
    /// Piece + position + `捉`.
    Capture(PiecePosition),
    /// Piece + position + `推`.
    Push(PiecePosition),
    /// Color + `按兵`.
    Pass(Player),
    /// Color + `认负`.
    Resign(Player),
}

/// An action result as written in notation: either a `变化：`/`胜负：`
/// pair, or a single `错误：` line.
#[derive(Debug, Clone, PartialEq)]
pub enum ReactionNotation {
    Changes {
        changes: Vec<ChangeNotation>,
        game_result: GameResult,
    },
    /// The message must be a single line: the text protocol renders an
    /// error reaction as exactly one `错误：` line. Keep every error
    /// message produced by this crate free of newlines.
    Error(String),
}

/// One entry of a `变化：` list, expressed against the pre-action board.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeNotation {
    /// Piece + absolute position + `占`: the piece arrived from off-board.
    Place(PlaceNotation),
    /// Piece + `提`: the piece left the board and nothing replaced it.
    Remove(PieceNotation),
    /// Piece + position: the piece now stands there. Doubles as a
    /// placement when the piece is not on the board.
    Move(PiecePosition),
}

/// A placement in notation: a canonical piece and a 1-based destination.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceNotation {
    pub piece: Piece,
    pub to: (u8, u8),
}

/// A piece reference followed by where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct PiecePosition {
    pub piece: PieceNotation,
    pub position: Position,
}

/// How notation refers to a piece: by color-prefixed name (`红车`, only
/// unambiguous when unique on the board) or by the 1-based coordinates of
/// its current point (`一二`).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PieceNotation {
    Name(Piece),
    Coord(u8, u8),
}

/// A destination in notation; all coordinates are 1-based.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Position {
    Relative(RelativePosition),
    Absolute(u8, u8),
}

/// A destination relative to the piece's current point; coordinates and
/// steps are 1-based.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RelativePosition {
    /// `平` + column: another column on the same row.
    Horizontal(u8),
    /// `直` + row: another row in the same column.
    Straight(u8),
    /// `进` + steps: forward — up for Red, down for Black, invalid for
    /// White.
    Advance(u8),
    /// `退` + steps: backward — down for Red, up for Black, invalid for
    /// White.
    Retreat(u8),
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let na = match self {
            Action::Place(p) => ActionNotation::Place(place_notation(p)),
            Action::Move(m) => ActionNotation::Move(piece_position(*m)),
            Action::Capture(m) => ActionNotation::Capture(piece_position(*m)),
            Action::Push(m) => ActionNotation::Push(piece_position(*m)),
            Action::Pass(p) => ActionNotation::Pass(*p),
            Action::Resign(p) => ActionNotation::Resign(*p),
        };
        write!(f, "{na}")
    }
}

impl Display for Reaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rn = ReactionNotation::Changes {
            changes: self.changes.iter().map(change_notation).collect(),
            game_result: self.game_result,
        };
        write!(f, "{rn}")
    }
}

impl Display for PieceChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", change_notation(self))
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", place_notation(self))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", piece_position(*self))
    }
}

fn piece_position(m: Move) -> PiecePosition {
    PiecePosition {
        piece: PieceNotation::Coord(m.from.0 + 1, m.from.1 + 1),
        position: Position::Absolute(m.to.0 + 1, m.to.1 + 1),
    }
}

fn place_notation(p: &Place) -> PlaceNotation {
    PlaceNotation { piece: p.piece, to: (p.to.0 + 1, p.to.1 + 1) }
}

fn change_notation(c: &PieceChange) -> ChangeNotation {
    match c {
        PieceChange::Move(m) => ChangeNotation::Move(piece_position(*m)),
        PieceChange::Place(p) => ChangeNotation::Place(place_notation(p)),
        PieceChange::Remove(x, y) => ChangeNotation::Remove(PieceNotation::Coord(x + 1, y + 1)),
    }
}

impl Display for ActionNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionNotation::Place(p) => write!(f, "{p}"),
            ActionNotation::Move(pp) => write!(f, "{pp}"),
            ActionNotation::Capture(pp) => write!(f, "{pp}捉"),
            ActionNotation::Push(pp) => write!(f, "{pp}推"),
            ActionNotation::Pass(player) => write!(f, "{player}按兵"),
            ActionNotation::Resign(player) => write!(f, "{player}认负"),
        }
    }
}

impl FromStr for ActionNotation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("empty action".into());
        }
        if let Some(player) = s.strip_suffix("按兵") {
            return Ok(Self::Pass(player.parse()?));
        }
        if let Some(player) = s.strip_suffix("认负") {
            return Ok(Self::Resign(player.parse()?));
        }
        if s.ends_with('占') {
            return Ok(Self::Place(s.parse()?));
        }
        if let Some(body) = s.strip_suffix('捉') {
            return Ok(Self::Capture(body.parse()?));
        }
        if let Some(body) = s.strip_suffix('推') {
            return Ok(Self::Push(body.parse()?));
        }
        Ok(Self::Move(s.parse()?))
    }
}

impl Display for ReactionNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReactionNotation::Error(msg) => write!(f, "错误：{msg}"),
            ReactionNotation::Changes { changes, game_result } => {
                write!(f, "变化：[")?;
                for (i, c) in changes.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{c}")?;
                }
                write!(f, "]\n胜负：{game_result}")
            },
        }
    }
}

impl FromStr for ReactionNotation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if let Some(rest) = s.strip_prefix("错误：") {
            return Ok(Self::Error(rest.to_string()));
        }
        let (changes_part, result_part) =
            s.split_once('\n').ok_or_else(|| "missing result line".to_string())?;
        let changes_str = changes_part
            .strip_prefix("变化：[")
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| format!("invalid changes prefix: {changes_part}"))?;

        let changes = if changes_str.is_empty() {
            Vec::new()
        } else {
            changes_str
                .split_whitespace()
                .map(str::parse::<ChangeNotation>)
                .collect::<Result<Vec<_>, _>>()?
        };

        let result_str = result_part
            .strip_prefix("胜负：")
            .ok_or_else(|| format!("invalid result line: {result_part}"))?;
        let game_result = result_str.trim().parse()?;

        Ok(Self::Changes { changes, game_result })
    }
}

impl Display for ChangeNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeNotation::Place(p) => write!(f, "{p}"),
            ChangeNotation::Remove(piece) => write!(f, "{piece}提"),
            ChangeNotation::Move(pp) => write!(f, "{pp}"),
        }
    }
}

impl FromStr for ChangeNotation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("empty change entry".into());
        }
        if let Some(body) = s.strip_suffix('提') {
            return Ok(Self::Remove(body.parse()?));
        }
        if s.ends_with('占') {
            return Ok(Self::Place(s.parse()?));
        }
        Ok(Self::Move(s.parse()?))
    }
}

impl Display for PlaceNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}占", self.piece, fmt_num(self.to.0), fmt_num(self.to.1))
    }
}

impl FromStr for PlaceNotation {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let body =
            s.strip_suffix('占').ok_or_else(|| format!("placement must end with 占: {s}"))?;
        let (pn, rest) = split_at_2(body)?;
        let p = match pn {
            PieceNotation::Name(p) => p,
            PieceNotation::Coord(_, _) => {
                return Err("placement requires a color-prefixed piece name".into());
            },
        };
        let to = match rest.parse::<Position>()? {
            Position::Absolute(c, r) => (c, r),
            Position::Relative(_) => return Err("placement requires an absolute position".into()),
        };
        Ok(PlaceNotation { piece: p, to })
    }
}

impl Display for PiecePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.piece, self.position)
    }
}

impl FromStr for PiecePosition {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (piece, rest) = split_at_2(s)?;
        let position = rest.parse()?;
        Ok(PiecePosition { piece, position })
    }
}

impl Display for PieceNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PieceNotation::Name(p) => write!(f, "{p}"),
            PieceNotation::Coord(col, row) => write!(f, "{}{}", fmt_num(*col), fmt_num(*row)),
        }
    }
}

impl FromStr for PieceNotation {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        if s.chars().count() != 2 {
            return Err(format!("piece reference must be 2 characters: {s}"));
        }
        if s.starts_with('红') || s.starts_with('黑') || s.starts_with('白') {
            return Ok(PieceNotation::Name(s.parse()?));
        }
        let (col, row) = parse_two_numbers(s)?;
        Ok(PieceNotation::Coord(col, row))
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Position::Absolute(col, row) => write!(f, "{}{}", fmt_num(*col), fmt_num(*row)),
            Position::Relative(rp) => write!(f, "{rp}"),
        }
    }
}

impl FromStr for Position {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        if s.chars().count() != 2 {
            return Err(format!("invalid position: {s}"));
        }
        if s.starts_with('平') || s.starts_with('进') || s.starts_with('退') || s.starts_with('直')
        {
            return Ok(Position::Relative(s.parse()?));
        }
        let (col, row) = parse_two_numbers(s)?;
        Ok(Position::Absolute(col, row))
    }
}

impl Display for RelativePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelativePosition::Horizontal(col) => write!(f, "平{}", fmt_num(*col)),
            RelativePosition::Straight(row) => write!(f, "直{}", fmt_num(*row)),
            RelativePosition::Advance(steps) => write!(f, "进{}", fmt_num(*steps)),
            RelativePosition::Retreat(steps) => write!(f, "退{}", fmt_num(*steps)),
        }
    }
}

impl FromStr for RelativePosition {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let mut iter = s.char_indices();
        let (_, c0) = iter.next().ok_or_else(|| format!("invalid relative position: {s}"))?;
        let Some((i1, c1)) = iter.next() else {
            return Err(format!("invalid relative position: {s}"));
        };
        if iter.next().is_some() {
            return Err(format!("invalid relative position: {s}"));
        }
        let n = parse_num(&s[i1 .. i1 + c1.len_utf8()])
            .ok_or_else(|| format!("not a coordinate digit: {c1}"))?;
        match c0 {
            '平' => Ok(RelativePosition::Horizontal(n)),
            '进' => Ok(RelativePosition::Advance(n)),
            '退' => Ok(RelativePosition::Retreat(n)),
            '直' => Ok(RelativePosition::Straight(n)),
            c => Err(format!("unknown relative keyword: {c}")),
        }
    }
}

fn split_at_2(s: &str) -> Result<(PieceNotation, &str), String> {
    let Some((off, _)) = s.char_indices().nth(2) else {
        return Err(format!("too short for piece+position: {s}"));
    };
    let piece: PieceNotation = s[.. off].parse()?;
    Ok((piece, &s[off ..]))
}

fn parse_two_numbers(s: &str) -> Result<(u8, u8), String> {
    let mut iter = s.char_indices();
    let (i0, c0) = iter.next().ok_or_else(|| format!("too short: {s}"))?;
    let Some((i1, c1)) = iter.next() else {
        return Err(format!("too short: {s}"));
    };
    let a = parse_num(&s[i0 .. i0 + c0.len_utf8()])
        .ok_or_else(|| format!("not a coordinate digit: {c0}"))?;
    let b = parse_num(&s[i1 .. i1 + c1.len_utf8()])
        .ok_or_else(|| format!("not a coordinate digit: {c1}"))?;
    Ok((a, b))
}

impl<'a> NotationResolver<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self { board }
    }

    // --- Convenience ---

    /// Parse an action string (e.g. `红车平五捉`) into an [`Action`].
    pub fn parse_action(&self, s: &str) -> Result<Action, String> {
        let an: ActionNotation = s.parse()?;
        self.resolve_action(an)
    }

    /// Format an [`Action`] as notation text.
    pub fn fmt_action(&self, action: &Action) -> String {
        self.action_notation(action).to_string()
    }

    /// Parse a reaction text against the pre-action board. The outer
    /// `Result` is a notation failure (malformed text, unresolvable
    /// pieces); the inner one distinguishes a success reaction from an
    /// `错误：` reaction.
    pub fn parse_reaction(&self, s: &str) -> Result<Result<Reaction, String>, String> {
        let rn: ReactionNotation = s.parse()?;
        Ok(self.resolve_reaction(rn))
    }

    /// Format an action outcome — success or error — as reaction text,
    /// against the pre-action board.
    pub fn fmt_reaction(&self, result: Result<Reaction, String>) -> String {
        self.reaction_notation(result).to_string()
    }

    /// Resolve a parsed action into 0-based API coordinates. An unsuffixed
    /// piece-position resolves to a move when the piece is on the board,
    /// and to a placement otherwise.
    pub fn resolve_action(&self, action: ActionNotation) -> Result<Action, String> {
        let action = match action {
            ActionNotation::Place(p) => Action::Place(self.place(p)?),
            ActionNotation::Move(pp) => self.resolve_move_or_place(pp)?,
            ActionNotation::Capture(pp) => Action::Capture(self.move_(pp.piece, pp.position)?),
            ActionNotation::Push(pp) => Action::Push(self.move_(pp.piece, pp.position)?),
            ActionNotation::Pass(p) => Action::Pass(p),
            ActionNotation::Resign(p) => Action::Resign(p),
        };
        Ok(action)
    }

    fn resolve_move_or_place(&self, pp: PiecePosition) -> Result<Action, String> {
        match pp.piece {
            PieceNotation::Name(p) => match self.board.find_unique(p) {
                Ok(from) => {
                    let to = self.resolve_position(from, p.color, pp.position)?;
                    Ok(Action::Move(Move { from, to }))
                },
                Err(false) => match pp.position {
                    Position::Absolute(col, row) => {
                        Ok(Action::Place(self.place(PlaceNotation { piece: p, to: (col, row) })?))
                    },
                    Position::Relative(_) => Err("placement requires absolute position".into()),
                },
                Err(true) => Err(format!("multiple {p} on board, identify by coordinates")),
            },
            PieceNotation::Coord(col, row) => {
                Ok(Action::Move(self.coord_move(col, row, pp.position)?))
            },
        }
    }

    fn move_(&self, piece: PieceNotation, position: Position) -> Result<Move, String> {
        match piece {
            PieceNotation::Name(p) => {
                let from = self.find_unique(p)?;
                let to = self.resolve_position(from, p.color, position)?;
                Ok(Move { from, to })
            },
            PieceNotation::Coord(col, row) => self.coord_move(col, row, position),
        }
    }

    fn coord_move(&self, col: u8, row: u8, position: Position) -> Result<Move, String> {
        let from = self.checked(col, row)?;
        let to = match position {
            Position::Absolute(c, r) => self.checked(c, r)?,
            Position::Relative(rp) => match rp {
                RelativePosition::Horizontal(c) => self.checked(c, from.1 + 1)?,
                RelativePosition::Straight(r) => self.checked(from.0 + 1, r)?,
                RelativePosition::Advance(_) | RelativePosition::Retreat(_) => {
                    return Err("进/退 not allowed with coordinate notation, \
                         use 直 for vertical moves"
                        .into());
                },
            },
        };
        self.board[from].ok_or_else(|| format!("no piece at ({},{})", from.0, from.1))?;
        Ok(Move { from, to })
    }

    /// Convert an [`Action`] into notation form, choosing name references
    /// for unique pieces and coordinates otherwise.
    pub fn action_notation(&self, action: &Action) -> ActionNotation {
        match action {
            Action::Place(p) => ActionNotation::Place(self.place_notation(*p)),
            Action::Move(m) => ActionNotation::Move(self.piece_position(m.from, m.to)),
            Action::Capture(m) => ActionNotation::Capture(self.piece_position(m.from, m.to)),
            Action::Push(m) => ActionNotation::Push(self.piece_position(m.from, m.to)),
            Action::Pass(p) => ActionNotation::Pass(*p),
            Action::Resign(p) => ActionNotation::Resign(*p),
        }
    }

    /// Resolve a parsed reaction against the pre-action board. An
    /// `错误：` reaction resolves to `Err` with its message.
    pub fn resolve_reaction(&self, reaction: ReactionNotation) -> Result<Reaction, String> {
        match reaction {
            ReactionNotation::Error(msg) => Err(msg),
            ReactionNotation::Changes { changes, game_result } => {
                let changes = changes
                    .into_iter()
                    .map(|c| self.resolve_change(c))
                    .collect::<Result<_, _>>()?;
                Ok(Reaction { changes, game_result })
            },
        }
    }

    /// Convert an action outcome — success or error — into notation form,
    /// against the pre-action board.
    pub fn reaction_notation(&self, result: Result<Reaction, String>) -> ReactionNotation {
        match result {
            Err(msg) => ReactionNotation::Error(msg),
            Ok(result) => {
                let changes = result.changes.into_iter().map(|c| self.change_notation(c)).collect();
                ReactionNotation::Changes { changes, game_result: result.game_result }
            },
        }
    }

    /// Resolve one change entry against the pre-action board. A name-based
    /// entry whose piece is not on the board resolves to a placement (the
    /// position must then be absolute).
    pub fn resolve_change(&self, change: ChangeNotation) -> Result<PieceChange, String> {
        match change {
            ChangeNotation::Place(p) => Ok(PieceChange::Place(self.place(p)?)),
            ChangeNotation::Remove(piece) => {
                let from = match piece {
                    PieceNotation::Name(p) => self.find_unique(p)?,
                    PieceNotation::Coord(col, row) => self.checked(col, row)?,
                };
                Ok(PieceChange::Remove(from.0, from.1))
            },
            ChangeNotation::Move(pp) => match pp.piece {
                PieceNotation::Name(p) => match self.board.find_unique(p) {
                    Ok(from) => {
                        let to = self.resolve_position(from, p.color, pp.position)?;
                        Ok(PieceChange::Move(Move { from, to }))
                    },
                    Err(false) => match pp.position {
                        Position::Absolute(col, row) => Ok(PieceChange::Place(
                            self.place(PlaceNotation { piece: p, to: (col, row) })?,
                        )),
                        Position::Relative(_) => Err("placement requires absolute position".into()),
                    },
                    Err(true) => Err(format!("multiple {p} on board, identify by coordinates")),
                },
                PieceNotation::Coord(col, row) => {
                    Ok(PieceChange::Move(self.coord_move(col, row, pp.position)?))
                },
            },
        }
    }

    /// Convert one piece change into notation form, against the pre-action
    /// board (piece references describe the board before the change).
    pub fn change_notation(&self, change: PieceChange) -> ChangeNotation {
        match change {
            PieceChange::Move(m) => ChangeNotation::Move(self.piece_position(m.from, m.to)),
            PieceChange::Place(place) => ChangeNotation::Place(self.place_notation(place)),
            PieceChange::Remove(x, y) => ChangeNotation::Remove(self.piece_notation((x, y))),
        }
    }

    fn place(&self, place: PlaceNotation) -> Result<Place, String> {
        Ok(Place { piece: place.piece, to: self.checked(place.to.0, place.to.1)? })
    }

    fn resolve_position(
        &self, from: (u8, u8), color: Color, pos: Position,
    ) -> Result<(u8, u8), String> {
        let result = pos
            .resolve((from.0 + 1, from.1 + 1), self.board.width(), self.board.height(), color)
            .ok_or_else(|| format!("cannot resolve position from ({},{})", from.0, from.1))?;
        self.checked(result.0, result.1)
    }

    fn place_notation(&self, place: Place) -> PlaceNotation {
        PlaceNotation { piece: place.piece, to: (place.to.0 + 1, place.to.1 + 1) }
    }

    fn piece_position(&self, from: (u8, u8), to: (u8, u8)) -> PiecePosition {
        PiecePosition { piece: self.piece_notation(from), position: self.position(from, to) }
    }

    fn piece_notation(&self, pos: (u8, u8)) -> PieceNotation {
        match self.board.get(pos) {
            Some(piece) if self.board.find_unique(piece).is_ok() => PieceNotation::Name(piece),
            _ => PieceNotation::Coord(pos.0 + 1, pos.1 + 1),
        }
    }

    fn position(&self, from: (u8, u8), to: (u8, u8)) -> Position {
        let color = self.board.get(from).map_or(Color::White, |p| p.color);
        Position::notation((from.0 + 1, from.1 + 1), color, (to.0 + 1, to.1 + 1))
    }

    fn find_unique(&self, piece: Piece) -> Result<(u8, u8), String> {
        match self.board.find_unique(piece) {
            Ok(from) => Ok((from.0, from.1)),
            Err(false) => Err(format!("piece {piece} not on board")),
            Err(true) => Err(format!("multiple {piece} on board, identify by coordinates")),
        }
    }

    /// Convert a 1-based notation coordinate to a 0-based board position,
    /// validating that it lies on the board. Rejects 零 (0) and anything
    /// beyond the board edge.
    fn checked(&self, col: u8, row: u8) -> Result<(u8, u8), String> {
        if col >= 1 && row >= 1 && self.board.in_bounds((col - 1, row - 1)) {
            Ok((col - 1, row - 1))
        } else {
            Err(format!("({col},{row}) is outside the board"))
        }
    }
}

impl Position {
    /// Resolve against 1-based coordinates; valid columns are 1..=width and
    /// valid rows 1..=height.
    fn resolve(self, from: (u8, u8), width: u8, height: u8, color: Color) -> Option<(u8, u8)> {
        match self {
            Position::Absolute(col, row) => {
                if (1 ..= width).contains(&col) && (1 ..= height).contains(&row) {
                    Some((col, row))
                } else {
                    None
                }
            },
            Position::Relative(ref rp) => rp.resolve(from, width, height, color),
        }
    }

    fn notation(from: (u8, u8), color: Color, to: (u8, u8)) -> Position {
        if let Some(position) = RelativePosition::notation(from, color, to) {
            return Position::Relative(position);
        }
        Position::Absolute(to.0, to.1)
    }
}

impl RelativePosition {
    /// Resolve against 1-based coordinates; see [`Position::resolve`].
    fn resolve(self, from: (u8, u8), width: u8, height: u8, color: Color) -> Option<(u8, u8)> {
        match self {
            RelativePosition::Horizontal(col) => {
                if (1 ..= width).contains(&col) {
                    Some((col, from.1))
                } else {
                    None
                }
            },
            RelativePosition::Straight(row) => {
                if (1 ..= height).contains(&row) {
                    Some((from.0, row))
                } else {
                    None
                }
            },
            RelativePosition::Advance(steps) => {
                let y = match color {
                    Color::Red => from.1.checked_sub(steps)?,
                    Color::Black => from.1 + steps,
                    Color::White => return None,
                };
                if (1 ..= height).contains(&y) { Some((from.0, y)) } else { None }
            },
            RelativePosition::Retreat(steps) => {
                let y = match color {
                    Color::Red => from.1 + steps,
                    Color::Black => from.1.checked_sub(steps)?,
                    Color::White => return None,
                };
                if (1 ..= height).contains(&y) { Some((from.0, y)) } else { None }
            },
        }
    }

    fn notation(from: (u8, u8), color: Color, to: (u8, u8)) -> Option<RelativePosition> {
        if from.1 == to.1 {
            return Some(RelativePosition::Horizontal(to.0));
        }
        if from.0 != to.0 {
            return None;
        }
        let position = match color {
            Color::Red => {
                if to.1 < from.1 {
                    RelativePosition::Advance(from.1 - to.1)
                } else {
                    RelativePosition::Retreat(to.1 - from.1)
                }
            },
            Color::Black => {
                if to.1 < from.1 {
                    RelativePosition::Retreat(from.1 - to.1)
                } else {
                    RelativePosition::Advance(to.1 - from.1)
                }
            },
            Color::White => RelativePosition::Straight(to.1),
        };
        Some(position)
    }
}
