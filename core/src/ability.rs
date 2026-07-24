use std::fmt::Debug;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Not;

/// A piece's ability set, stored as a bit per ability.
///
/// In the ability docs, "ally" and "enemy" compare the colors of the two
/// pieces involved: same color = ally, different color = enemy. White is a
/// color of its own, so white pieces interact with both sides under the
/// enemy rules. When several formations modify the same ability bit, the
/// updates combine with bitwise AND — any formation that disables a bit
/// wins.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Ability(Bits);

type Bits = u32;

/// Builder for [`Ability`] with one named boolean per ability bit.
// Force explicit specification of every ability field. No defaults, no omissions.
// added to struct level due to https://github.com/rust-lang/rust/issues/159323
#[expect(non_snake_case)]
#[derive(Copy, Clone)]
pub struct AbilityConfig {
    pub controlled_by_red: bool,
    pub controlled_by_black: bool,
    pub push_ally: bool,
    pub push_enemy: bool,
    pub pushed_by_ally: bool,
    pub pushed_by_enemy: bool,
    pub pass_ally: bool,
    pub pass_enemy: bool,
    pub passed_by_ally: bool,
    pub passed_by_enemy: bool,
    pub capture: bool,
    pub captured: bool,
    pub capture_on_captured: bool,
    pub captured_on_capture: bool,
    pub jump_capture: bool,
    pub any_distance: bool,
    pub direction_cross: bool,
    pub direction_diagonal: bool,
    pub direction_shape_L: bool,
    pub control_white: bool,
    pub vital: bool,
}

impl Ability {
    /// The empty ability set.
    pub const NONE: Ability = Ability(0);
    /// The red player may command (move) this piece.
    pub const CONTROLLED_BY_RED: Ability = Ability(1 << 0);
    /// The black player may command (move) this piece.
    pub const CONTROLLED_BY_BLACK: Ability = Ability(1 << 1);
    /// Push: land on a target and shove it one step farther along the
    /// movement direction — for L-shaped moves, to the next knight point
    /// on the same line: a horse at (0,0) pushing the target on (1,2)
    /// sends it to (2,4). The shove is external force, so the target
    /// needs no direction ability of its own. If the target cannot make
    /// that step — landing occupied or off the board, or its own path
    /// there blocked — the push escalates into a capture, destroying
    /// the target regardless of its abilities or color. Unlike normal
    /// capture, escalated push works against friendly pieces too.
    ///
    /// Pushing an ally requires mover PUSH_ALLY **or** target
    /// PUSHED_BY_ALLY; either side's consent suffices.
    pub const PUSH_ALLY: Ability = Ability(1 << 2);
    /// Pushing an enemy requires mover PUSH_ENEMY **and** target
    /// PUSHED_BY_ENEMY; both must agree. See [`Self::PUSH_ALLY`] for how
    /// pushing works.
    pub const PUSH_ENEMY: Ability = Ability(1 << 3);
    /// Can be shoved by allies; see [`Self::PUSH_ALLY`].
    pub const PUSHED_BY_ALLY: Ability = Ability(1 << 4);
    /// Can be shoved by enemies; see [`Self::PUSH_ENEMY`].
    pub const PUSHED_BY_ENEMY: Ability = Ability(1 << 5);
    /// Pass: step through an occupied point on the path as if it were
    /// empty — stopping on it is still impossible. For L-shaped moves
    /// this clears the leg-blocking point.
    ///
    /// Passing an ally requires mover PASS_ALLY **or** blocker
    /// PASSED_BY_ALLY; either side's consent suffices.
    pub const PASS_ALLY: Ability = Ability(1 << 6);
    /// Passing an enemy requires mover PASS_ENEMY **and** blocker
    /// PASSED_BY_ENEMY; both must agree. See [`Self::PASS_ALLY`] for how
    /// passing works.
    pub const PASS_ENEMY: Ability = Ability(1 << 7);
    /// Can be passed through by allies; see [`Self::PASS_ALLY`].
    pub const PASSED_BY_ALLY: Ability = Ability(1 << 8);
    /// Can be passed through by enemies; see [`Self::PASS_ENEMY`].
    pub const PASSED_BY_ENEMY: Ability = Ability(1 << 9);
    /// Normal capture: move onto an enemy-occupied point and remove that
    /// piece. Requires attacker CAPTURE and target CAPTURED, and every
    /// piece on the path must be passable.
    pub const CAPTURE: Ability = Ability(1 << 10);
    /// Required for normal capture and jump capture. Escalated push
    /// captures bypass both this ability and the color restriction —
    /// a blocked push destroys the target regardless of CAPTURED or
    /// whether it is friend or foe.
    pub const CAPTURED: Ability = Ability(1 << 11);
    /// Retaliation: when this piece is captured, the capturer is
    /// destroyed as well.
    pub const CAPTURE_ON_CAPTURED: Ability = Ability(1 << 12);
    /// Sacrifice: when this piece captures, it is destroyed as well.
    pub const CAPTURED_ON_CAPTURE: Ability = Ability(1 << 13);
    /// Jump capture: capture an enemy over exactly one screen piece on
    /// the path; whether the screen is passable is irrelevant.
    /// Independent of CAPTURE — a piece with JUMP_CAPTURE alone can still
    /// jump-capture. Works along L-shaped paths too, with the leg blocker
    /// as the screen.
    pub const JUMP_CAPTURE: Ability = Ability(1 << 14);
    /// Slide any number of steps along one allowed direction instead of a
    /// single step. For L-shaped moves this chains knight moves along the
    /// same line: (0,0) → (1,2) → (2,4) → (3,6).
    pub const ANY_DISTANCE: Ability = Ability(1 << 15);
    /// Move in cross (horizontal/vertical) directions.
    pub const DIRECTION_CROSS: Ability = Ability(1 << 16);
    /// Move in diagonal directions.
    pub const DIRECTION_DIAGONAL: Ability = Ability(1 << 17);
    /// Move in L-shape (knight, 日) directions.
    pub const DIRECTION_SHAPE_L: Ability = Ability(1 << 18);
    /// Place white pieces from the shared pool onto empty points covered
    /// by this piece's formation. Commanding the placed white pieces is
    /// not part of this bit — the wizard's formation effect grants that.
    pub const CONTROL_WHITE: Ability = Ability(1 << 19);
    /// A side with no vital piece left (on the board or in its pool)
    /// loses; when both sides lose theirs in the same action, the game is
    /// a draw. Two vital pieces standing inside each other's formation
    /// pattern also end the game in a draw.
    pub const VITAL: Ability = Ability(1 << 20);

