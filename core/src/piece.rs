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

    /// The white piece: one-step cross movement, capturable by anyone,
    /// controlled by nobody until a wizard's formation covers it.
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
            pass_ally: false,
            pass_enemy: false,
            passed_by_ally: false,
            passed_by_enemy: true,
            capture: false,
            captured: true,
            capture_on_captured: false,
            captured_on_capture: false,
            jump_capture: false,
            any_distance: false,
            direction_cross: true,
            direction_diagonal: false,
            direction_shape_L: false,
            control_white: false,
            vital: false,
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
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: true,
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
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: true,
                vital: false,
            }
            .build(),
        }
    }

    const fn traitor(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '叛',
            color,
            formation: Self::orient(color, Formation::TRAITOR),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: false,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
            }
            .build(),
        }
    }

    const fn dog(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '犬',
            color,
            formation: Self::orient(color, Formation::DOG),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: false,
                direction_cross: false,
                direction_diagonal: true,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: true,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: false,
                direction_cross: false,
                direction_diagonal: false,
                direction_shape_L: true,
                control_white: false,
                vital: false,
            }
            .build(),
        }
    }

    const fn river(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '河',
            color,
            formation: Self::orient(color, Formation::RIVER),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: true,
                pass_enemy: true,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: true,
                passed_by_enemy: false,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
            }
            .build(),
        }
    }

    const fn cannon(color: Color) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(color);
        Piece {
            name: '炮',
            color,
            formation: Self::orient(color, Formation::CANNON),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: false,
                captured_on_capture: false,
                jump_capture: true,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
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
                pass_ally: false,
                pass_enemy: false,
                passed_by_ally: false,
                passed_by_enemy: true,
                capture: false,
                captured: true,
                capture_on_captured: true,
                captured_on_capture: true,
                jump_capture: false,
                any_distance: true,
                direction_cross: true,
                direction_diagonal: false,
                direction_shape_L: false,
                control_white: false,
                vital: false,
            }
            .build(),
        }
    }

    // Canonical piece definitions, one constant per (name, color).
    pub const RED_GENERAL: Piece = Self::general(Color::Red);
    pub const BLACK_GENERAL: Piece = Self::general(Color::Black);
    pub const RED_WIZARD: Piece = Self::wizard(Color::Red);
    pub const BLACK_WIZARD: Piece = Self::wizard(Color::Black);
    pub const RED_TRAITOR: Piece = Self::traitor(Color::Red);
    pub const BLACK_TRAITOR: Piece = Self::traitor(Color::Black);
    pub const RED_SPY: Piece = Self::spy(Color::Red);
    pub const BLACK_SPY: Piece = Self::spy(Color::Black);
    pub const RED_ROOK: Piece = Self::rook(Color::Red);
    pub const BLACK_ROOK: Piece = Self::rook(Color::Black);
    pub const RED_PAWN: Piece = Self::pawn(Color::Red);
    pub const BLACK_PAWN: Piece = Self::pawn(Color::Black);
    pub const RED_DOG: Piece = Self::dog(Color::Red);
    pub const BLACK_DOG: Piece = Self::dog(Color::Black);
    pub const RED_HORSE: Piece = Self::horse(Color::Red);
    pub const BLACK_HORSE: Piece = Self::horse(Color::Black);
    pub const RED_RIVER: Piece = Self::river(Color::Red);
    pub const BLACK_RIVER: Piece = Self::river(Color::Black);
    pub const RED_MOUNTAIN: Piece = Self::mountain(Color::Red);
    pub const BLACK_MOUNTAIN: Piece = Self::mountain(Color::Black);
    pub const RED_WIND: Piece = Self::wind(Color::Red);
    pub const BLACK_WIND: Piece = Self::wind(Color::Black);
    pub const RED_FOREST: Piece = Self::forest(Color::Red);
    pub const BLACK_FOREST: Piece = Self::forest(Color::Black);
    pub const RED_SPEAR: Piece = Self::spear(Color::Red);
    pub const BLACK_SPEAR: Piece = Self::spear(Color::Black);
    pub const RED_SHIELD: Piece = Self::shield(Color::Red);
    pub const BLACK_SHIELD: Piece = Self::shield(Color::Black);
    pub const RED_CANNON: Piece = Self::cannon(Color::Red);
    pub const BLACK_CANNON: Piece = Self::cannon(Color::Black);
    pub const RED_MINE: Piece = Self::mine(Color::Red);
    pub const BLACK_MINE: Piece = Self::mine(Color::Black);

    /// The standard 16-piece red army, used as the default red pool.
    pub const RED_PLAYER_PIECES: [Piece; 16] = [
        Piece::RED_GENERAL,
        Piece::RED_WIZARD,
        Piece::RED_TRAITOR,
        Piece::RED_SPY,
        Piece::RED_ROOK,
        Piece::RED_PAWN,
        Piece::RED_DOG,
        Piece::RED_HORSE,
        Piece::RED_RIVER,
        Piece::RED_MOUNTAIN,
        Piece::RED_WIND,
        Piece::RED_FOREST,
        Piece::RED_SPEAR,
        Piece::RED_SHIELD,
        Piece::RED_CANNON,
        Piece::RED_MINE,
    ];

    /// The standard 16-piece black army, used as the default black pool.
    pub const BLACK_PLAYER_PIECES: [Piece; 16] = [
        Piece::BLACK_GENERAL,
        Piece::BLACK_WIZARD,
        Piece::BLACK_TRAITOR,
        Piece::BLACK_SPY,
        Piece::BLACK_ROOK,
        Piece::BLACK_PAWN,
        Piece::BLACK_DOG,
        Piece::BLACK_HORSE,
        Piece::BLACK_RIVER,
        Piece::BLACK_MOUNTAIN,
        Piece::BLACK_WIND,
        Piece::BLACK_FOREST,
        Piece::BLACK_SPEAR,
        Piece::BLACK_SHIELD,
        Piece::BLACK_CANNON,
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
            ('叛', Color::Red) => Piece::RED_TRAITOR,
            ('叛', Color::Black) => Piece::BLACK_TRAITOR,
            ('谍', Color::Red) => Piece::RED_SPY,
            ('谍', Color::Black) => Piece::BLACK_SPY,
            ('车', Color::Red) => Piece::RED_ROOK,
            ('车', Color::Black) => Piece::BLACK_ROOK,
            ('卒', Color::Red) => Piece::RED_PAWN,
            ('卒', Color::Black) => Piece::BLACK_PAWN,
            ('犬', Color::Red) => Piece::RED_DOG,
            ('犬', Color::Black) => Piece::BLACK_DOG,
            ('马', Color::Red) => Piece::RED_HORSE,
            ('马', Color::Black) => Piece::BLACK_HORSE,
            ('河', Color::Red) => Piece::RED_RIVER,
            ('河', Color::Black) => Piece::BLACK_RIVER,
            ('山', Color::Red) => Piece::RED_MOUNTAIN,
            ('山', Color::Black) => Piece::BLACK_MOUNTAIN,
            ('风', Color::Red) => Piece::RED_WIND,
            ('风', Color::Black) => Piece::BLACK_WIND,
            ('林', Color::Red) => Piece::RED_FOREST,
            ('林', Color::Black) => Piece::BLACK_FOREST,
            ('矛', Color::Red) => Piece::RED_SPEAR,
            ('矛', Color::Black) => Piece::BLACK_SPEAR,
            ('盾', Color::Red) => Piece::RED_SHIELD,
            ('盾', Color::Black) => Piece::BLACK_SHIELD,
            ('炮', Color::Red) => Piece::RED_CANNON,
            ('炮', Color::Black) => Piece::BLACK_CANNON,
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

    /// Whether this piece can move through `blocker`: same color needs
    /// mover PASS_ALLY **or** blocker PASSED_BY_ALLY; different colors need
    /// mover PASS_ENEMY **and** blocker PASSED_BY_ENEMY.
    pub fn can_pass(&self, blocker: Piece) -> bool {
        if self.color == blocker.color {
            self.ability.has_ability(Ability::PASS_ALLY)
                || blocker.ability.has_ability(Ability::PASSED_BY_ALLY)
        } else {
            self.ability.has_ability(Ability::PASS_ENEMY)
                && blocker.ability.has_ability(Ability::PASSED_BY_ENEMY)
        }
    }

    /// Whether this piece can shove `target`: same color needs mover
    /// PUSH_ALLY **or** target PUSHED_BY_ALLY; different colors need mover
    /// PUSH_ENEMY **and** target PUSHED_BY_ENEMY.
    pub fn can_push(&self, target: Piece) -> bool {
        if self.color == target.color {
            self.ability.has_ability(Ability::PUSH_ALLY)
                || target.ability.has_ability(Ability::PUSHED_BY_ALLY)
        } else {
            self.ability.has_ability(Ability::PUSH_ENEMY)
                && target.ability.has_ability(Ability::PUSHED_BY_ENEMY)
        }
    }

    /// Whether this piece can capture `target` normally: different colors,
    /// mover CAPTURE, target CAPTURED. Path rules are the caller's concern.
    pub fn can_capture(&self, target: Piece) -> bool {
        self.color != target.color
            && self.ability.has_ability(Ability::CAPTURE)
            && target.ability.has_ability(Ability::CAPTURED)
    }

    /// Whether this piece can jump-capture `target` over `piece_count`
    /// screen pieces: different colors, mover JUMP_CAPTURE, target
    /// CAPTURED, and exactly one piece on the path (passable or not).
    pub fn can_jump_capture(&self, target: Piece, piece_count: u8) -> bool {
        self.color != target.color
            && self.ability.has_ability(Ability::JUMP_CAPTURE)
            && target.ability.has_ability(Ability::CAPTURED)
            && piece_count == 1
    }

    /// Whether `player` may command this piece, per its CONTROLLED_BY_*
    /// abilities.
    pub fn can_controlled_by(&self, player: Player) -> bool {
        match player {
            Player::Red => self.ability.has_ability(Ability::CONTROLLED_BY_RED),
            Player::Black => self.ability.has_ability(Ability::CONTROLLED_BY_BLACK),
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
