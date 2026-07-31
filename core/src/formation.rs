#![expect(clippy::unusual_byte_groupings)]

use crate::ability::Ability;
use crate::piece::Color;

/// A piece's formation: which of the eight surrounding points its aura
/// covers, and how it modifies the abilities of pieces standing there.
/// White pieces are neutral and ignore formation effects except for the
/// control granted by the army, agent, and spy.
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
    pub const ARMY: Self = Self { points: Self::CORNER, effect: Self::army };
    pub const AGENT: Self = Self { points: Self::CORNER, effect: Self::agent };
    pub const SPY: Self = Self { points: Self::CORNER, effect: Self::spy };
    pub const SCHOLAR: Self = Self { points: Self::MIDDLE, effect: Self::scholar };
    pub const PAWN: Self = Self { points: Self::MIDDLE, effect: Self::pawn };
    pub const ROOK: Self = Self { points: Self::MIDDLE, effect: Self::rook };
    pub const HORSE: Self = Self { points: Self::MIDDLE, effect: Self::horse };
    pub const WIND: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::wind };
    pub const MOUNTAIN: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::mountain };
    pub const FIRE: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::fire };
    pub const FOREST: Self = Self { points: Self::UPPER_TRIANGLE, effect: Self::forest };
    pub const SPEAR: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::spear };
    pub const SHIELD: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::shield };
    pub const SHELL: Self = Self { points: Self::LOWER_TRIANGLE, effect: Self::shell };
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

    /// Grant `mask` to colored allies and strip it from colored enemies.
    /// White pieces are neutral and remain unchanged.
    fn grant_allies_strip_enemies(
        owner: Color, object: Color, mask: Ability,
    ) -> (Ability, Ability) {
        match object {
            Color::White => (Ability::NONE, Ability::NONE),
            _ if owner == object => (mask, mask),
            _ => (mask, Ability::NONE),
        }
    }

    /// Allies gain DRAW; enemies lose it.
    /// White pieces become controlled by the general's player;
    pub fn general(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::White) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            (Color::Red, Color::Red) => (Ability::DRAW, Ability::DRAW),
            (Color::Red, Color::Black) => (Ability::DRAW, Ability::NONE),
            (Color::Black, Color::White) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
            (Color::Black, Color::Red) => (Ability::DRAW, Ability::NONE),
            (Color::Black, Color::Black) => (Ability::DRAW, Ability::DRAW),
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// White pieces become controlled by the army's player;
    /// same-color allies gain DIVIDE; different-color enemies lose it.
    pub fn army(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::White) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            (Color::Red, Color::Red) => (Ability::DIVIDE, Ability::DIVIDE),
            (Color::Red, Color::Black) => (Ability::DIVIDE, Ability::NONE),
            (Color::Black, Color::White) => {
                (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK)
            },
            (Color::Black, Color::Red) => (Ability::DIVIDE, Ability::NONE),
            (Color::Black, Color::Black) => (Ability::DIVIDE, Ability::DIVIDE),
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Enemy pieces become also controlled by the agent's player;
    /// allies have the opponent's control disabled (purges foreign
    /// control from the agent's own side).
    pub fn agent(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::Red) => (Ability::CONTROLLED_BY_BLACK, Ability::NONE),
            (Color::Red, _) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            (Color::Black, Color::Black) => (Ability::CONTROLLED_BY_RED, Ability::NONE),
            (Color::Black, _) => (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK),
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Allied pieces become also controlled by the opponent;
    /// enemy pieces have the spy player's control disabled (the spy
    /// strips its own side's control from enemies).
    pub fn spy(owner: Color, object: Color) -> (Ability, Ability) {
        match (owner, object) {
            (Color::Red, Color::Black) => (Ability::CONTROLLED_BY_RED, Ability::NONE),
            (Color::Red, _) => (Ability::CONTROLLED_BY_BLACK, Ability::CONTROLLED_BY_BLACK),
            (Color::Black, Color::Red) => (Ability::CONTROLLED_BY_BLACK, Ability::NONE),
            (Color::Black, _) => (Ability::CONTROLLED_BY_RED, Ability::CONTROLLED_BY_RED),
            _ => (Ability::NONE, Ability::NONE),
        }
    }

    /// Allies gain DIRECTION_DIAGONAL; enemies lose it.
    pub fn scholar(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_DIAGONAL)
    }

    /// Allies gain DIRECTION_CROSS; enemies lose it.
    pub fn pawn(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_CROSS)
    }

    /// Allies gain ANY_DISTANCE; enemies lose it.
    pub fn rook(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::ANY_DISTANCE)
    }

    /// Allies gain DIRECTION_SHAPE_L; enemies lose it.
    pub fn horse(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::DIRECTION_SHAPE_L)
    }

    /// Allies gain both push abilities; enemies lose both.
    pub fn wind(owner: Color, object: Color) -> (Ability, Ability) {
        let mask = Ability::PUSH_ALLY | Ability::PUSH_ENEMY;
        Self::grant_allies_strip_enemies(owner, object, mask)
    }

    /// Takes over both pushed-by abilities: allies become pushable by
    /// allies only, enemies pushable by the mountain's side only.
    pub fn mountain(owner: Color, object: Color) -> (Ability, Ability) {
        if object == Color::White {
            return (Ability::NONE, Ability::NONE);
        }
        let mask = Ability::PUSHED_BY_ALLY | Ability::PUSHED_BY_ENEMY;
        let update =
            if owner == object { Ability::PUSHED_BY_ALLY } else { Ability::PUSHED_BY_ENEMY };
        (mask, update)
    }

    /// Allies gain active push escalation and lose passive; enemies gain
    /// passive and lose active.
    pub fn fire(owner: Color, object: Color) -> (Ability, Ability) {
        if object == Color::White {
            return (Ability::NONE, Ability::NONE);
        }
        let mask = Ability::CAPTURE_ON_PUSH_BLOCKED | Ability::CAPTURED_ON_PUSH_BLOCKED;
        let update = if owner == object {
            Ability::CAPTURE_ON_PUSH_BLOCKED
        } else {
            Ability::CAPTURED_ON_PUSH_BLOCKED
        };
        (mask, update)
    }

    /// Allies gain passive capture demotion and lose active; enemies gain
    /// active and lose passive.
    pub fn forest(owner: Color, object: Color) -> (Ability, Ability) {
        if object == Color::White {
            return (Ability::NONE, Ability::NONE);
        }
        let mask = Ability::PUSH_ON_CAPTURE_UNBLOCKED | Ability::PUSHED_ON_CAPTURE_UNBLOCKED;
        let update = if owner == object {
            Ability::PUSHED_ON_CAPTURE_UNBLOCKED
        } else {
            Ability::PUSH_ON_CAPTURE_UNBLOCKED
        };
        (mask, update)
    }

    /// Allies gain CAPTURE; enemies lose it.
    pub fn spear(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURE)
    }

    /// Allies become uncapturable; enemies become capturable.
    pub fn shield(owner: Color, object: Color) -> (Ability, Ability) {
        if object == Color::White {
            return (Ability::NONE, Ability::NONE);
        }
        let mask = Ability::CAPTURED;
        let update = if owner == object { Ability::NONE } else { Ability::CAPTURED };
        (mask, update)
    }

    /// Allies gain CAPTURED_ON_CAPTURE; enemies lose it.
    pub fn shell(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURED_ON_CAPTURE)
    }

    /// Allies gain CAPTURE_ON_CAPTURED; enemies lose it.
    pub fn mine(owner: Color, object: Color) -> (Ability, Ability) {
        Self::grant_allies_strip_enemies(owner, object, Ability::CAPTURE_ON_CAPTURED)
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
