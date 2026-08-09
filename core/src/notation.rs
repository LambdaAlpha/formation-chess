use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use crate::action::Action;
use crate::action::GameResult;
use crate::action::Move;
use crate::action::Place;
use crate::action::PoolChange;
use crate::action::PositionChange;
use crate::action::PositionChanges;
use crate::action::Reaction;
use crate::board::Board;
use crate::chinese_num::fmt_num;
use crate::chinese_num::parse_num;
use crate::game::Game;
use crate::game::Phase;
use crate::piece::Piece;
use crate::piece::PieceId;
use crate::piece::Player;

/// Resolves between 1‑based notation types and 0‑based API types, using a
/// game for board and pool lookups. When formatting or parsing
/// the changes of an already-executed action, the resolver must hold the
/// game as it stood **before** that action.
pub struct NotationResolver<'a> {
    game: &'a Game,
}

/// An action as written in notation, e.g. `红车平五推` or `黑将认负`.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionNotation {
    /// Piece + position without a suffix: a move, or a placement when the
    /// piece is not on the board.
    Phase(PiecePosition),
    /// Piece + position + `捉`.
    Capture(PiecePosition),
    /// Piece + position + `推`.
    Push(PiecePosition),
    /// Piece + position + `拉`.
    Pull(PiecePosition),
    /// Piece + position + `和`.
    Draw(PiecePosition),
    /// Piece ID + `认负`.
    Resign(PieceId),
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
    /// Piece + `失`: the piece left the board and nothing replaced it.
    Remove(PieceNotation),
    /// Piece + position: the piece now stands there. Doubles as a
    /// placement when the piece is not on the board.
    Phase(PiecePosition),
}

/// A placement in notation: a piece identity and a 1-based destination.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceNotation {
    pub piece: PieceId,
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
    Name(PieceId),
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
    /// `进` + steps: forward — up for Red, down for Black.
    Advance(u8),
    /// `退` + steps: backward — down for Red, up for Black.
    Retreat(u8),
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let na = match self {
            Action::Place(p) => ActionNotation::Phase(place_notation(*p).into()),
            Action::Move(m) => ActionNotation::Phase(piece_position(*m)),
            Action::Capture(m) => ActionNotation::Capture(piece_position(*m)),
            Action::Push(m) => ActionNotation::Push(piece_position(*m)),
            Action::Pull(m) => ActionNotation::Pull(piece_position(*m)),
            Action::Draw(m) => ActionNotation::Draw(piece_position(*m)),
            Action::Resign(_, _) => {
                ActionNotation::Resign(PieceId { name: '黑', player: Player::Red })
            },
        };
        write!(f, "{na}")
    }
}

impl Display for Reaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let changes = self.changes.iter().map(|change| change_notation(*change)).collect();
        let rn = ReactionNotation::Changes { changes, game_result: self.game_result };
        write!(f, "{rn}")
    }
}

impl Display for PositionChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", change_notation(*self))
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", place_notation(*self))
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

fn vital_id(player: Player) -> PieceId {
    PieceId { name: Piece::GENERAL_NAME, player }
}

fn place_notation(p: Place) -> PlaceNotation {
    PlaceNotation { piece: p.piece, to: (p.to.0 + 1, p.to.1 + 1) }
}

fn change_notation(change: PositionChange) -> ChangeNotation {
    if let Some(piece) = change.new {
        ChangeNotation::Place(PlaceNotation {
            piece: piece.id(),
            to: (change.at.0 + 1, change.at.1 + 1),
        })
    } else {
        ChangeNotation::Remove(PieceNotation::Coord(change.at.0 + 1, change.at.1 + 1))
    }
}

impl From<PlaceNotation> for PiecePosition {
    fn from(notation: PlaceNotation) -> Self {
        Self {
            piece: PieceNotation::Name(notation.piece),
            position: Position::Absolute(notation.to.0, notation.to.1),
        }
    }
}

impl Display for ActionNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionNotation::Phase(pp) => write!(f, "{pp}"),
            ActionNotation::Capture(pp) => write!(f, "{pp}捉"),
            ActionNotation::Push(pp) => write!(f, "{pp}推"),
            ActionNotation::Pull(pp) => write!(f, "{pp}拉"),
            ActionNotation::Draw(pp) => write!(f, "{pp}和"),
            ActionNotation::Resign(piece) => write!(f, "{piece}认负"),
        }
    }
}

