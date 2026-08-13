use std::fmt::Debug;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Not;

/// A piece's ability set, stored as a bit per ability.
///
/// In the ability docs, "ally" and "enemy" compare Red and Black piece
/// colors. When several formations modify the same ability bit, the updates
/// combine with bitwise AND — any formation that disables a bit wins.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Ability(Bits);

type Bits = u32;

/// Builder for [`Ability`] with one named boolean per ability bit.
// Force explicit specification of every ability field. No defaults, no omissions.
#[derive(Copy, Clone)]
pub struct AbilityConfig {
    pub initiative: bool,
    pub passivity: bool,
    pub push_friend: bool,
    pub push_enemy: bool,
    pub friend_push: bool,
    pub enemy_push: bool,
    pub pull_friend: bool,
    pub pull_enemy: bool,
    pub friend_pull: bool,
    pub enemy_pull: bool,
    pub hidden_capture: bool,
    pub easy_capture: bool,
    pub overt_capture: bool,
    pub hard_capture: bool,
    pub capture: bool,
    pub capturable: bool,
    pub counter_capture: bool,
    pub force_capture: bool,
    pub swift_move: bool,
    pub orthogonal_move: bool,
    pub diagonal_move: bool,
    pub broad_step: bool,
    pub leader: bool,
    pub peace_talk: bool,
}

