#![expect(clippy::unusual_byte_groupings)]

use crate::ability::Ability;
use crate::piece::Color;

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
    pub effect: fn(Color, Color) -> (Ability, Ability),
}

impl Formation {
    pub const GENERAL: Self = Self { points: Self::CORNER, effect: Self::general };
    pub const WIZARD: Self = Self { points: Self::CORNER, effect: Self::wizard };
    pub const TRAITOR: Self = Self { points: Self::CORNER, effect: Self::traitor };
    pub const SPY: Self = Self { points: Self::CORNER, effect: Self::spy };
    pub const ROOK: Self = Self { points: Self::MIDDLE, effect: Self::rook };
    pub const PAWN: Self = Self { points: Self::MIDDLE, effect: Self::pawn };
    pub const DOG: Self = Self { points: Self::MIDDLE, effect: Self::dog };
    pub const HORSE: Self = Self { points: Self::MIDDLE, effect: Self::horse };
    pub const RIVER: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::river };
    pub const MOUNTAIN: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::mountain };
    pub const WIND: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::wind };
    pub const FOREST: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::forest };
    pub const SPEAR: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::spear };
    pub const SHIELD: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::shield };
    pub const CANNON: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::cannon };
    pub const MINE: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::mine };
    pub const WHITE: Self = Self { points: Self::POINTS_NONE, effect: Self::white };

    // Point-set bit layout: bits 0-2 top row, 3-4 middle sides, 5-7 bottom
    // row, in the digit groups written as 0b<bottom>_<middle>_<top>.
    pub const POINTS_FULL: u8 = 0b111_11_111;
    pub const CORNER: u8 = 0b101_00_101;
    pub const MIDDLE: u8 = 0b010_11_010;
    pub const UPPER_TRIANGLE: u8 = 0b101_00_010;
    pub const LOWER_TRIANGLE: u8 = 0b010_00_101;
    pub const POINTS_NONE: u8 = 0b000_00_000;

    pub const TOP_LEFT: u8 = 0b000_00_001;
    pub const TOP_MIDDLE: u8 = 0b000_00_010;
    pub const TOP_RIGHT: u8 = 0b000_00_100;
    pub const MIDDLE_LEFT: u8 = 0b000_01_000;
    pub const MIDDLE_RIGHT: u8 = 0b000_10_000;
    pub const BOTTOM_LEFT: u8 = 0b001_00_000;
    pub const BOTTOM_MIDDLE: u8 = 0b010_00_000;
    pub const BOTTOM_RIGHT: u8 = 0b100_00_000;

    /// No effect (placeholder).
    pub fn general(_owner: Color, _object: Color) -> (Ability, Ability) {
        (Ability::NONE, Ability::NONE)
    }

    /// White pieces become controlled by the wizard's player.
    pub fn wizard(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::White) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            (Color::Black, Color::White) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Enemy pieces become also controlled by the traitor's player.
    pub fn traitor(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Black, Color::Red) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
            (Color::Red, Color::Black) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Allied pieces become also controlled by the opponent.
    pub fn spy(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::Red) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
            (Color::Black, Color::Black) => {
                (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED)
            },
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Allies gain ANY_DISTANCE; enemies lose it.
    pub fn rook(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::ANY_DISTANCE;
        let update = if owner == object { Ability::ANY_DISTANCE } else { Ability::NONE };
        (mask, update)
    }

    /// Allies gain DIRECTION_CROSS; enemies lose it.
    pub fn pawn(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::DIRECTION_CROSS;
        let update = if owner == object { Ability::DIRECTION_CROSS } else { Ability::NONE };
        (mask, update)
    }

    /// Allies gain DIRECTION_DIAGONAL; enemies lose it.
    pub fn dog(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::DIRECTION_DIAGONAL;
        let update = if owner == object { Ability::DIRECTION_DIAGONAL } else { Ability::NONE };
        (mask, update)
    }

    /// Allies gain DIRECTION_SHAPE_L; enemies lose it.
    pub fn horse(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::DIRECTION_SHAPE_L;
        let update = if owner == object { Ability::DIRECTION_SHAPE_L } else { Ability::NONE };
        (mask, update)
    }

    /// Allies gain both push abilities; enemies lose both.
    pub fn river(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::PUSH_ALLY | Ability::PUSH_ENEMY;
        let update =
            if owner == object { Ability::PUSH_ALLY | Ability::PUSH_ENEMY } else { Ability::NONE };
        (mask, update)
    }

    /// Takes over both pushed-by abilities: allies become pushable by
    /// allies only, enemies pushable by the mountain's side only.
    pub fn mountain(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::PUSHED_BY_ALLY | Ability::PUSHED_BY_ENEMY;
        let update =
            if owner == object { Ability::PUSHED_BY_ALLY } else { Ability::PUSHED_BY_ENEMY };
        (mask, update)
    }

    /// Allies gain both pass abilities; enemies lose both.
    pub fn wind(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::PASS_ALLY | Ability::PASS_ENEMY;
        let update =
            if owner == object { Ability::PASS_ALLY | Ability::PASS_ENEMY } else { Ability::NONE };
        (mask, update)
    }

    /// Takes over both passed-by abilities: allies become passable by
    /// allies only, enemies passable by the forest's side only.
    pub fn forest(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::PASSED_BY_ALLY | Ability::PASSED_BY_ENEMY;
        let update =
            if owner == object { Ability::PASSED_BY_ALLY } else { Ability::PASSED_BY_ENEMY };
        (mask, update)
    }

    /// Allies gain CAPTURE; enemies lose it.
    pub fn spear(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::CAPTURE;
        let update = if owner == object { Ability::CAPTURE } else { Ability::NONE };
        (mask, update)
    }

    /// Allies become uncapturable; enemies become capturable.
    pub fn shield(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::CAPTURED;
        let update = if owner == object { Ability::NONE } else { Ability::CAPTURED };
        (mask, update)
    }

    /// Allies gain JUMP_CAPTURE; enemies lose it.
    pub fn cannon(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::JUMP_CAPTURE;
        let update = if owner == object { Ability::JUMP_CAPTURE } else { Ability::NONE };
        (mask, update)
    }

    /// Everyone in range gains both mutual-destruction abilities.
    pub fn mine(_owner: Color, _object: Color) -> (Ability, Ability) {
        let mask = Ability::CAPTURE_ON_CAPTURED | Ability::CAPTURED_ON_CAPTURE;
        let update = Ability::CAPTURE_ON_CAPTURED | Ability::CAPTURED_ON_CAPTURE;
        (mask, update)
    }

    /// No effect (white pieces cover no points anyway).
    pub fn white(_owner: Color, _object: Color) -> (Ability, Ability) {
        (Ability::NONE, Ability::NONE)
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
}
