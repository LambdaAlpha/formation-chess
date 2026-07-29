use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;

use crate::ability::Ability;
use crate::action::Action;
use crate::action::GameResult;
use crate::action::Move;
use crate::action::Place;
use crate::action::PositionChange;
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

struct PlaceResult {
    piece: Piece,
    index: usize,
    changes: Vec<PositionChange>,
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
        Self::validate_pool(config)?;
        if !config.red_pool.is_empty() || !config.black_pool.is_empty() {
            config.board.validate_halves()?;
            Self::validate_alternation(config)?;
        }
        Self::validate_vital_result(config)?;
        Ok(())
    }

    fn validate_pool(config: &GameConfig) -> Result<(), String> {
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
        Ok(())
    }

    fn validate_alternation(config: &GameConfig) -> Result<(), String> {
        match config.result {
            GameResult::RedWin if config.player != Player::Red => {
                return Err(format!(
                    "result is {} but {} is to move",
                    config.result, config.player,
                ));
            },
            GameResult::BlackWin if config.player != Player::Black => {
                return Err(format!(
                    "result is {} but {} is to move",
                    config.result, config.player,
                ));
            },
            GameResult::Draw => {
                return Err(format!("result is {} in placement phase", config.result));
            },
            _ => {},
        }
        let check_player = match config.result {
            GameResult::RedWin => Player::Black,
            GameResult::BlackWin => Player::Red,
            _ => config.player,
        };
        let alternate = match check_player {
            Player::Red => config.red_pool.len() == config.black_pool.len(),
            Player::Black => config.red_pool.len() + 1 == config.black_pool.len(),
        };
        if !alternate {
            return Err(format!(
                "pool sizes cannot alternate: 红 pool {} pieces, 黑 pool {} pieces, but {} is to move",
                config.red_pool.len(),
                config.black_pool.len(),
                config.player,
            ));
        }
        Ok(())
    }

    fn validate_vital_result(config: &GameConfig) -> Result<(), String> {
        let red = Self::count_vital(config, Player::Red);
        if red > 1 {
            return Err(format!("{} must have at most one vital piece, found {red}", Player::Red));
        }
        let black = Self::count_vital(config, Player::Black);
        if black > 1 {
            return Err(format!(
                "{} must have at most one vital piece, found {black}",
                Player::Black
            ));
        }
        let valid = match config.result {
            GameResult::Unfinished => red > 0 && black > 0,
            GameResult::RedWin => red > 0,
            GameResult::BlackWin => black > 0,
            GameResult::Draw => true,
        };
        if valid {
            return Ok(());
        }
        Err(format!(
            "validate_vital_result failed, declared result is {}, but red has vital: {red}, black has vital: {black}",
            config.result
        ))
    }

    fn count_vital(config: &GameConfig, player: Player) -> usize {
        let pool = match player {
            Player::Red => &config.red_pool,
            Player::Black => &config.black_pool,
        };
        let pool = pool.iter().filter(|p| p.ability.has(Ability::VITAL)).count();
        let vital = |p: &Piece| p.color == player.color() && p.ability.has(Ability::VITAL);
        let board = config.board.iter().filter(vital).count();
        pool + board
    }

    /// Execute an action for the player to move. On success the turn
    /// passes to the opponent and the result is recomputed from the board.
    /// On error, self is unchanged. Error messages must stay single-line:
    /// the notation protocol renders them as one `错误：` line.
    pub fn action(&mut self, action: Action) -> Result<Reaction, String> {
        if self.result != GameResult::Unfinished {
            return Err(format!("game is already decided: {}", self.result));
        }
        match action {
            Action::Place(place) => {
                let pr = self.try_place(place)?;
                self.apply_place(pr.piece, pr.index, pr.changes, GameResult::Unfinished)
            },
            Action::Move(move_) => self.apply_move(self.try_move(move_)?),
            Action::Capture(move_) => self.apply_move(self.try_capture(move_)?),
            Action::Push(move_) => self.apply_move(self.try_push(move_)?),
            Action::Draw(move_) => {
                let changes = self.try_draw(move_)?;
                self.apply_draw(changes)
            },
            Action::Pass(player) => {
                self.try_pass(player)?;
                self.switch_player();
                Ok(Reaction { changes: vec![], game_result: self.result })
            },
            Action::Resign(player) => {
                let game_result = self.try_resign(player)?;
                self.result = game_result;
                self.switch_player();
                Ok(Reaction { changes: vec![], game_result })
            },
        }
    }

    /// Validate and simulate an action without modifying game state.
    /// Returns the [`Reaction`] the action would produce.
    pub fn try_action(&self, action: Action) -> Result<Reaction, String> {
        if self.result != GameResult::Unfinished {
            return Err(format!("game is already decided: {}", self.result));
        }
        match action {
            Action::Place(place) => {
                let pr = self.try_place(place)?;
                Ok(Reaction { changes: pr.changes, game_result: GameResult::Unfinished })
            },
            Action::Move(move_) => {
                let changes = self.try_move(move_)?;
                let game_result = self.move_result(&changes);
                Ok(Reaction { changes, game_result })
            },
            Action::Capture(move_) => {
                let changes = self.try_capture(move_)?;
                let game_result = self.move_result(&changes);
                Ok(Reaction { changes, game_result })
            },
            Action::Push(move_) => {
                let changes = self.try_push(move_)?;
                let game_result = self.move_result(&changes);
                Ok(Reaction { changes, game_result })
            },
            Action::Draw(move_) => {
                let changes = self.try_draw(move_)?;
                Ok(Reaction { changes, game_result: GameResult::Draw })
            },
            Action::Pass(player) => {
                self.try_pass(player)?;
                let result = self.move_result(&[]);
                Ok(Reaction { changes: vec![], game_result: result })
            },
            Action::Resign(player) => {
                let result = self.try_resign(player)?;
                Ok(Reaction { changes: vec![], game_result: result })
            },
        }
    }

    /// Enumerate all legal actions for the piece at `(x, y)` in the
    /// current position. Returns an empty list during the placement phase,
    /// when the position is empty, or when the piece is not controlled by
    /// the player to move.
    pub fn valid_moves(&self, x: u8, y: u8) -> Vec<Action> {
        if self.is_placement_phase() || self.result != GameResult::Unfinished {
            return vec![];
        }
        let Some(piece) = self.board.effective((x, y)) else {
            return vec![];
        };
        if !piece.can_controlled_by(self.player) {
            return vec![];
        }
        self.board.valid_moves(self.player, (x, y))
    }

    /// Positions where the current player may place a white piece.
    /// Returns empty when in placement phase or when `white_pool` is zero.
    /// Only the first CONTROL_WHITE piece is consulted — each side fields a
    /// single Wizard and no formation can grant CONTROL_WHITE, so there is
    /// at most one eligible piece per player.
    pub fn valid_white_placements(&self) -> Vec<(u8, u8)> {
        if self.is_placement_phase()
            || self.result != GameResult::Unfinished
            || self.white_pool == 0
        {
            return vec![];
        }
        self.board.valid_white_placements(self.player)
    }

    /// Non‑mutating placement validation (handles both colored and white).
    fn try_place(&self, place: Place) -> Result<PlaceResult, String> {
        let (piece, index) = self.find_in_pool(place.piece)?;
        if piece.color == Color::White {
            if self.is_placement_phase() {
                return Err("cannot place white pieces during the placement phase".into());
            }
            let changes = self.board.try_place_white(piece, place.to, self.player)?;
            return Ok(PlaceResult { piece, index, changes });
        }
        if piece.color != self.player.color() {
            return Err(format!(
                "player {} cannot place piece of color {}",
                self.player, place.piece.color
            ));
        }
        let changes = self.board.try_place(piece, place.to)?;
        Ok(PlaceResult { piece, index, changes })
    }

    fn find_in_pool(&self, piece: Piece) -> Result<(Piece, usize), String> {
        match piece.color {
            Color::White => {
                if piece != self.white {
                    return Err(format!("white piece {piece} does not match this game's"));
                }
                if self.white_pool == 0 {
                    return Err("no white pieces available to place".into());
                }
                Ok((self.white, 0))
            },
            Color::Red => {
                for (i, p) in self.red_pool.iter().enumerate() {
                    if *p == piece {
                        return Ok((self.red_pool[i], i));
                    }
                }
                Err(format!("piece {piece} not in pool"))
            },
            Color::Black => {
                for (i, p) in self.black_pool.iter().enumerate() {
                    if *p == piece {
                        return Ok((self.black_pool[i], i));
                    }
                }
                Err(format!("piece {piece} not in pool"))
            },
        }
    }

    fn apply_place(
        &mut self, piece: Piece, index: usize, changes: Vec<PositionChange>, result: GameResult,
    ) -> Result<Reaction, String> {
        self.board.apply(&changes);
        match piece.color {
            Color::Red => {
                self.red_pool.remove(index);
            },
            Color::Black => {
                self.black_pool.remove(index);
            },
            Color::White => self.white_pool -= 1,
        }
        self.switch_player();
        self.result = result;
        Ok(Reaction { changes, game_result: result })
    }

    fn try_move(&self, move_: Move) -> Result<Vec<PositionChange>, String> {
        self.check_move(move_.from)?;
        self.board.try_move(move_.from, move_.to)
    }

    fn try_push(&self, move_: Move) -> Result<Vec<PositionChange>, String> {
        self.check_move(move_.from)?;
        self.board.try_push(move_.from, move_.to)
    }

    fn try_capture(&self, move_: Move) -> Result<Vec<PositionChange>, String> {
        self.check_move(move_.from)?;
        self.board.try_capture(move_.from, move_.to)
    }

    fn try_draw(&self, move_: Move) -> Result<Vec<PositionChange>, String> {
        self.check_move(move_.from)?;
        let Some(target) = self.board.get(move_.to) else {
            return Err(format!("destination ({},{}) is empty", move_.to.0, move_.to.1));
        };
        let opponent_color = match self.player {
            Player::Red => Color::Black,
            Player::Black => Color::Red,
        };
        if target.color != opponent_color {
            return Err(format!("cannot draw with {} at ({},{})", target, move_.to.0, move_.to.1));
        }
        self.board.try_draw(move_.from, move_.to)
    }

    fn apply_draw(&mut self, changes: Vec<PositionChange>) -> Result<Reaction, String> {
        self.board.apply(&changes);
        self.white_pool += 1;
        self.switch_player();
        let game_result = GameResult::Draw;
        self.result = game_result;
        Ok(Reaction { changes, game_result })
    }

    fn check_move(&self, from: (u8, u8)) -> Result<(), String> {
        if self.is_placement_phase() {
            return Err("cannot move pieces during the placement phase".into());
        }
        let Some(piece) = self.board.effective(from) else {
            return Err(format!("no piece at ({},{})", from.0, from.1));
        };
        if !piece.can_controlled_by(self.player) {
            return Err(format!(
                "player {} cannot control piece {} at ({},{})",
                self.player, piece, from.0, from.1
            ));
        }
        Ok(())
    }

    fn apply_move(&mut self, changes: Vec<PositionChange>) -> Result<Reaction, String> {
        let captured = self.count_captured(&changes);
        let game_result = self.move_result(&changes);
        self.board.apply(&changes);
        self.white_pool += captured;
        self.switch_player();
        self.result = game_result;
        Ok(Reaction { changes, game_result })
    }

    fn count_captured(&self, changes: &[PositionChange]) -> u8 {
        let mut captured: i8 = 0;
        for change in changes {
            if self.board[change.at].is_some() {
                captured += 1;
            } else {
                captured -= 1;
            }
            if change.piece.is_some() {
                captured -= 1;
            } else {
                captured += 1;
            }
        }
        assert!(captured >= 0, "moving will never increase piece");
        (captured / 2) as u8
    }

    fn move_result(&self, changes: &[PositionChange]) -> GameResult {
        let red = self.move_vital(Color::Red, changes);
        let black = self.move_vital(Color::Black, changes);
        match (red, black) {
            (false, false) => GameResult::Draw,
            (false, true) => GameResult::BlackWin,
            (true, false) => GameResult::RedWin,
            (true, true) => GameResult::Unfinished,
        }
    }

    fn move_vital(&self, color: Color, changes: &[PositionChange]) -> bool {
        let mut removed = false;
        let mut added = false;
        for &change in changes {
            if let Some(old) = self.board.get(change.at)
                && old.color == color
                && old.ability.has(Ability::VITAL)
            {
                removed = true;
            }
            if let Some(new) = change.piece
                && new.color == color
                && new.ability.has(Ability::VITAL)
            {
                added = true;
            }
        }
        added || !removed
    }

    fn try_pass(&self, player: Player) -> Result<(), String> {
        if self.is_placement_phase() {
            return Err("cannot pass during the placement phase".into());
        }
        self.check_player(player)
    }

    fn try_resign(&self, player: Player) -> Result<GameResult, String> {
        self.check_player(player)?;
        let result = match self.player {
            Player::Red => GameResult::BlackWin,
            Player::Black => GameResult::RedWin,
        };
        Ok(result)
    }

    fn check_player(&self, player: Player) -> Result<(), String> {
        if player == self.player {
            return Ok(());
        }
        Err(format!("action declares player {player}, but player {} is to move", self.player))
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
    let Some(inner) = s.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Err(format!("pool must be bracketed: {s}"));
    };
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split_whitespace()
        .map(|s| {
            let mut chars = s.chars();
            let Some(name) = chars.next() else {
                return Err("empty piece name in pool".to_string());
            };
            if chars.next().is_some() {
                return Err(format!("piece name in pool must be a single character: {s}"));
            }
            let Some(piece) = Piece::lookup(name, color) else {
                return Err(format!("unknown piece in pool: {s}"));
            };
            Ok(piece)
        })
        .collect()
}

fn parse_config(s: &str) -> Result<GameConfig, String> {
    let mut lines = s.lines();
    let player_line = lines.next().ok_or("missing player line")?;
    let Some(player) = player_line.strip_prefix("行棋方：").and_then(|s| s.parse().ok()) else {
        return Err(format!("invalid player: {player_line}"));
    };

    let red_line = lines.next().ok_or("missing red pool")?;
    let red = parse_pool(red_line.strip_prefix("红方：").ok_or("invalid red pool")?, Color::Red)?;

    let black_line = lines.next().ok_or("missing black pool")?;
    let black =
        parse_pool(black_line.strip_prefix("黑方：").ok_or("invalid black pool")?, Color::Black)?;

    let white_line = lines.next().ok_or("missing white count")?;
    let Some(white_count) = white_line.strip_prefix("白方：").and_then(|s| s.parse().ok())
    else {
        return Err(format!("invalid white count: {white_line}"));
    };

    let result_line = lines.next().ok_or("missing result line")?;
    let Some(result) = result_line.strip_prefix("胜负：").and_then(|s| s.trim().parse().ok())
    else {
        return Err(format!("invalid result: {result_line}"));
    };

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
