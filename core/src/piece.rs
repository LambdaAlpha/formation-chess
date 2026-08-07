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
    pub player: Player,
    pub formation: Formation,
    pub ability: Ability,
}

/// The stable identity of a piece: its name and player.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceId {
    pub name: char,
    pub player: Player,
}

/// One of the two players.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Player {
    Red,
    Black,
}

impl Piece {
    /// Return the stable name-and-color identity of this piece.
    pub const fn id(&self) -> PieceId {
        PieceId { name: self.name, player: self.player }
    }

    /// (controlled_by_red, controlled_by_black) for a piece of `color`.
    const fn controlled(player: Player) -> (bool, bool) {
        match player {
            Player::Red => (true, false),
            Player::Black => (false, true),
        }
    }

    /// Vertically mirror a formation for Black pieces, whose advance
    /// direction points toward the bottom of the board.
    const fn orient(player: Player, formation: Formation) -> Formation {
        match player {
            Player::Red => formation,
            Player::Black => formation.flipped(),
        }
    }

    pub const GENERAL_NAME: char = '将';

    const fn general(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '将',
            player,
            formation: Self::orient(player, Formation::GENERAL),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: true,
                draw: true,
            }
            .build(),
        }
    }

    const fn army(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '军',
            player,
            formation: Self::orient(player, Formation::ARMY),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn agent(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '间',
            player,
            formation: Self::orient(player, Formation::AGENT),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn spy(player: Player) -> Piece {
        Piece {
            name: '谍',
            player,
            formation: Self::orient(player, Formation::SPY),
            ability: AbilityConfig {
                controlled_by_red: true,
                controlled_by_black: true,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn scholar(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '士',
            player,
            formation: Self::orient(player, Formation::SCHOLAR),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn pawn(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '卒',
            player,
            formation: Self::orient(player, Formation::PAWN),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn rook(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '车',
            player,
            formation: Self::orient(player, Formation::ROOK),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn horse(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '马',
            player,
            formation: Self::orient(player, Formation::HORSE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn wind(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '风',
            player,
            formation: Self::orient(player, Formation::WIND),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn mountain(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '山',
            player,
            formation: Self::orient(player, Formation::MOUNTAIN),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: true,
                pushed_by_enemy: false,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn fire(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '火',
            player,
            formation: Self::orient(player, Formation::FIRE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: true,
                push_enemy: true,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn forest(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '林',
            player,
            formation: Self::orient(player, Formation::FOREST),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn spear(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '矛',
            player,
            formation: Self::orient(player, Formation::SPEAR),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn shield(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '盾',
            player,
            formation: Self::orient(player, Formation::SHIELD),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn shell(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '弹',
            player,
            formation: Self::orient(player, Formation::SHELL),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    const fn mine(player: Player) -> Piece {
        let (controlled_by_red, controlled_by_black) = Self::controlled(player);
        Piece {
            name: '雷',
            player,
            formation: Self::orient(player, Formation::MINE),
            ability: AbilityConfig {
                controlled_by_red,
                controlled_by_black,
                push_ally: false,
                push_enemy: false,
                pushed_by_ally: false,
                pushed_by_enemy: true,
                pull_ally: false,
                pull_enemy: false,
                pulled_by_ally: false,
                pulled_by_enemy: false,
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
                vital: false,
                draw: false,
            }
            .build(),
        }
    }

    // Canonical piece definitions, one constant per (name, color).
    pub const RED_GENERAL: Piece = Self::general(Player::Red);
    pub const BLACK_GENERAL: Piece = Self::general(Player::Black);
    pub const RED_ARMY: Piece = Self::army(Player::Red);
    pub const BLACK_ARMY: Piece = Self::army(Player::Black);
    pub const RED_AGENT: Piece = Self::agent(Player::Red);
    pub const BLACK_AGENT: Piece = Self::agent(Player::Black);
    pub const RED_SPY: Piece = Self::spy(Player::Red);
    pub const BLACK_SPY: Piece = Self::spy(Player::Black);
    pub const RED_SCHOLAR: Piece = Self::scholar(Player::Red);
    pub const BLACK_SCHOLAR: Piece = Self::scholar(Player::Black);
    pub const RED_PAWN: Piece = Self::pawn(Player::Red);
    pub const BLACK_PAWN: Piece = Self::pawn(Player::Black);
    pub const RED_ROOK: Piece = Self::rook(Player::Red);
    pub const BLACK_ROOK: Piece = Self::rook(Player::Black);
    pub const RED_HORSE: Piece = Self::horse(Player::Red);
    pub const BLACK_HORSE: Piece = Self::horse(Player::Black);
    pub const RED_WIND: Piece = Self::wind(Player::Red);
    pub const BLACK_WIND: Piece = Self::wind(Player::Black);
    pub const RED_MOUNTAIN: Piece = Self::mountain(Player::Red);
    pub const BLACK_MOUNTAIN: Piece = Self::mountain(Player::Black);
    pub const RED_FIRE: Piece = Self::fire(Player::Red);
    pub const BLACK_FIRE: Piece = Self::fire(Player::Black);
    pub const RED_FOREST: Piece = Self::forest(Player::Red);
    pub const BLACK_FOREST: Piece = Self::forest(Player::Black);
    pub const RED_SPEAR: Piece = Self::spear(Player::Red);
    pub const BLACK_SPEAR: Piece = Self::spear(Player::Black);
    pub const RED_SHIELD: Piece = Self::shield(Player::Red);
    pub const BLACK_SHIELD: Piece = Self::shield(Player::Black);
    pub const RED_SHELL: Piece = Self::shell(Player::Red);
    pub const BLACK_SHELL: Piece = Self::shell(Player::Black);
    pub const RED_MINE: Piece = Self::mine(Player::Red);
    pub const BLACK_MINE: Piece = Self::mine(Player::Black);

    /// The standard 16-piece red army, used as the default red pool.
    pub const RED_PLAYER_PIECES: [Piece; 16] = [
        Piece::RED_GENERAL,
        Piece::RED_ARMY,
        Piece::RED_AGENT,
        Piece::RED_SPY,
        Piece::RED_SCHOLAR,
        Piece::RED_PAWN,
        Piece::RED_ROOK,
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
        Piece::BLACK_ARMY,
        Piece::BLACK_AGENT,
        Piece::BLACK_SPY,
        Piece::BLACK_SCHOLAR,
        Piece::BLACK_PAWN,
        Piece::BLACK_ROOK,
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
    /// combination.
    pub fn lookup(name: char, player: Player) -> Option<Piece> {
        let piece = match (name, player) {
            ('将', Player::Red) => Piece::RED_GENERAL,
            ('将', Player::Black) => Piece::BLACK_GENERAL,
            ('军', Player::Red) => Piece::RED_ARMY,
            ('军', Player::Black) => Piece::BLACK_ARMY,
            ('间', Player::Red) => Piece::RED_AGENT,
            ('间', Player::Black) => Piece::BLACK_AGENT,
            ('谍', Player::Red) => Piece::RED_SPY,
            ('谍', Player::Black) => Piece::BLACK_SPY,
            ('士', Player::Red) => Piece::RED_SCHOLAR,
            ('士', Player::Black) => Piece::BLACK_SCHOLAR,
            ('卒', Player::Red) => Piece::RED_PAWN,
            ('卒', Player::Black) => Piece::BLACK_PAWN,
            ('车', Player::Red) => Piece::RED_ROOK,
            ('车', Player::Black) => Piece::BLACK_ROOK,
            ('马', Player::Red) => Piece::RED_HORSE,
            ('马', Player::Black) => Piece::BLACK_HORSE,
            ('风', Player::Red) => Piece::RED_WIND,
            ('风', Player::Black) => Piece::BLACK_WIND,
            ('山', Player::Red) => Piece::RED_MOUNTAIN,
            ('山', Player::Black) => Piece::BLACK_MOUNTAIN,
            ('火', Player::Red) => Piece::RED_FIRE,
            ('火', Player::Black) => Piece::BLACK_FIRE,
            ('林', Player::Red) => Piece::RED_FOREST,
            ('林', Player::Black) => Piece::BLACK_FOREST,
            ('矛', Player::Red) => Piece::RED_SPEAR,
            ('矛', Player::Black) => Piece::BLACK_SPEAR,
            ('盾', Player::Red) => Piece::RED_SHIELD,
            ('盾', Player::Black) => Piece::BLACK_SHIELD,
            ('弹', Player::Red) => Piece::RED_SHELL,
            ('弹', Player::Black) => Piece::BLACK_SHELL,
            ('雷', Player::Red) => Piece::RED_MINE,
            ('雷', Player::Black) => Piece::BLACK_MINE,
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
            let (mask, update) = (neighbor.formation.effect)(neighbor.player, self.player);
            effect_update = effect_update.masked_and(mask, update);
            effect_mask |= mask;
        }
        self.ability = self.ability.masked_set(effect_mask, effect_update);
    }

    /// Whether this piece can shove `target`: same color needs mover
    /// PUSH_ALLY **or** target PUSHED_BY_ALLY; different colors need mover
    /// PUSH_ENEMY **and** target PUSHED_BY_ENEMY.
    pub fn can_push(&self, target: Piece) -> bool {
        if self.player == target.player {
            return self.ability.has(Ability::PUSH_ALLY)
                || target.ability.has(Ability::PUSHED_BY_ALLY);
        }
        self.ability.has(Ability::PUSH_ENEMY) && target.ability.has(Ability::PUSHED_BY_ENEMY)
    }

    /// Whether this piece can pull `target`: same color needs mover
    /// PULL_ALLY **or** target PULLED_BY_ALLY; different colors need mover
    /// PULL_ENEMY **and** target PULLED_BY_ENEMY.
    pub fn can_pull(&self, target: Piece) -> bool {
        if self.player == target.player {
            return self.ability.has(Ability::PULL_ALLY)
                || target.ability.has(Ability::PULLED_BY_ALLY);
        }
        self.ability.has(Ability::PULL_ENEMY) && target.ability.has(Ability::PULLED_BY_ENEMY)
    }

    /// Whether this piece can capture `target` normally or through
    /// mutual-destruction bypass, regardless of color, and either
    /// (i) attacker has CAPTURED_ON_CAPTURE (sacrifice, bypasses target's
    /// CAPTURED), (ii) target has CAPTURE_ON_CAPTURED (retaliation, bypasses
    /// attacker's CAPTURE), or (iii) attacker has CAPTURE and target has
    /// CAPTURED. Path rules are the caller's concern.
    pub fn can_capture(&self, target: Piece) -> bool {
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

impl From<Piece> for PieceId {
    fn from(piece: Piece) -> Self {
        piece.id()
    }
}

/// Equality is **name and color only**; formation and abilities are
/// ignored. Pool lookups and board searches rely on this.
impl PartialEq for Piece {
    fn eq(&self, other: &Piece) -> bool {
        self.name == other.name && self.player == other.player
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.player, self.name)
    }
}

impl Display for PieceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.player, self.name)
    }
}

impl FromStr for PieceId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let mut indices = s.char_indices();
        let (Some(_), Some((name_start, name)), None) =
            (indices.next(), indices.next(), indices.next())
        else {
            return Err(format!("invalid piece: {s}"));
        };
        let player: Player = s[.. name_start].parse()?;
        let Some(piece) = Piece::lookup(name, player) else {
            return Err(format!("unknown piece: {s}"));
        };
        Ok(Self::from(piece))
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
        let player: Player = s[.. name_start].parse()?;
        let Some(piece) = Self::lookup(name, player) else {
            return Err(format!("unknown piece: {s}"));
        };
        Ok(piece)
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
