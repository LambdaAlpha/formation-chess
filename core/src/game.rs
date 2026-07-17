use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;

use crate::ability::Ability;
use crate::action::Action;
use crate::action::GameResult;
use crate::action::PieceChange;
use crate::action::Place;
use crate::action::Reaction;
use crate::board::Board;
use crate::board::parse_board_from_lines;
use crate::piece::Color;
use crate::piece::Piece;
pub use crate::piece::Player;

/// A running game: board, pools, player to move, and result. Constructed
/// from a validated [`GameConfig`]; mutated only through [`Game::action`].
#[derive(Clone)]
pub struct Game {
    player: Player,
    board: Board,
    red_pool: Vec<Piece>,
    black_pool: Vec<Piece>,
    white: Piece,
    white_pool: u8,
    result: GameResult,
}

/// An unvalidated game snapshot: the same fields as [`Game`], freely
/// constructible and parseable from the text protocol. [`Game::new`]
/// validates it.
#[derive(Clone)]
pub struct GameConfig {
    pub player: Player,
    pub board: Board,
    pub red_pool: Vec<Piece>,
    pub black_pool: Vec<Piece>,
    /// The piece placed by CONTROL_WHITE placements.
    pub white: Piece,
    /// Number of white pieces available for placement.
    pub white_pool: u8,
    pub result: GameResult,
}

/// The standard initial setup: an empty 9×10 board, both 16-piece armies
/// in their pools, no white pieces, and Red to move.
impl Default for GameConfig {
    fn default() -> Self {
        let player = Player::Red;
        let board = Board::new(9, 10);
        let red_pool = Piece::RED_PLAYER_PIECES.to_vec();
        let black_pool = Piece::BLACK_PLAYER_PIECES.to_vec();
        let white = Piece::WHITE;
        let white_pool = 0;
        let result = GameResult::Unfinished;
        Self { player, board, red_pool, black_pool, white, white_pool, result }
    }
}