impl FromStr for ActionNotation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("empty action".into());
        }
        if let Some(piece) = s.strip_suffix("认负") {
            return Ok(Self::Resign(piece.parse()?));
        }
        if let Some(body) = s.strip_suffix('捉') {
            return Ok(Self::Capture(body.parse()?));
        }
        if let Some(body) = s.strip_suffix('推') {
            return Ok(Self::Push(body.parse()?));
        }
        if let Some(body) = s.strip_suffix('拉') {
            return Ok(Self::Pull(body.parse()?));
        }
        if let Some(body) = s.strip_suffix('和') {
            return Ok(Self::Draw(body.parse()?));
        }
        Ok(Self::Phase(s.parse()?))
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
        let Some((changes_part, result_part)) = s.split_once('\n') else {
            return Err("missing result line".to_string());
        };
        let Some(changes_str) = changes_part.strip_prefix("变化：[") else {
            return Err(format!("invalid changes prefix: {changes_part}"));
        };
        let Some(changes_str) = changes_str.strip_suffix(']') else {
            return Err(format!("invalid changes prefix: {changes_part}"));
        };

        let changes = if changes_str.is_empty() {
            Vec::new()
        } else {
            changes_str
                .split_whitespace()
                .map(str::parse::<ChangeNotation>)
                .collect::<Result<Vec<_>, _>>()?
        };

        let Some(result_str) = result_part.strip_prefix("胜负：") else {
            return Err(format!("invalid result line: {result_part}"));
        };
        let game_result = result_str.trim().parse()?;

        Ok(Self::Changes { changes, game_result })
    }
}

impl Display for ChangeNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeNotation::Place(p) => write!(f, "{p}占"),
            ChangeNotation::Remove(piece) => write!(f, "{piece}失"),
            ChangeNotation::Phase(pp) => write!(f, "{pp}"),
        }
    }
}

impl FromStr for ChangeNotation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("empty change entry".into());
        }
        if let Some(body) = s.strip_suffix('失') {
            return Ok(Self::Remove(body.parse()?));
        }
        if let Some(body) = s.strip_suffix('占') {
            return Ok(Self::Place(body.parse()?));
        }
        Ok(Self::Phase(s.parse()?))
    }
}

impl Display for PlaceNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.piece, fmt_num(self.to.0), fmt_num(self.to.1))
    }
}

