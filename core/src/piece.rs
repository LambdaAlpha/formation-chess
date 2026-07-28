use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;

use crate::ability::Ability;
use crate::ability::AbilityConfig;
use crate::board::Neighbor;
use crate::formation::Formation;

/// A piece: its kind (name + color), formation, and current abilities.
/// Note that equality compares **only** name and color — two pieces with
/// different abilities (e.g. before and after formation effects) are still
/// equal.
#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub name: char,
    pub color: Color,
    pub formation: Formation,
    pub ability: Ability,
}

/// A piece color. Unlike [`Player`], this includes White, the color of
/// captured-and-recycled pieces owned by neither player.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
    White,
}

/// One of the two players.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Player {
    Red,
    Black,
}

impl Piece {
    /// (controlled_by_red, controlled_by_black) for a piece of `color`.
    const fn controlled(color: Color) -> (bool, bool) {
        match color {
            Color::Red => (true, false),
            Color::Black => (false, true),
            Color::White => (false, false),
        }
    }

    /// Vertically mirror a formation for Black pieces, whose advance
    /// direction points toward the bottom of the board.
    const fn orient(color: Color, formation: Formation) -> Formation {
        match color {
            Color::Black => formation.flipped(),
            _ => formation,
        }
    }

    /// The white piece: cross + diagonal + L-shaped movement at any
    /// distance, capturable by anyone, controlled by nobody until a
    /// wizard's formation covers it.
    pub const WHITE: Piece = Piece {
        name: '子',
        color: Color::White,
        formation: Formation::WHITE,
        ability: AbilityConfig {
            controlled_by_red: false,
            controlled_by_black: false,
            push_ally: false,
            push_enemy: false,
            pushed_by_ally: false,
            pushed_by_enemy: true,
            capture_on_push_blocked: false,
            captured_on_push_blocked: false,
            push_on_capture_unblocked: false,
            pushed_on_capture_unblocked: false,
            capture: false,
            captured: true,
            capture_on_captured: false,
            captured_on_capture: false,
            any_distance: true,
            direction_cross: true,
            direction_diagonal: true,
            direction_shape_L: true,
            control_white: false,
            vital: false,
            draw: false,
        }
        .build(),
    };