impl Game {
    /// Validate `config` and start a game from it. Rejects snapshots that
    /// break the rules: wrong pool colors, more than one vital piece per
    /// side, placement-phase pieces outside their half, pools that cannot
    /// alternate, or a declared-unfinished position that is already
    /// decided.
    pub fn new(config: GameConfig) -> Result<Self, String> {
        Self::validate_config(&config)?;
        Ok(Self {
            player: config.player,
            board: config.board,
            red_pool: config.red_pool,
            black_pool: config.black_pool,
            white: config.white,
            white_pool: config.white_pool,
            result: config.result,
        })
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// The player to move.
    pub fn player(&self) -> Player {
        self.player
    }

    /// Red pieces not yet placed on the board.
    pub fn red_pool(&self) -> &[Piece] {
        &self.red_pool
    }

    /// Black pieces not yet placed on the board.
    pub fn black_pool(&self) -> &[Piece] {
        &self.black_pool
    }

    /// Number of white pieces available for placement.
    pub fn white_pool(&self) -> u8 {
        self.white_pool
    }

    pub fn result(&self) -> GameResult {
        self.result
    }

    /// True while either player still has pieces to place. During this
    /// phase only placement actions are accepted.
    pub fn is_placement_phase(&self) -> bool {
        !self.red_pool.is_empty() || !self.black_pool.is_empty()
    }

    fn validate_config(config: &GameConfig) -> Result<(), String> {
        if config.white.color != Color::White {
            return Err(format!("white piece is {}", config.white));
        }
        for piece in &config.red_pool {
            if piece.color != Color::Red {
                return Err(format!("red pool contains {piece}"));
            }
        }
        for piece in &config.black_pool {
            if piece.color != Color::Black {
                return Err(format!("black pool contains {piece}"));
            }
        }
        Self::validate_vital(config, Player::Red)?;
        Self::validate_vital(config, Player::Black)?;
        if !config.red_pool.is_empty() || !config.black_pool.is_empty() {
            Self::validate_halves(config)?;
            Self::validate_alternation(config)?;
        }
        if config.result == GameResult::Unfinished {
            let computed =
                Self::compute_result(&config.board, &config.red_pool, &config.black_pool);
            if computed != GameResult::Unfinished {
                return Err(format!(
                    "declared result is Unfinished, but the position is already decided: \
                     {computed}"
                ));
            }
        }
        Ok(())
    }

    fn validate_vital(config: &GameConfig, player: Player) -> Result<(), String> {
        let pool = match player {
            Player::Red => &config.red_pool,
            Player::Black => &config.black_pool,
        };
        let pool_count = pool.iter().filter(|p| p.ability.has_ability(Ability::VITAL)).count();
        let count = pool_count + config.board.vital_count(player.color());
        if count > 1 {
            return Err(format!("{player} must have at most one vital piece, found {count}"));
        }
        Ok(())
    }

    fn validate_halves(config: &GameConfig) -> Result<(), String> {
        let height = config.board.height();
        let half = height / 2;
        let midpoint = height.div_ceil(2);
        for y in 0 .. midpoint {
            for x in 0 .. config.board.width() {
                let Some(piece) = config.board[(x, y)] else { continue };
                if piece.color == Color::Red {
                    return Err(format!(
                        "red piece {piece} at ({x},{y}) must be in the bottom half \
                         during placement"
                    ));
                }
            }
        }
        for y in half .. height {
            for x in 0 .. config.board.width() {
                let Some(piece) = config.board[(x, y)] else { continue };
                if piece.color == Color::Black {
                    return Err(format!(
                        "black piece {piece} at ({x},{y}) must be in the top half \
                         during placement"
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_alternation(config: &GameConfig) -> Result<(), String> {
        let (current, opponent) = match config.player {
            Player::Red => (config.red_pool.len(), config.black_pool.len()),
            Player::Black => (config.black_pool.len(), config.red_pool.len()),
        };
        // With strict alternation the player to move either has as many
        // pieces left to place as the opponent (about to start a round) or
        // exactly one more (the opponent already placed this round); the
        // player to move can never have fewer.
        if current != opponent && current != opponent + 1 {
            return Err(format!(
                "placement pools cannot alternate: player {} to move has {current} pieces \
                 to place, opponent has {opponent}",
                config.player
            ));
        }
        Ok(())
    }

    fn compute_result(board: &Board, red_pool: &[Piece], black_pool: &[Piece]) -> GameResult {
        let red = board.find_vital(Color::Red);
        let black = board.find_vital(Color::Black);

        let pool_has_vital =
            |pool: &[Piece]| pool.iter().any(|p| p.ability.has_ability(Ability::VITAL));
        let red_alive = red.is_some() || pool_has_vital(red_pool);
        let black_alive = black.is_some() || pool_has_vital(black_pool);
        match (red_alive, black_alive) {
            // Both vital pieces perished in the same action (e.g. mutual
            // destruction): neither side outlives the other, so it is a draw.
            (false, false) => return GameResult::Draw,
            (false, true) => return GameResult::BlackWin,
            (true, false) => return GameResult::RedWin,
            (true, true) => {},
        }

        if let (Some(((rx, ry), rp)), Some(((bx, by), bp))) = (red, black) {
            let (dx, dy) = (rx as i8 - bx as i8, ry as i8 - by as i8);
            if bp.formation.contains(dx, dy) && rp.formation.contains(-dx, -dy) {
                return GameResult::Draw;
            }
        }

        GameResult::Unfinished
    }

    /// Execute an action for the player to move. On success the turn
    /// passes to the opponent and the result is recomputed from the board
    /// (a resign keeps the resigner as the player to move). On error, self
    /// is unchanged. Error messages must stay single-line: the notation
    /// protocol renders them as one `错误：` line.
    pub fn action(&mut self, action: Action) -> Result<Reaction, String> {
        if self.result != GameResult::Unfinished {
            return Err(format!("game is already decided: {}", self.result));
        }
        let changes = match action {
            Action::Place(place) => self.place(place)?,
            Action::Move(move_) => {
                self.check_move_phase(move_.from)?;
                if self.board.in_bounds(move_.to) && self.board[move_.to].is_some() {
                    return Err(
                        "move to an occupied destination requires a push/capture suffix".into()
                    );
                }
                let outcome = self.board.move_to(move_)?;
                self.white_pool += outcome.captured;
                outcome.changes
            },
            Action::Capture(move_) => {
                self.check_move_phase(move_.from)?;
                let outcome = self.board.move_capture(move_)?;
                self.white_pool += outcome.captured;
                outcome.changes
            },
            Action::Push(move_) => {
                self.check_move_phase(move_.from)?;
                let outcome = self.board.move_push(move_)?;
                self.white_pool += outcome.captured;
                outcome.changes
            },
            Action::Pass(player) => self.pass(player)?,
            Action::Resign(player) => self.resign(player)?,
        };
        // A resign has already decided the game; keep the resigner as the
        // player to move and skip the board check.
        if self.result == GameResult::Unfinished {
            self.switch_player();
            self.result = Self::compute_result(&self.board, &self.red_pool, &self.black_pool);
        }
        Ok(Reaction { changes, game_result: self.result })
    }

    fn place(&mut self, place: Place) -> Result<Vec<PieceChange>, String> {
        if place.piece.color == Color::White {
            return self.place_white(place);
        }
        if place.piece.color != self.player.color() {
            return Err(format!(
                "player {} cannot place piece of color {}",
                self.player, place.piece.color
            ));
        }
        let pool = match place.piece.color {
            Color::Red => &self.red_pool,
            Color::Black => &self.black_pool,
            Color::White => unreachable!(),
        };
        let position = pool
            .iter()
            .position(|piece| *piece == place.piece)
            .ok_or_else(|| format!("piece {} not in pool", place.piece))?;
        // Place the canonical piece from the pool: `Piece` equality only
        // compares name and color, so the caller-supplied piece must not be
        // trusted for abilities or formation.
        let piece = pool[position];
        let changes = self.board.place(piece, place.to)?;
        match place.piece.color {
            Color::Red => self.red_pool.swap_remove(position),
            Color::Black => self.black_pool.swap_remove(position),
            Color::White => unreachable!(),
        };
        Ok(changes)
    }

    fn place_white(&mut self, place: Place) -> Result<Vec<PieceChange>, String> {
        if self.is_placement_phase() {
            return Err("cannot place white pieces during the placement phase".into());
        }
        if place.piece != self.white {
            return Err(format!("white piece {} does not match this game's", place.piece));
        }
        if self.white_pool == 0 {
            return Err("no white pieces available to place".into());
        }
        let changes = self.board.place_white(self.white, place.to, self.player)?;
        self.white_pool -= 1;
        Ok(changes)
    }

    fn check_move_phase(&self, from: (u8, u8)) -> Result<(), String> {
        if self.is_placement_phase() {
            return Err("cannot move pieces during the placement phase".into());
        }
        let piece = self
            .board
            .effective(from)
            .ok_or_else(|| format!("no piece at ({},{})", from.0, from.1))?;
        if !piece.can_controlled_by(self.player) {
            return Err(format!(
                "player {} cannot control piece {} at ({},{})",
                self.player, piece, from.0, from.1
            ));
        }
        Ok(())
    }

    fn pass(&mut self, player: Player) -> Result<Vec<PieceChange>, String> {
        if self.is_placement_phase() {
            return Err("cannot pass during the placement phase".into());
        }
        self.check_player(player)?;
        Ok(Vec::new())
    }

    fn resign(&mut self, player: Player) -> Result<Vec<PieceChange>, String> {
        if self.is_placement_phase() {
            return Err("cannot resign during the placement phase".into());
        }
        self.check_player(player)?;
        self.result = match self.player {
            Player::Red => GameResult::BlackWin,
            Player::Black => GameResult::RedWin,
        };
        Ok(Vec::new())
    }

    fn check_player(&self, player: Player) -> Result<(), String> {
        if player != self.player {
            return Err(format!(
                "action declares player {player}, but player {} is to move",
                self.player
            ));
        }
        Ok(())
    }

    fn switch_player(&mut self) {
        self.player = match self.player {
            Player::Red => Player::Black,
            Player::Black => Player::Red,
        };
    }
}

struct Snapshot<'a> {
    player: Player,
    red: &'a [Piece],
    black: &'a [Piece],
    white_count: u8,
    result: GameResult,
    board: &'a Board,
}

impl Display for Snapshot<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "行棋方：{}", self.player)?;
        fmt_pool(Player::Red, self.red, f)?;
        fmt_pool(Player::Black, self.black, f)?;
        writeln!(f, "白方：{}", self.white_count)?;
        writeln!(f, "胜负：{}", self.result)?;
        writeln!(f, "棋盘：")?;
        write!(f, "{}", self.board)
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Snapshot {
            player: self.player(),
            red: self.red_pool(),
            black: self.black_pool(),
            white_count: self.white_pool(),
            result: self.result(),
            board: self.board(),
        }
        .fmt(f)
    }
}

impl Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for GameConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Snapshot {
            player: self.player,
            red: &self.red_pool,
            black: &self.black_pool,
            white_count: self.white_pool,
            result: self.result,
            board: &self.board,
        }
        .fmt(f)
    }
}

impl Debug for GameConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

fn fmt_pool(player: Player, pool: &[Piece], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{player}方：[")?;
    for (i, p) in pool.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }
        write!(f, "{}", p.name)?;
    }
    writeln!(f, "]")
}