impl FromStr for PlaceNotation {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (pn, rest) = split_at_2(s)?;
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
        if s.starts_with('红') || s.starts_with('黑') {
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
        let Some((_, c0)) = iter.next() else {
            return Err(format!("invalid relative position: {s}"));
        };
        let Some((i1, c1)) = iter.next() else {
            return Err(format!("invalid relative position: {s}"));
        };
        if iter.next().is_some() {
            return Err(format!("invalid relative position: {s}"));
        }
        let Some(n) = parse_num(&s[i1 .. i1 + c1.len_utf8()]) else {
            return Err(format!("not a coordinate digit: {c1}"));
        };
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
    let Some((i0, c0)) = iter.next() else {
        return Err(format!("too short: {s}"));
    };
    let Some((i1, c1)) = iter.next() else {
        return Err(format!("too short: {s}"));
    };
    let Some(a) = parse_num(&s[i0 .. i0 + c0.len_utf8()]) else {
        return Err(format!("not a coordinate digit: {c0}"));
    };
    let Some(b) = parse_num(&s[i1 .. i1 + c1.len_utf8()]) else {
        return Err(format!("not a coordinate digit: {c1}"));
    };
    Ok((a, b))
}

impl<'a> NotationResolver<'a> {
    /// Create a resolver against a game snapshot. Reaction formatting and
    /// parsing require the game as it stood before the action.
    pub fn new(game: &'a Game) -> Self {
        Self { game }
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

    /// Parse a reaction text against the pre-action game. The outer
    /// `Result` is a notation failure (malformed text, unresolvable
    /// pieces); the inner one distinguishes a success reaction from an
    /// `错误：` reaction.
    pub fn parse_reaction(&self, s: &str) -> Result<Result<Reaction, String>, String> {
        let rn: ReactionNotation = s.parse()?;
        Ok(self.resolve_reaction(rn))
    }

    /// Format an action outcome — success or error — as reaction text,
    /// against the pre-action game.
    pub fn fmt_reaction(&self, result: Result<Reaction, String>) -> String {
        self.reaction_notation(result).to_string()
    }

    /// Resolve a parsed action into 0-based API coordinates. An unsuffixed
    /// piece-position uses the resolver's `place_phase` to decide:
    /// placement phase → Place, movement phase → Move.
    pub fn resolve_action(&self, action: ActionNotation) -> Result<Action, String> {
        let action = match action {
            ActionNotation::Phase(pp) => self.resolve_move_or_place(pp)?,
            ActionNotation::Capture(pp) => Action::Capture(self.move_(pp)?),
            ActionNotation::Push(pp) => Action::Push(self.move_(pp)?),
            ActionNotation::Pull(pp) => Action::Pull(self.move_(pp)?),
            ActionNotation::Draw(pp) => Action::Draw(self.move_(pp)?),
            ActionNotation::Resign(piece) => self.resolve_resign(piece)?,
        };
        Ok(action)
    }

    fn resolve_move_or_place(&self, pp: PiecePosition) -> Result<Action, String> {
        let action = if self.game.phase() == Phase::Place
            && let PieceNotation::Name(piece) = pp.piece
            && let Position::Absolute(x, y) = pp.position
        {
            Action::Place(Place { piece, to: self.checked(x, y)? })
        } else {
            Action::Move(self.move_(pp)?)
        };
        Ok(action)
    }

    fn resolve_resign(&self, piece: PieceId) -> Result<Action, String> {
        if piece.name != Piece::GENERAL_NAME {
            return Err(format!("{} is not a vital piece", piece));
        }
        if self.game.phase() == Phase::Place {
            if piece.player != self.game.player() {
                return Err(format!("认负 requires {}'s vital piece", self.game.player()));
            }
            return Ok(Action::Resign(0, 0));
        }
        let at = self.find_unique(piece)?;
        Ok(Action::Resign(at.0, at.1))
    }

    fn move_(&self, pp: PiecePosition) -> Result<Move, String> {
        match pp.piece {
            PieceNotation::Name(p) => {
                let from = self.find_unique(p)?;
                let to = self.resolve_position(from, p.player, pp.position)?;
                Ok(Move { from, to })
            },
            PieceNotation::Coord(col, row) => self.coord_move(col, row, pp.position),
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
        if self.game.board()[from].is_none() {
            return Err(format!("no piece at ({},{})", from.0, from.1));
        }
        Ok(Move { from, to })
    }

    /// Convert an [`Action`] into notation form, choosing name references
    /// for unique pieces and coordinates otherwise.
    pub fn action_notation(&self, action: &Action) -> ActionNotation {
        match action {
            Action::Place(p) => ActionNotation::Phase(self.place_notation(*p).into()),
            Action::Move(m) => ActionNotation::Phase(self.piece_position(m.from, m.to)),
            Action::Capture(m) => ActionNotation::Capture(self.piece_position(m.from, m.to)),
            Action::Push(m) => ActionNotation::Push(self.piece_position(m.from, m.to)),
            Action::Pull(m) => ActionNotation::Pull(self.piece_position(m.from, m.to)),
            Action::Draw(m) => ActionNotation::Draw(self.piece_position(m.from, m.to)),
            Action::Resign(x, y) => ActionNotation::Resign(self.resign_id((*x, *y))),
        }
    }

    /// Resolve a parsed reaction against the pre-action game. An
    /// `错误：` reaction resolves to `Err` with its message.
    pub fn resolve_reaction(&self, reaction: ReactionNotation) -> Result<Reaction, String> {
        match reaction {
            ReactionNotation::Error(msg) => Err(msg),
            ReactionNotation::Changes { changes, game_result } => {
                let mut position_changes = Vec::new();
                for c in changes {
                    position_changes.extend(self.resolve_change(c)?);
                }
                let mut resolved = Board::normalize_changes(&position_changes);
                let pool_change = self.resolve_pool_change(&mut resolved)?;
                let changes = PositionChanges::try_from_slice(&resolved)?;
                Ok(Reaction { changes, pool_change, game_result })
            },
        }
    }

    /// Convert an action outcome — success or error — into notation form,
    /// against the pre-action game.
    pub fn reaction_notation(&self, result: Result<Reaction, String>) -> ReactionNotation {
        match result {
            Err(msg) => ReactionNotation::Error(msg),
            Ok(result) => {
                let changes = self.changes_notation(result.changes.as_slice());
                ReactionNotation::Changes { changes, game_result: result.game_result }
            },
        }
    }

    /// Resolve one change entry against the pre-action board. A name-based
    /// entry whose piece is not on the board resolves to a placement (the
    /// position must then be absolute).
    pub fn resolve_change(&self, change: ChangeNotation) -> Result<Vec<PositionChange>, String> {
        match change {
            ChangeNotation::Place(p) => self.resolve_place_change(p),
            ChangeNotation::Remove(piece) => {
                let from = match piece {
                    PieceNotation::Name(p) => self.find_unique(p)?,
                    PieceNotation::Coord(col, row) => self.checked(col, row)?,
                };
                Ok(vec![PositionChange { at: from, old: self.game.board().get(from), new: None }])
            },
            ChangeNotation::Phase(pp) => {
                if self.game.phase() == Phase::Place
                    && let PieceNotation::Name(piece) = pp.piece
                    && let Position::Absolute(col, row) = pp.position
                {
                    self.resolve_place_change(PlaceNotation { piece, to: (col, row) })
                } else {
                    self.resolve_change_move(pp)
                }
            },
        }
    }

    /// Resolve a `Phase` entry as a move. Returns an error when the piece
    /// is not on the pre-action board.
    fn resolve_change_move(&self, pp: PiecePosition) -> Result<Vec<PositionChange>, String> {
        let move_ = self.move_(pp)?;
        let Some(piece) = self.game.board().get(move_.from) else {
            return Err(format!("no piece at ({},{})", move_.from.0, move_.from.1));
        };
        Ok(vec![PositionChange { at: move_.from, old: Some(piece), new: None }, PositionChange {
            at: move_.to,
            old: self.game.board().get(move_.to),
            new: Some(piece),
        }])
    }

    /// Convert reversible position changes into notation form, against the
    /// pre-action game. Matching departures and arrivals become moves;
    /// unmatched arrivals become placements, and unmatched departures become
    /// removals unless an arrival at the same point already implies capture.
    pub fn changes_notation(&self, changes: &[PositionChange]) -> Vec<ChangeNotation> {
        let mut departures = Vec::new();
        let mut arrivals = Vec::new();
        for change in changes {
            if change.old == change.new {
                continue;
            }
            if let Some(piece) = change.old {
                departures.push((change.at, piece, false));
            }
            if let Some(piece) = change.new {
                arrivals.push((change.at, piece));
            }
        }

        let mut result = Vec::new();
        for (to, piece) in arrivals {
            let mut from = None;
            for departure in &mut departures {
                if !departure.2 && departure.1 == piece {
                    departure.2 = true;
                    from = Some(departure.0);
                    break;
                }
            }
            if let Some(from) = from {
                result.push(ChangeNotation::Phase(self.piece_position(from, to)));
            } else {
                result.push(ChangeNotation::Place(PlaceNotation {
                    piece: piece.id(),
                    to: (to.0 + 1, to.1 + 1),
                }));
            }
        }

        for (at, _, matched) in departures {
            if matched {
                continue;
            }
            let mut replaced = false;
            for change in changes {
                if change.at == at && change.new.is_some() {
                    replaced = true;
                    break;
                }
            }
            if !replaced {
                result.push(ChangeNotation::Remove(self.piece_notation(at)));
            }
        }

        result
    }

    fn resolve_place_change(&self, notation: PlaceNotation) -> Result<Vec<PositionChange>, String> {
        let place = self.place(notation)?;
        let piece = self.resolve_external_piece(place.piece)?;
        Ok(vec![PositionChange {
            at: place.to,
            old: self.game.board().get(place.to),
            new: Some(piece),
        }])
    }

    fn resolve_external_piece(&self, piece: PieceId) -> Result<Piece, String> {
        if self.game.phase() == Phase::Place {
            let (piece, _) = self.game.find_in_pool(piece)?;
            return Ok(piece);
        }
        let Some(piece) = Piece::lookup(piece.name, piece.player) else {
            return Err(format!("unknown piece: {piece}"));
        };
        Ok(piece)
    }

    fn resolve_pool_change(&self, changes: &mut [PositionChange]) -> Result<PoolChange, String> {
        if self.game.phase() != Phase::Place || changes.is_empty() {
            return Ok(PoolChange::Unchanged);
        }
        if changes.len() != 1 {
            return Err("placement reaction must contain exactly one position change".into());
        }
        let change = &mut changes[0];
        if change.old.is_some() {
            return Err("placement reaction destination must be empty".into());
        }
        let Some(piece_id) = change.new.map(|piece| piece.id()) else {
            return Err("placement reaction must place a piece".into());
        };
        let (piece, index) = self.game.find_in_pool(piece_id)?;
        change.new = Some(piece);
        Ok(PoolChange::Removed { index, piece })
    }

    fn place(&self, place: PlaceNotation) -> Result<Place, String> {
        Ok(Place { piece: place.piece, to: self.checked(place.to.0, place.to.1)? })
    }

    fn resolve_position(
        &self, from: (u8, u8), player: Player, pos: Position,
    ) -> Result<(u8, u8), String> {
        let Some(result) = pos.resolve(
            (from.0 + 1, from.1 + 1),
            self.game.board().width(),
            self.game.board().height(),
            player,
        ) else {
            return Err(format!("cannot resolve position from ({},{})", from.0, from.1));
        };
        self.checked(result.0, result.1)
    }

    fn place_notation(&self, place: Place) -> PlaceNotation {
        PlaceNotation { piece: place.piece, to: (place.to.0 + 1, place.to.1 + 1) }
    }

    fn piece_position(&self, from: (u8, u8), to: (u8, u8)) -> PiecePosition {
        PiecePosition { piece: self.piece_notation(from), position: self.position(from, to) }
    }

    fn resign_id(&self, pos: (u8, u8)) -> PieceId {
        if self.game.phase() == Phase::Place {
            return vital_id(self.game.player());
        }
        let Some(piece) = self.game.board().get(pos) else {
            return vital_id(self.game.player());
        };
        piece.id()
    }

    fn piece_notation(&self, pos: (u8, u8)) -> PieceNotation {
        let Some(piece) = self.game.board().get(pos) else {
            return PieceNotation::Coord(pos.0 + 1, pos.1 + 1);
        };
        let piece = piece.id();
        if self.game.board().find_unique(piece).is_ok() {
            PieceNotation::Name(piece)
        } else {
            PieceNotation::Coord(pos.0 + 1, pos.1 + 1)
        }
    }

    fn position(&self, from: (u8, u8), to: (u8, u8)) -> Position {
        let Some(piece) = self.game.board().get(from) else {
            return Position::Absolute(to.0 + 1, to.1 + 1);
        };
        Position::notation((from.0 + 1, from.1 + 1), piece.player, (to.0 + 1, to.1 + 1))
    }

    fn find_unique(&self, piece: PieceId) -> Result<(u8, u8), String> {
        match self.game.board().find_unique(piece) {
            Ok(from) => Ok((from.0, from.1)),
            Err(false) => Err(format!("piece {piece} not on board")),
            Err(true) => Err(format!("multiple {piece} on board, identify by coordinates")),
        }
    }

    /// Convert a 1-based notation coordinate to a 0-based board position,
    /// validating that it lies on the board. Rejects 零 (0) and anything
    /// beyond the board edge.
    fn checked(&self, col: u8, row: u8) -> Result<(u8, u8), String> {
        if col >= 1 && row >= 1 && self.game.board().in_bounds((col - 1, row - 1)) {
            Ok((col - 1, row - 1))
        } else {
            Err(format!("({col},{row}) is outside the board"))
        }
    }
}

impl Position {
    /// Resolve against 1-based coordinates; valid columns are 1..=width and
    /// valid rows 1..=height.
    fn resolve(self, from: (u8, u8), width: u8, height: u8, player: Player) -> Option<(u8, u8)> {
        match self {
            Position::Absolute(col, row) => {
                if (1 ..= width).contains(&col) && (1 ..= height).contains(&row) {
                    Some((col, row))
                } else {
                    None
                }
            },
            Position::Relative(ref rp) => rp.resolve(from, width, height, player),
        }
    }

    fn notation(from: (u8, u8), player: Player, to: (u8, u8)) -> Position {
        if let Some(position) = RelativePosition::notation(from, player, to) {
            return Position::Relative(position);
        }
        Position::Absolute(to.0, to.1)
    }
}

impl RelativePosition {
    /// Resolve against 1-based coordinates; see [`Position::resolve`].
    fn resolve(self, from: (u8, u8), width: u8, height: u8, player: Player) -> Option<(u8, u8)> {
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
                let y = match player {
                    Player::Red => from.1.checked_sub(steps)?,
                    Player::Black => from.1 + steps,
                };
                if (1 ..= height).contains(&y) { Some((from.0, y)) } else { None }
            },
            RelativePosition::Retreat(steps) => {
                let y = match player {
                    Player::Red => from.1 + steps,
                    Player::Black => from.1.checked_sub(steps)?,
                };
                if (1 ..= height).contains(&y) { Some((from.0, y)) } else { None }
            },
        }
    }

    fn notation(from: (u8, u8), player: Player, to: (u8, u8)) -> Option<RelativePosition> {
        if from.1 == to.1 {
            return Some(RelativePosition::Horizontal(to.0));
        }
        if from.0 != to.0 {
            return None;
        }
        let position = match player {
            Player::Red => {
                if to.1 < from.1 {
                    RelativePosition::Advance(from.1 - to.1)
                } else {
                    RelativePosition::Retreat(to.1 - from.1)
                }
            },
            Player::Black => {
                if to.1 < from.1 {
                    RelativePosition::Retreat(from.1 - to.1)
                } else {
                    RelativePosition::Advance(to.1 - from.1)
                }
            },
        };
        Some(position)
    }
}
