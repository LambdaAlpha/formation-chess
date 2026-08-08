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

    /// Allies gain DRAW; enemies lose it.
    pub fn general(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DRAW)
    }

    /// Allies gain control; enemies lose control.
    pub fn stratagem(owner: Player, object: Player) -> (Ability, Ability) {
        match (owner, object) {
            (Player::Red, Player::Red) => (Ability::CONTROLLED_BY_BLACK, Ability::NONE),
            (Player::Red, Player::Black) => {
                (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED)
            },
            (Player::Black, Player::Black) => (Ability::CONTROLLED_BY_RED, Ability::NONE),
            (Player::Black, Player::Red) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
        }
    }

    /// Allies gain active push escalation; enemies gain passive escalation.
    pub fn momentum(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::CAPTURE_ON_PUSH_BLOCKED | Ability::CAPTURED_ON_PUSH_BLOCKED;
        let update = if owner == object {
            Ability::CAPTURE_ON_PUSH_BLOCKED
        } else {
            Ability::CAPTURED_ON_PUSH_BLOCKED
        };
        (mask, update)
    }

    /// Allies gain passive capture demotion; enemies gain active demotion.
    pub fn adaptation(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PUSH_ON_CAPTURE_UNBLOCKED | Ability::PUSHED_ON_CAPTURE_UNBLOCKED;
        let update = if owner == object {
            Ability::PUSHED_ON_CAPTURE_UNBLOCKED
        } else {
            Ability::PUSH_ON_CAPTURE_UNBLOCKED
        };
        (mask, update)
    }

    /// Allies gain DIRECTION_DIAGONAL; enemies lose it.
    pub fn scholar(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_DIAGONAL)
    }

    /// Allies gain DIRECTION_CROSS; enemies lose it.
    pub fn pawn(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_CROSS)
    }

    /// Allies gain DIRECTION_SHAPE_L; enemies lose it.
    pub fn horse(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_SHAPE_L)
    }

    /// Allies gain ANY_DISTANCE; enemies lose it.
    pub fn rook(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::ANY_DISTANCE)
    }

    /// Allies gain both pull abilities; enemies lose both.
    pub fn wind(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PULL_ALLY | Ability::PULL_ENEMY;
        Self::grant_allies_strip_enemies(owner, object, mask)
    }

    /// Takes over both pulled-by abilities: allies become pullable by
    /// allies only, enemies pullable by the forest's side only.
    pub fn forest(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PULLED_BY_ALLY | Ability::PULLED_BY_ENEMY;
        let update =
            if owner == object { Ability::PULLED_BY_ALLY } else { Ability::PULLED_BY_ENEMY };
        (mask, update)
    }

    /// Allies gain both push abilities; enemies lose both.
    pub fn fire(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PUSH_ALLY | Ability::PUSH_ENEMY;
        Self::grant_allies_strip_enemies(owner, object, mask)
    }

    /// Takes over both pushed-by abilities: allies become pushable by
    /// allies only, enemies pushable by the mountain's side only.
    pub fn mountain(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::PUSHED_BY_ALLY | Ability::PUSHED_BY_ENEMY;
        let update =
            if owner == object { Ability::PUSHED_BY_ALLY } else { Ability::PUSHED_BY_ENEMY };
        (mask, update)
    }

    /// Allies gain CAPTURE; enemies lose it.
    pub fn spear(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURE)
    }

    /// Allies become uncapturable; enemies become capturable.
    pub fn shield(owner: Player, object: Player) -> (Ability, Ability) {
        let mask = Ability::CAPTURED;
        let update = if owner == object { Ability::NONE } else { Ability::CAPTURED };
        (mask, update)
    }

    /// Allies gain CAPTURED_ON_CAPTURE; enemies lose it.
    pub fn shell(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURED_ON_CAPTURE)
    }

    /// Allies gain CAPTURE_ON_CAPTURED; enemies lose it.
    pub fn mine(owner: Player, object: Player) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURE_ON_CAPTURED)
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