    /// Whether **any** of the bits in `ability` is set. For single-bit
    /// queries this is a plain membership test; multi-bit queries are
    /// "has at least one", not "has all".
    pub fn has_ability(&self, ability: Ability) -> bool {
        self.0 & ability.0 != 0
    }

    /// AND `other` into the bits selected by `mask`; bits outside `mask`
    /// are unchanged.
    pub fn masked_and(self, mask: Self, other: Self) -> Self {
        (self & !mask) | (other & self & mask)
    }

    /// OR `other` into the bits selected by `mask`; bits outside `mask`
    /// are unchanged.
    pub fn masked_or(self, mask: Self, other: Self) -> Self {
        (self & !mask) | ((other | self) & mask)
    }

    /// Flip the bits selected by `mask`; bits outside `mask` are unchanged.
    pub fn masked_not(self, mask: Self) -> Self {
        (self & !mask) | (!self & mask)
    }

    /// Replace the bits selected by `mask` with the corresponding bits of
    /// `new`; bits outside `mask` are unchanged.
    pub fn masked_set(self, mask: Self, new: Self) -> Self {
        (self & !mask) | (new & mask)
    }

    /// Bitwise AND; const counterpart of the `&` operator.
    pub const fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Bitwise OR; const counterpart of the `|` operator.
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Bitwise XOR; const counterpart of the `^` operator.
    pub const fn xor(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    /// Bitwise NOT; const counterpart of the `!` operator.
    pub const fn negate(self) -> Self {
        Self(!self.0)
    }
}

impl AbilityConfig {
    /// Collect the boolean fields into an [`Ability`] bit set.
    pub const fn build(self) -> Ability {
        Ability::NONE
            .or(if self.controlled_by_red { Ability::CONTROLLED_BY_RED } else { Ability::NONE })
            .or(if self.controlled_by_black { Ability::CONTROLLED_BY_BLACK } else { Ability::NONE })
            .or(if self.push_ally { Ability::PUSH_ALLY } else { Ability::NONE })
            .or(if self.push_enemy { Ability::PUSH_ENEMY } else { Ability::NONE })
            .or(if self.pushed_by_ally { Ability::PUSHED_BY_ALLY } else { Ability::NONE })
            .or(if self.pushed_by_enemy { Ability::PUSHED_BY_ENEMY } else { Ability::NONE })
            .or(if self.pass_ally { Ability::PASS_ALLY } else { Ability::NONE })
            .or(if self.pass_enemy { Ability::PASS_ENEMY } else { Ability::NONE })
            .or(if self.passed_by_ally { Ability::PASSED_BY_ALLY } else { Ability::NONE })
            .or(if self.passed_by_enemy { Ability::PASSED_BY_ENEMY } else { Ability::NONE })
            .or(if self.capture { Ability::CAPTURE } else { Ability::NONE })
            .or(if self.captured { Ability::CAPTURED } else { Ability::NONE })
            .or(if self.capture_on_captured { Ability::CAPTURE_ON_CAPTURED } else { Ability::NONE })
            .or(if self.captured_on_capture { Ability::CAPTURED_ON_CAPTURE } else { Ability::NONE })
            .or(if self.jump_capture { Ability::JUMP_CAPTURE } else { Ability::NONE })
            .or(if self.any_distance { Ability::ANY_DISTANCE } else { Ability::NONE })
            .or(if self.direction_cross { Ability::DIRECTION_CROSS } else { Ability::NONE })
            .or(if self.direction_diagonal { Ability::DIRECTION_DIAGONAL } else { Ability::NONE })
            .or(if self.direction_shape_L { Ability::DIRECTION_SHAPE_L } else { Ability::NONE })
            .or(if self.control_white { Ability::CONTROL_WHITE } else { Ability::NONE })
            .or(if self.vital { Ability::VITAL } else { Ability::NONE })
    }
}

impl From<AbilityConfig> for Ability {
    fn from(config: AbilityConfig) -> Ability {
        config.build()
    }
}

impl Not for Ability {
    type Output = Ability;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitAnd for Ability {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Ability {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Ability {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Ability {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Debug for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        for (i, (ability, name)) in ABILITIES.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            if !self.has_ability(*ability) {
                f.write_str("-")?;
            }
            f.write_str(name)?;
        }
        f.write_str("}")
    }
}

const ABILITIES: &[(Ability, &str)] = &[
    (Ability::CONTROLLED_BY_RED, "controlled_by_red"),
    (Ability::CONTROLLED_BY_BLACK, "controlled_by_black"),
    (Ability::PUSH_ALLY, "push_ally"),
    (Ability::PUSH_ENEMY, "push_enemy"),
    (Ability::PUSHED_BY_ALLY, "pushed_by_ally"),
    (Ability::PUSHED_BY_ENEMY, "pushed_by_enemy"),
    (Ability::PASS_ALLY, "pass_ally"),
    (Ability::PASS_ENEMY, "pass_enemy"),
    (Ability::PASSED_BY_ALLY, "passed_by_ally"),
    (Ability::PASSED_BY_ENEMY, "passed_by_enemy"),
    (Ability::CAPTURE, "capture"),
    (Ability::CAPTURED, "captured"),
    (Ability::CAPTURE_ON_CAPTURED, "capture_on_captured"),
    (Ability::CAPTURED_ON_CAPTURE, "captured_on_capture"),
    (Ability::JUMP_CAPTURE, "jump_capture"),
    (Ability::ANY_DISTANCE, "any_distance"),
    (Ability::DIRECTION_CROSS, "direction_cross"),
    (Ability::DIRECTION_DIAGONAL, "direction_diagonal"),
    (Ability::DIRECTION_SHAPE_L, "direction_shape_L"),
    (Ability::CONTROL_WHITE, "control_white"),
    (Ability::VITAL, "vital"),
];
