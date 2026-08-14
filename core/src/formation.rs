#![expect(clippy::unusual_byte_groupings)]

use crate::ability::Ability;
use crate::piece::Player;

/// A piece's formation: which of the eight surrounding points its aura
/// covers, and how it modifies the abilities of pieces standing there.
#[derive(Debug, Copy, Clone)]
pub struct Formation {
    /// Bit set of covered points; see the `TOP_LEFT` … `BOTTOM_RIGHT`
    /// constants for the bit layout.
    pub points: u8,
    /// (owner, object) -> (mask, update): the formation overwrites the
    /// `mask` bits of the object piece's abilities with `update`. When
    /// several formations mask the same bit, the updates are ANDed, so
    /// disabling always wins.
    pub effect: fn(Player, Player) -> (Ability, Ability),
}

impl Formation {
    pub const GENERAL: Self = Self { points: Self::CORNER, effect: Self::general };
    pub const STRATAGEM: Self = Self { points: Self::CORNER, effect: Self::stratagem };
    pub const MOMENTUM: Self = Self { points: Self::CORNER, effect: Self::momentum };
    pub const ADAPTATION: Self = Self { points: Self::CORNER, effect: Self::adaptation };
    pub const WIND: Self = Self { points: Self::MIDDLE, effect: Self::wind };
    pub const FOREST: Self = Self { points: Self::MIDDLE, effect: Self::forest };
    pub const FIRE: Self = Self { points: Self::MIDDLE, effect: Self::fire };
    pub const MOUNTAIN: Self = Self { points: Self::MIDDLE, effect: Self::mountain };
    pub const SPEAR: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::spear };
    pub const SHIELD: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::shield };
    pub const SHELL: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::shell };
    pub const MINE: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::mine };
    pub const SCHOLAR: Self = Self { points: Self::LOWER_PENTAGON, effect: Self::scholar };
    pub const PAWN: Self = Self { points: Self::LOWER_PENTAGON, effect: Self::pawn };
    pub const HORSE: Self = Self { points: Self::LOWER_PENTAGON, effect: Self::horse };
    pub const ROOK: Self = Self { points: Self::LOWER_PENTAGON, effect: Self::rook };

    // Point-set bit layout: bits 0-2 top row, 3-4 middle sides, 5-7 bottom
    // row, in the digit groups written as 0b<bottom>_<middle>_<top>.
    pub const POINTS_FULL: u8 = 0b111_11_111;
    pub const CORNER: u8 = 0b101_00_101;
    pub const MIDDLE: u8 = 0b010_11_010;
    pub const UPPER_TRIANGLE: u8 = 0b101_00_010;
    pub const LOWER_PENTAGON: u8 = 0b010_11_101;

    pub const TOP_LEFT: u8 = 0b000_00_001;
    pub const TOP_MIDDLE: u8 = 0b000_00_010;
    pub const TOP_RIGHT: u8 = 0b000_00_100;
    pub const MIDDLE_LEFT: u8 = 0b000_01_000;
    pub const MIDDLE_RIGHT: u8 = 0b000_10_000;
    pub const BOTTOM_LEFT: u8 = 0b001_00_000;
    pub const BOTTOM_MIDDLE: u8 = 0b010_00_000;
    pub const BOTTOM_RIGHT: u8 = 0b100_00_000;

    /// Grant `mask` to allies and strip it from enemies.
    fn grant_allies_strip_enemies(
        owner: Player, object: Player, mask: Ability,
    ) -> (Ability, Ability) {
        if owner == object {
            return (mask, mask);
        }
        (mask, Ability::NONE)
    }

    /// Allies gain Peace Talk; enemies lose it.
    pub fn general(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::PEACE_TALK)
    }

    /// Allies lose enemy control; enemies gain enemy control.
    pub fn stratagem(owner: Player, object: Player) -> (Ability, Ability) {
        let update = if owner == object { Ability::NONE } else { Ability::PASSIVITY };
        (Ability::PASSIVITY, update)
    }

    /// Allies gain Hidden Capture; enemies gain Easy Capture.
    pub fn momentum(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::HIDDEN_CAPTURE | Ability::EASY_CAPTURE;
        let update = if owner == object { Ability::HIDDEN_CAPTURE } else { Ability::EASY_CAPTURE };
        (mask, update)
    }

    /// Allies gain Hard Capture; enemies gain Overt Capture.
    pub fn adaptation(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::OVERT_CAPTURE | Ability::HARD_CAPTURE;
        let update = if owner == object { Ability::HARD_CAPTURE } else { Ability::OVERT_CAPTURE };
        (mask, update)
    }

    /// Allies gain Diagonal Move; enemies lose it.
    pub fn scholar(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIAGONAL_MOVE)
    }

    /// Allies gain Orthogonal Move; enemies lose it.
    pub fn pawn(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::ORTHOGONAL_MOVE)
    }

    /// Allies gain Broad Step; enemies lose it.
    pub fn horse(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::BROAD_STEP)
    }

    /// Allies gain Swift Move; enemies lose it.
    pub fn rook(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::SWIFT_MOVE)
    }

    /// Allies gain Pull Ally and Pull Enemy; enemies lose both.
    pub fn wind(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PULL_ALLY | Ability::PULL_ENEMY;
        Self::grant_allies_strip_enemies(owner, object, mask)
    }

    /// Replaces both passive pull abilities: allies gain Ally Pull and lose Enemy Pull;
    /// enemies gain Enemy Pull and lose Ally Pull.
    pub fn forest(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::ALLY_PULL | Ability::ENEMY_PULL;
        let update = if owner == object { Ability::ALLY_PULL } else { Ability::ENEMY_PULL };
        (mask, update)
    }

    /// Allies gain Push Ally and Push Enemy; enemies lose both.
    pub fn fire(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PUSH_ALLY | Ability::PUSH_ENEMY;
        Self::grant_allies_strip_enemies(owner, object, mask)
    }

    /// Replaces both passive push abilities: allies gain Ally Push and lose Enemy Push;
    /// enemies gain Enemy Push and lose Ally Push.
    pub fn mountain(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::ALLY_PUSH | Ability::ENEMY_PUSH;
        let update = if owner == object { Ability::ALLY_PUSH } else { Ability::ENEMY_PUSH };
        (mask, update)
    }

    /// Allies gain Capture; enemies lose it.
    pub fn spear(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURE)
    }

    /// Allies become uncapturable; enemies become capturable.
    pub fn shield(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::CAPTURABLE;
        let update = if owner == object { Ability::NONE } else { Ability::CAPTURABLE };
        (mask, update)
    }

    /// Allies gain Force Capture; enemies lose it.
    pub fn shell(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::FORCE_CAPTURE)
    }

    /// Allies gain Counter Capture; enemies lose it.
    pub fn mine(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::COUNTER_CAPTURE)
    }

    /// Whether the formation covers the neighbor at relative offset (dx, dy),
    /// each in [-1, 1] and not both zero. Offsets outside this range return
    /// false (pieces farther apart cannot be in each other's formation).
    pub fn contains(&self, dx: i8, dy: i8) -> bool {
        let mask = match (dx, dy) {
            (-1, -1) => Self::TOP_LEFT,
            (0, -1) => Self::TOP_MIDDLE,
            (1, -1) => Self::TOP_RIGHT,
            (-1, 0) => Self::MIDDLE_LEFT,
            (1, 0) => Self::MIDDLE_RIGHT,
            (-1, 1) => Self::BOTTOM_LEFT,
            (0, 1) => Self::BOTTOM_MIDDLE,
            (1, 1) => Self::BOTTOM_RIGHT,
            _ => return false,
        };
        (self.points & mask) != 0
    }

    /// Return a copy with the top and bottom rows of the points bitmap
    /// swapped (vertical mirror). The middle (left/right) row is unchanged.
    /// This is used to orient formations for Black, whose advance direction
    /// is opposite to the canonical Red-oriented layout.
    pub const fn flipped(self) -> Self {
        let top = (self.points & 0b000_00_111) << 5;
        let bottom = (self.points & 0b111_00_000) >> 5;
        let middle = self.points & 0b000_11_000;
        Self { points: top | bottom | middle, effect: self.effect }
    }
}