    const fn general(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '将',
            color,
            formation: Self::orient(color, Formation::GENERAL),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: true,
                draw: true,
            }
            .build(),
        }
    }

    const fn wizard(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '巫',
            color,
            formation: Self::orient(color, Formation::WIZARD),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: true,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn agent(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '间',
            color,
            formation: Self::orient(color, Formation::AGENT),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn spy(color: Color) -> Piece {
        Piece {
            name: '谍',
            color,
            formation: Self::orient(color, Formation::SPY),
            ability: AbilityConfig {
                controlled_by_red: true,
                controlled_by_black: true,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn rook(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '车',
            color,
            formation: Self::orient(color, Formation::ROOK),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn pawn(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '卒',
            color,
            formation: Self::orient(color, Formation::PAWN),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: false,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn scholar(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '士',
            color,
            formation: Self::orient(color, Formation::SCHOLAR),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: false,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn horse(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '马',
            color,
            formation: Self::orient(color, Formation::HORSE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: false,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn wind(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '风',
            color,
            formation: Self::orient(color, Formation::WIND),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn mountain(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '山',
            color,
            formation: Self::orient(color, Formation::MOUNTAIN),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: true,
                pushed_by_enemy: false,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn fire(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '火',
            color,
            formation: Self::orient(color, Formation::FIRE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: true,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn forest(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '林',
            color,
            formation: Self::orient(color, Formation::FOREST),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn spear(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '矛',
            color,
            formation: Self::orient(color, Formation::SPEAR),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn shield(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '盾',
            color,
            formation: Self::orient(color, Formation::SHIELD),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: false,
                capture_on_captured: false,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn shell(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '弹',
            color,
            formation: Self::orient(color, Formation::SHELL),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: true,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn mine(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '雷',
            color,
            formation: Self::orient(color, Formation::MINE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                capture_on_push_blocked: false,
                captured_on_push_blocked: false,
                push_on_capture_unblocked: false,
                pushed_on_capture_unblocked: false,
                capture: false,
                captured: true,
                capture_on_captured: true,
                captured_on_capture: false,
                any_distance: true,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    // Canonical piece definitions, one constant per (name, color).
    pub const RED_GENERAL: Piece = Self::general(Color::Red);
    pub const BLACK_GENERAL: Piece = Self::general(Color::Black);
    pub const RED_WIZARD: Piece = Self::wizard(Color::Red);
    pub const BLACK_WIZARD: Piece = Self::wizard(Color::Black);
    pub const RED_AGENT: Piece = Self::agent(Color::Red);
    pub const BLACK_AGENT: Piece = Self::agent(Color::Black);
    pub const RED_SPY: Piece = Self::spy(Color::Red);
    pub const BLACK_SPY: Piece = Self::spy(Color::Black);
    pub const RED_ROOK: Piece = Self::rook(Color::Red);
    pub const BLACK_ROOK: Piece = Self::rook(Color::Black);
    pub const RED_PAWN: Piece = Self::pawn(Color::Red);
    pub const BLACK_PAWN: Piece = Self::pawn(Color::Black);
    pub const RED_SCHOLAR: Piece = Self::scholar(Color::Red);
    pub const BLACK_SCHOLAR: Piece = Self::scholar(Color::Black);
    pub const RED_HORSE: Piece = Self::horse(Color::Red);
    pub const BLACK_HORSE: Piece = Self::horse(Color::Black);
    pub const RED_WIND: Piece = Self::wind(Color::Red);
    pub const BLACK_WIND: Piece = Self::wind(Color::Black);
    pub const RED_MOUNTAIN: Piece = Self::mountain(Color::Red);
    pub const BLACK_MOUNTAIN: Piece = Self::mountain(Color::Black);
    pub const RED_FIRE: Piece = Self::fire(Color::Red);
    pub const BLACK_FIRE: Piece = Self::fire(Color::Black);
    pub const RED_FOREST: Piece = Self::forest(Color::Red);
    pub const BLACK_FOREST: Piece = Self::forest(Color::Black);
    pub const RED_SPEAR: Piece = Self::spear(Color::Red);
    pub const BLACK_SPEAR: Piece = Self::spear(Color::Black);
    pub const RED_SHIELD: Piece = Self::shield(Color::Red);
    pub const BLACK_SHIELD: Piece = Self::shield(Color::Black);
    pub const RED_SHELL: Piece = Self::shell(Color::Red);
    pub const BLACK_SHELL: Piece = Self::shell(Color::Black);
    pub const RED_MINE: Piece = Self::mine(Color::Red);
    pub const BLACK_MINE: Piece = Self::mine(Color::Black);

    /// The standard 16-piece red army, used as the default red pool.
    pub const RED_PLAYER_PIECES: [Piece; 16] = [
        Piece::RED_GENERAL,
        Piece::RED_WIZARD,
        Piece::RED_AGENT,
        Piece::RED_SPY,
        Piece::RED_ROOK,
        Piece::RED_PAWN,
        Piece::RED_SCHOLAR,
        Piece::RED_HORSE,
        Piece::RED_WIND,
        Piece::RED_MOUNTAIN,
        Piece::RED_FIRE,
        Piece::RED_FOREST,
        Piece::RED_SPEAR,
        Piece::RED_SHIELD,
        Piece::RED_SHELL,
        Piece::RED_MINE,
    ];

    /// The standard 16-piece black army, used as the default black pool.
    pub const BLACK_PLAYER_PIECES: [Piece; 16] = [
        Piece::BLACK_GENERAL,
        Piece::BLACK_WIZARD,
        Piece::BLACK_AGENT,
        Piece::BLACK_SPY,
        Piece::BLACK_ROOK,
        Piece::BLACK_PAWN,
        Piece::BLACK_SCHOLAR,
        Piece::BLACK_HORSE,
        Piece::BLACK_WIND,
        Piece::BLACK_MOUNTAIN,
        Piece::BLACK_FIRE,
        Piece::BLACK_FOREST,
        Piece::BLACK_SPEAR,
        Piece::BLACK_SHIELD,
        Piece::BLACK_SHELL,
        Piece::BLACK_MINE,
    ];

    /// The canonical piece for a name and color, or None for an unknown
    /// combination (e.g. a white 将, or a name that is not a piece).
    pub fn lookup(name: char, color: Color) -> Option<Piece> {
        let piece = match (name, color) {
            ('子', Color::White) => Piece::WHITE,
            ('将', Color::Red) => Piece::RED_GENERAL,
            ('将', Color::Black) => Piece::BLACK_GENERAL,
            ('巫', Color::Red) => Piece::RED_WIZARD,
            ('巫', Color::Black) => Piece::BLACK_WIZARD,
            ('间', Color::Red) => Piece::RED_AGENT,
            ('间', Color::Black) => Piece::BLACK_AGENT,
            ('谍', Color::Red) => Piece::RED_SPY,
            ('谍', Color::Black) => Piece::BLACK_SPY,
            ('车', Color::Red) => Piece::RED_ROOK,
            ('车', Color::Black) => Piece::BLACK_ROOK,
            ('卒', Color::Red) => Piece::RED_PAWN,
            ('卒', Color::Black) => Piece::BLACK_PAWN,
            ('士', Color::Red) => Piece::RED_SCHOLAR,
            ('士', Color::Black) => Piece::BLACK_SCHOLAR,
            ('马', Color::Red) => Piece::RED_HORSE,
            ('马', Color::Black) => Piece::BLACK_HORSE,
            ('风', Color::Red) => Piece::RED_WIND,
            ('风', Color::Black) => Piece::BLACK_WIND,
            ('山', Color::Red) => Piece::RED_MOUNTAIN,
            ('山', Color::Black) => Piece::BLACK_MOUNTAIN,
            ('火', Color::Red) => Piece::RED_FIRE,
            ('火', Color::Black) => Piece::BLACK_FIRE,
            ('林', Color::Red) => Piece::RED_FOREST,
            ('林', Color::Black) => Piece::BLACK_FOREST,
            ('矛', Color::Red) => Piece::RED_SPEAR,
            ('矛', Color::Black) => Piece::BLACK_SPEAR,
            ('盾', Color::Red) => Piece::RED_SHIELD,
            ('盾', Color::Black) => Piece::BLACK_SHIELD,
            ('弹', Color::Red) => Piece::RED_SHELL,
            ('弹', Color::Black) => Piece::BLACK_SHELL,
            ('雷', Color::Red) => Piece::RED_MINE,
            ('雷', Color::Black) => Piece::BLACK_MINE,
            _ => return None,
        };
        Some(piece)
    }

    /// Compute the effective abilities of this piece considering all surrounding
    /// formations. `neighbors` are the raw pieces around it (see
    /// [`crate::board::Board::local`]); effects combine order-independently,
    /// with disabling winning over enabling.
    pub fn take_effect(&mut self, neighbors: &[Neighbor]) {
        let (mut effect_mask, mut effect_update) = (Ability::NONE, !Ability::NONE);
        // must be independent of effect application order
        for n in neighbors {
            let Some(neighbor) = n.piece else {
                continue;
            };
            if !neighbor.formation.contains(-n.dx, -n.dy) {
                continue;
            }
            let (mask, update) = (neighbor.formation.effect)(neighbor.color, self.color);
            effect_update = effect_update.masked_and(mask, update);
            effect_mask |= mask;
        }
        self.ability = self.ability.masked_set(effect_mask, effect_update);
    }

    /// Whether this piece can shove `target`: same color needs mover
    /// PUSH_ALLY **or** target PUSHED_BY_ALLY; different colors need mover
    /// PUSH_ENEMY **and** target PUSHED_BY_ENEMY.
    pub fn can_push(&self, target: Piece) -> bool {
        if self.color == target.color {
            self.ability.has(Ability::PUSH_ALLY) || target.ability.has(Ability::PUSHED_BY_ALLY)
        } else {
            self.ability.has(Ability::PUSH_ENEMY) && target.ability.has(Ability::PUSHED_BY_ENEMY)
        }
    }

    /// Whether this piece can capture `target` normally or through
    /// mutual-destruction bypass: different colors, and either
    /// (i) attacker has CAPTURED_ON_CAPTURE (sacrifice, bypasses target's
    /// CAPTURED), (ii) target has CAPTURE_ON_CAPTURED (retaliation, bypasses
    /// attacker's CAPTURE), or (iii) attacker has CAPTURE and target has
    /// CAPTURED. Path rules are the caller's concern.
    pub fn can_capture(&self, target: Piece) -> bool {
        if self.color == target.color {
            return false;
        }
        if self.ability.has(Ability::CAPTURED_ON_CAPTURE)
            || target.ability.has(Ability::CAPTURE_ON_CAPTURED)
        {
            return true;
        }
        self.ability.has(Ability::CAPTURE) && target.ability.has(Ability::CAPTURED)
    }

    /// Whether `player` may command this piece, per its CONTROLLED_BY_*
    /// abilities.
    pub fn can_controlled_by(&self, player: Player) -> bool {
        match player {
            Player::Red => self.ability.has(Ability::CONTROLLED_BY_RED),
            Player::Black => self.ability.has(Ability::CONTROLLED_BY_BLACK),
        }
    }
}

/// Equality is **name and color only**; formation and abilities are
/// ignored. Pool lookups and board searches rely on this.
impl PartialEq for Piece {
    fn eq(&self, other: &Piece) -> bool {
        self.name == other.name && self.color == other.color
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.color, self.name)
    }
}

impl FromStr for Piece {
    type Err = String;
    /// Parse a color-prefixed piece name (e.g. `红车`) into its canonical piece.
    fn from_str(s: &str) -> Result<Self, String> {
        let mut indices = s.char_indices();
        let (Some(_), Some((name_start, name)), None) =
            (indices.next(), indices.next(), indices.next())
        else {
            return Err(format!("invalid piece: {s}"));
        };
        let color: Color = s[.. name_start].parse()?;
        let Some(piece) = Self::lookup(name, color) else {
            return Err(format!("unknown piece: {s}"));
        };
        Ok(piece)
    }
}

impl Player {
    /// The [`Color`] this player's own pieces carry.
    pub fn color(self) -> Color {
        match self {
            Player::Red => Color::Red,
            Player::Black => Color::Black,
        }
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Player::Red => "红",
            Player::Black => "黑",
        };
        f.write_str(s)
    }
}

impl Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl FromStr for Player {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "红" => Ok(Player::Red),
            "黑" => Ok(Player::Black),
            _ => Err(format!("unknown player: {s}")),
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Color::Red => "红",
            Color::Black => "黑",
            Color::White => "白",
        };
        write!(f, "{s}")
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl FromStr for Color {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "红" => Ok(Color::Red),
            "黑" => Ok(Color::Black),
            "白" => Ok(Color::White),
            _ => Err(format!("unknown color: {s}")),
        }
    }
}