impl Ability {
    /// The empty ability set.
    pub const NONE: Ability = Ability(0);
    /// Initiative: the piece's owner can control it.
    pub const INITIATIVE: Ability = Ability(1 << 0);
    /// Passivity: the piece's enemy can control it.
    pub const PASSIVITY: Ability = Ability(1 << 1);
    /// Push Ally: actively push an allied piece.
    pub const PUSH_FRIEND: Ability = Ability(1 << 2);
    /// Push Enemy: actively push an enemy piece.
    pub const PUSH_ENEMY: Ability = Ability(1 << 3);
    /// Ally Push: be pushed by an allied piece.
    pub const FRIEND_PUSH: Ability = Ability(1 << 4);
    /// Enemy Push: be pushed by an enemy piece.
    pub const ENEMY_PUSH: Ability = Ability(1 << 5);
    /// Pull Ally: actively pull an allied piece.
    pub const PULL_FRIEND: Ability = Ability(1 << 6);
    /// Pull Enemy: actively pull an enemy piece.
    pub const PULL_ENEMY: Ability = Ability(1 << 7);
    /// Ally Pull: be pulled by an allied piece.
    pub const FRIEND_PULL: Ability = Ability(1 << 8);
    /// Enemy Pull: be pulled by an enemy piece.
    pub const ENEMY_PULL: Ability = Ability(1 << 9);
    /// Hidden Capture: a blocked push captures the target.
    pub const HIDDEN_CAPTURE: Ability = Ability(1 << 10);
    /// Easy Capture: when this piece cannot retreat from a push, it captures the pusher.
    pub const EASY_CAPTURE: Ability = Ability(1 << 11);
    /// Overt Capture: a capture pushes the target when it has a valid landing.
    pub const OVERT_CAPTURE: Ability = Ability(1 << 12);
    /// Hard Capture: when this piece would be captured but has a valid landing, it is pushed instead.
    pub const HARD_CAPTURE: Ability = Ability(1 << 13);
    /// Capture: actively initiate a normal capture.
    pub const CAPTURE: Ability = Ability(1 << 14);
    /// Capturable: be the target of a normal capture.
    pub const CAPTURABLE: Ability = Ability(1 << 15);
    /// Counter Capture: remove the capturer too and bypass its Capture requirement.
    pub const COUNTER_CAPTURE: Ability = Ability(1 << 16);
    /// Force Capture: capture regardless of the target's Capturable ability, removing the attacker too.
    pub const FORCE_CAPTURE: Ability = Ability(1 << 17);
    /// Swift Move: repeat the same movement direction for any number of steps.
    pub const SWIFT_MOVE: Ability = Ability(1 << 18);
    /// Orthogonal Move: move horizontally or vertically.
    pub const ORTHOGONAL_MOVE: Ability = Ability(1 << 19);
    /// Diagonal Move: move along a diagonal.
    pub const DIAGONAL_MOVE: Ability = Ability(1 << 20);
    /// Broad Step: move by a knight step.
    pub const BROAD_STEP: Ability = Ability(1 << 21);
    /// Leader: the owner loses when it has no remaining Leader pieces.
    pub const LEADER: Ability = Ability(1 << 22);
    /// Peace Talk: exchange positions with an enemy Leader and immediately draw.
    pub const PEACE_TALK: Ability = Ability(1 << 23);
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
        a.add(if self.initiative { Ability::INITIATIVE } else { NONE });
        a.add(if self.passivity { Ability::PASSIVITY } else { NONE });
        a.add(if self.push_friend { Ability::PUSH_FRIEND } else { NONE });
        a.add(if self.push_enemy { Ability::PUSH_ENEMY } else { NONE });
        a.add(if self.friend_push { Ability::FRIEND_PUSH } else { NONE });
        a.add(if self.enemy_push { Ability::ENEMY_PUSH } else { NONE });
        a.add(if self.pull_friend { Ability::PULL_FRIEND } else { NONE });
        a.add(if self.pull_enemy { Ability::PULL_ENEMY } else { NONE });
        a.add(if self.friend_pull { Ability::FRIEND_PULL } else { NONE });
        a.add(if self.enemy_pull { Ability::ENEMY_PULL } else { NONE });
        a.add(if self.hidden_capture { Ability::HIDDEN_CAPTURE } else { NONE });
        a.add(if self.easy_capture { Ability::EASY_CAPTURE } else { NONE });
        a.add(if self.overt_capture { Ability::OVERT_CAPTURE } else { NONE });
        a.add(if self.hard_capture { Ability::HARD_CAPTURE } else { NONE });
        a.add(if self.capture { Ability::CAPTURE } else { NONE });
        a.add(if self.capturable { Ability::CAPTURABLE } else { NONE });
        a.add(if self.counter_capture { Ability::COUNTER_CAPTURE } else { NONE });
        a.add(if self.force_capture { Ability::FORCE_CAPTURE } else { NONE });
        a.add(if self.swift_move { Ability::SWIFT_MOVE } else { NONE });
        a.add(if self.orthogonal_move { Ability::ORTHOGONAL_MOVE } else { NONE });
        a.add(if self.diagonal_move { Ability::DIAGONAL_MOVE } else { NONE });
        a.add(if self.broad_step { Ability::BROAD_STEP } else { NONE });
        a.add(if self.leader { Ability::LEADER } else { NONE });
        a.add(if self.peace_talk { Ability::PEACE_TALK } else { NONE });
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
    (Ability::INITIATIVE, "主动"),
    (Ability::PASSIVITY, "被动"),
    (Ability::PUSH_FRIEND, "推友"),
    (Ability::PUSH_ENEMY, "推敌"),
    (Ability::FRIEND_PUSH, "友推"),
    (Ability::ENEMY_PUSH, "敌推"),
    (Ability::PULL_FRIEND, "拉友"),
    (Ability::PULL_ENEMY, "拉敌"),
    (Ability::FRIEND_PULL, "友拉"),
    (Ability::ENEMY_PULL, "敌拉"),
    (Ability::HIDDEN_CAPTURE, "暗捉"),
    (Ability::EASY_CAPTURE, "易捉"),
    (Ability::OVERT_CAPTURE, "明捉"),
    (Ability::HARD_CAPTURE, "难捉"),
    (Ability::CAPTURE, "捕捉"),
    (Ability::CAPTURABLE, "被捉"),
    (Ability::COUNTER_CAPTURE, "反捉"),
    (Ability::FORCE_CAPTURE, "强捉"),
    (Ability::SWIFT_MOVE, "疾行"),
    (Ability::ORTHOGONAL_MOVE, "纵横"),
    (Ability::DIAGONAL_MOVE, "交错"),
    (Ability::BROAD_STEP, "阔步"),
    (Ability::LEADER, "首领"),
    (Ability::PEACE_TALK, "议和"),
];
