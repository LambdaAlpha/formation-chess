use std::fmt::Debug;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Not;

/// A piece's ability set, stored as a bit per ability.
///
/// In the ability docs, "ally" and "enemy" normally compare Red and Black
/// piece colors. White pieces are neutral for formation effects: formations
/// do not modify their abilities, except that the army, agent, and spy may
/// grant control over them. Capture and push interactions with white pieces
/// are governed separately by the pieces' abilities. When several formations
/// modify the same ability bit, the updates combine with bitwise AND — any
/// formation that disables a bit wins.
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
    pub capture_on_push_blocked: bool,
    pub captured_on_push_blocked: bool,
    pub push_on_capture_unblocked: bool,
    pub pushed_on_capture_unblocked: bool,
    pub capture: bool,
    pub captured: bool,
    pub capture_on_captured: bool,
    pub captured_on_capture: bool,
    pub any_distance: bool,
    pub direction_cross: bool,
    pub direction_diagonal: bool,
    pub direction_shape_L: bool,
    pub divide: bool,
    pub vital: bool,
    pub draw: bool,
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
    /// needs no direction ability of its own.
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
    /// Push escalation (active): when this piece pushes and the push is
    /// blocked (target cannot land), the push becomes a capture —
    /// destroying the target regardless of its abilities or color.
    pub const CAPTURE_ON_PUSH_BLOCKED: Ability = Ability(1 << 6);
    /// Push escalation (passive): when this piece is pushed and the push
    /// is blocked, the pusher captures it. See
    /// [`Self::CAPTURE_ON_PUSH_BLOCKED`].
    pub const CAPTURED_ON_PUSH_BLOCKED: Ability = Ability(1 << 7);
    /// Capture demotion (active): when this piece captures without blockers
    /// on the path, the capture becomes a push — shoving the target one
    /// step farther instead of capturing it.
    pub const PUSH_ON_CAPTURE_UNBLOCKED: Ability = Ability(1 << 8);
    /// Capture demotion (passive): when this piece would be captured
    /// without blockers on the path, it is pushed instead. See
    /// [`Self::PUSH_ON_CAPTURE_UNBLOCKED`].
    pub const PUSHED_ON_CAPTURE_UNBLOCKED: Ability = Ability(1 << 9);
    /// Normal capture: move onto an enemy-occupied point and remove that
    /// piece. Requires attacker CAPTURE and target CAPTURED, and every
    /// piece on the path must not block the move. Also succeeds when
    /// the attacker has CAPTURED_ON_CAPTURE (sacrifice, ignores target's
    /// CAPTURED) or the target has CAPTURE_ON_CAPTURED (retaliation,
    /// ignores attacker's CAPTURE).
    pub const CAPTURE: Ability = Ability(1 << 10);
    /// Required for normal capture. Escalated pushes and retaliation
    /// (CAPTURE_ON_CAPTURED) bypass this bit — a blocked push destroys
    /// the target regardless of CAPTURED, and a piece with
    /// CAPTURE_ON_CAPTURED is capturable even without CAPTURED.
    pub const CAPTURED: Ability = Ability(1 << 11);
    /// Retaliation: when this piece is captured, the capturer is
    /// destroyed as well. Also makes this piece capturable even by
    /// pieces without CAPTURE — the capturer's CAPTURE requirement is
    /// bypassed.
    pub const CAPTURE_ON_CAPTURED: Ability = Ability(1 << 12);
    /// Sacrifice: when this piece captures another, it dies as well.
    /// Also allows capturing targets even without CAPTURED — the
    /// target's CAPTURED requirement is bypassed.
    pub const CAPTURED_ON_CAPTURE: Ability = Ability(1 << 13);
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
    /// Move to an empty point and leave a white piece behind at the
    /// original position (the `Divide` action, a.k.a. dividing forces).
    /// Commanding the left-behind white pieces is not part of this bit
    /// — the army's, agent's, and spy's formation effects grant that.
    pub const DIVIDE: Ability = Ability(1 << 19);
    /// A side with no vital piece left (on the board or in its pool)
    /// loses; when both sides lose theirs in the same action, the game is
    /// a draw.
    pub const VITAL: Ability = Ability(1 << 20);
    /// Can move onto an opponent's vital piece to end the game in a draw
    /// (the `Draw` action). Granted to allies by the General's formation.
    pub const DRAW: Ability = Ability(1 << 21);

    /// Whether **any** of the bits in `ability` is set. For single-bit
    /// queries this is a plain membership test; multi-bit queries are
    /// "has at least one", not "has all".
    pub fn has(&self, ability: Ability) -> bool {
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

    pub const fn add(&mut self, other: Self) {
        self.0 |= other.0;
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
        const NONE: Ability = Ability::NONE;
        let mut a = NONE;
        a.add(if self.controlled_by_red { Ability::CONTROLLED_BY_RED } else { NONE });
        a.add(if self.controlled_by_black { Ability::CONTROLLED_BY_BLACK } else { NONE });
        a.add(if self.push_ally { Ability::PUSH_ALLY } else { NONE });
        a.add(if self.push_enemy { Ability::PUSH_ENEMY } else { NONE });
        a.add(if self.pushed_by_ally { Ability::PUSHED_BY_ALLY } else { NONE });
        a.add(if self.pushed_by_enemy { Ability::PUSHED_BY_ENEMY } else { NONE });
        a.add(if self.capture_on_push_blocked { Ability::CAPTURE_ON_PUSH_BLOCKED } else { NONE });
        a.add(if self.captured_on_push_blocked { Ability::CAPTURED_ON_PUSH_BLOCKED } else { NONE });
        a.add(if self.push_on_capture_unblocked {
            Ability::PUSH_ON_CAPTURE_UNBLOCKED
        } else {
            NONE
        });
        a.add(if self.pushed_on_capture_unblocked {
            Ability::PUSHED_ON_CAPTURE_UNBLOCKED
        } else {
            NONE
        });
        a.add(if self.capture { Ability::CAPTURE } else { NONE });
        a.add(if self.captured { Ability::CAPTURED } else { NONE });
        a.add(if self.capture_on_captured { Ability::CAPTURE_ON_CAPTURED } else { NONE });
        a.add(if self.captured_on_capture { Ability::CAPTURED_ON_CAPTURE } else { NONE });
        a.add(if self.any_distance { Ability::ANY_DISTANCE } else { NONE });
        a.add(if self.direction_cross { Ability::DIRECTION_CROSS } else { NONE });
        a.add(if self.direction_diagonal { Ability::DIRECTION_DIAGONAL } else { NONE });
        a.add(if self.direction_shape_L { Ability::DIRECTION_SHAPE_L } else { NONE });
        a.add(if self.divide { Ability::DIVIDE } else { NONE });
        a.add(if self.vital { Ability::VITAL } else { NONE });
        a.add(if self.draw { Ability::DRAW } else { NONE });
        a
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
            if !self.has(*ability) {
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
    (Ability::CAPTURE_ON_PUSH_BLOCKED, "capture_on_push_blocked"),
    (Ability::CAPTURED_ON_PUSH_BLOCKED, "captured_on_push_blocked"),
    (Ability::PUSH_ON_CAPTURE_UNBLOCKED, "push_on_capture_unblocked"),
    (Ability::PUSHED_ON_CAPTURE_UNBLOCKED, "pushed_on_capture_unblocked"),
    (Ability::CAPTURE, "capture"),
    (Ability::CAPTURED, "captured"),
    (Ability::CAPTURE_ON_CAPTURED, "capture_on_captured"),
    (Ability::CAPTURED_ON_CAPTURE, "captured_on_capture"),
    (Ability::ANY_DISTANCE, "any_distance"),
    (Ability::DIRECTION_CROSS, "direction_cross"),
    (Ability::DIRECTION_DIAGONAL, "direction_diagonal"),
    (Ability::DIRECTION_SHAPE_L, "direction_shape_L"),
    (Ability::DIVIDE, "divide"),
    (Ability::VITAL, "vital"),
    (Ability::DRAW, "draw"),
];