impl FromStr for GameConfig {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        parse_config(s)
    }
}

impl FromStr for Game {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let config: GameConfig = s.parse()?;
        Self::new(config)
    }
}

fn parse_pool(s: &str, color: Color) -> Result<Vec<Piece>, String> {
    let inner = s
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| format!("pool must be bracketed: {s}"))?;
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split_whitespace()
        .map(|s| {
            let mut chars = s.chars();
            let name = chars.next().ok_or_else(|| "empty piece name in pool".to_string())?;
            if chars.next().is_some() {
                return Err(format!("piece name in pool must be a single character: {s}"));
            }
            Piece::lookup(name, color).ok_or_else(|| format!("unknown piece in pool: {s}"))
        })
        .collect()
}

fn parse_config(s: &str) -> Result<GameConfig, String> {
    let mut lines = s.lines();
    let player_line = lines.next().ok_or("missing player line")?;
    let player: Player = player_line
        .strip_prefix("行棋方：")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("invalid player: {player_line}"))?;

    let red_line = lines.next().ok_or("missing red pool")?;
    let red = parse_pool(red_line.strip_prefix("红方：").ok_or("invalid red pool")?, Color::Red)?;

    let black_line = lines.next().ok_or("missing black pool")?;
    let black =
        parse_pool(black_line.strip_prefix("黑方：").ok_or("invalid black pool")?, Color::Black)?;

    let white_line = lines.next().ok_or("missing white count")?;
    let white_count: u8 = white_line
        .strip_prefix("白方：")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("invalid white count: {white_line}"))?;

    let result_line = lines.next().ok_or("missing result line")?;
    let result: GameResult = result_line
        .strip_prefix("胜负：")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| format!("invalid result: {result_line}"))?;

    let board_line = lines.next().ok_or("missing board line")?;
    if board_line.trim() != "棋盘：" {
        return Err(format!("invalid board line: {board_line}"));
    }
    let board = parse_board_from_lines(&mut lines).map_err(|e| format!("board: {e}"))?;

    Ok(GameConfig {
        player,
        board,
        red_pool: red,
        black_pool: black,
        white: Piece::WHITE,
        white_pool: white_count,
        result,
    })
}
