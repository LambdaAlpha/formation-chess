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
        Piece {
            name: '将',
            player,
            formation: Self::orient(player, Formation::GENERAL),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: false,
                diagonal_move: true,
                broad_step: false,
                leader: true,
                peace_talk: true,
            }
            .build(),
        }
    }

    const fn stratagem(player: Player) -> Piece {
        Piece {
            name: '计',
            player,
            formation: Self::orient(player, Formation::STRATAGEM),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: false,
                diagonal_move: true,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn momentum(player: Player) -> Piece {
        Piece {
            name: '势',
            player,
            formation: Self::orient(player, Formation::MOMENTUM),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: true,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: false,
                diagonal_move: true,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn adaptation(player: Player) -> Piece {
        Piece {
            name: '变',
            player,
            formation: Self::orient(player, Formation::ADAPTATION),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: true,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: false,
                diagonal_move: true,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn wind(player: Player) -> Piece {
        Piece {
            name: '风',
            player,
            formation: Self::orient(player, Formation::WIND),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn forest(player: Player) -> Piece {
        Piece {
            name: '林',
            player,
            formation: Self::orient(player, Formation::FOREST),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: true,
                enemy_pull: false,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn fire(player: Player) -> Piece {
        Piece {
            name: '火',
            player,
            formation: Self::orient(player, Formation::FIRE),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: false,
                enemy_push: true,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn mountain(player: Player) -> Piece {
        Piece {
            name: '山',
            player,
            formation: Self::orient(player, Formation::MOUNTAIN),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: true,
                push_enemy: true,
                friend_push: true,
                enemy_push: false,
                pull_friend: true,
                pull_enemy: true,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: false,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn spear(player: Player) -> Piece {
        Piece {
            name: '矛',
            player,
            formation: Self::orient(player, Formation::SPEAR),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: false,
                broad_step: true,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn shield(player: Player) -> Piece {
        Piece {
            name: '盾',
            player,
            formation: Self::orient(player, Formation::SHIELD),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: false,
                counter_capture: false,
                force_capture: false,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: false,
                broad_step: true,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn shell(player: Player) -> Piece {
        Piece {
            name: '弹',
            player,
            formation: Self::orient(player, Formation::SHELL),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: true,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: false,
                broad_step: true,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn mine(player: Player) -> Piece {
        Piece {
            name: '雷',
            player,
            formation: Self::orient(player, Formation::MINE),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: true,
                force_capture: false,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: false,
                broad_step: true,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn scholar(player: Player) -> Piece {
        Piece {
            name: '士',
            player,
            formation: Self::orient(player, Formation::SCHOLAR),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: true,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn pawn(player: Player) -> Piece {
        Piece {
            name: '卒',
            player,
            formation: Self::orient(player, Formation::PAWN),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: false,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn horse(player: Player) -> Piece {
        Piece {
            name: '马',
            player,
            formation: Self::orient(player, Formation::HORSE),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: false,
                orthogonal_move: false,
                diagonal_move: false,
                broad_step: true,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    const fn rook(player: Player) -> Piece {
        Piece {
            name: '车',
            player,
            formation: Self::orient(player, Formation::ROOK),
            ability: AbilityConfig {
                initiative: true,
                passivity: false,
                push_friend: false,
                push_enemy: false,
                friend_push: false,
                enemy_push: true,
                pull_friend: false,
                pull_enemy: false,
                friend_pull: false,
                enemy_pull: true,
                hidden_capture: false,
                easy_capture: false,
                overt_capture: false,
                hard_capture: false,
                capture: true,
                capturable: true,
                counter_capture: false,
                force_capture: false,
                swift_move: true,
                orthogonal_move: true,
                diagonal_move: false,
                broad_step: false,
                leader: false,
                peace_talk: false,
            }
            .build(),
        }
    }

    // Canonical piece definitions, one constant per (name, color).
    pub const RED_GENERAL: Piece = Self::general(Player::Red);
    pub const BLACK_GENERAL: Piece = Self::general(Player::Black);
    pub const RED_STRATAGEM: Piece = Self::stratagem(Player::Red);
    pub const BLACK_STRATAGEM: Piece = Self::stratagem(Player::Black);
    pub const RED_MOMENTUM: Piece = Self::momentum(Player::Red);
    pub const BLACK_MOMENTUM: Piece = Self::momentum(Player::Black);
    pub const RED_ADAPTATION: Piece = Self::adaptation(Player::Red);
    pub const BLACK_ADAPTATION: Piece = Self::adaptation(Player::Black);
    pub const RED_WIND: Piece = Self::wind(Player::Red);
    pub const BLACK_WIND: Piece = Self::wind(Player::Black);
    pub const RED_FOREST: Piece = Self::forest(Player::Red);
    pub const BLACK_FOREST: Piece = Self::forest(Player::Black);
    pub const RED_FIRE: Piece = Self::fire(Player::Red);
    pub const BLACK_FIRE: Piece = Self::fire(Player::Black);
    pub const RED_MOUNTAIN: Piece = Self::mountain(Player::Red);
    pub const BLACK_MOUNTAIN: Piece = Self::mountain(Player::Black);
    pub const RED_SPEAR: Piece = Self::spear(Player::Red);
    pub const BLACK_SPEAR: Piece = Self::spear(Player::Black);
    pub const RED_SHIELD: Piece = Self::shield(Player::Red);
    pub const BLACK_SHIELD: Piece = Self::shield(Player::Black);
    pub const RED_SHELL: Piece = Self::shell(Player::Red);
    pub const BLACK_SHELL: Piece = Self::shell(Player::Black);
    pub const RED_MINE: Piece = Self::mine(Player::Red);
    pub const BLACK_MINE: Piece = Self::mine(Player::Black);
    pub const RED_SCHOLAR: Piece = Self::scholar(Player::Red);
    pub const BLACK_SCHOLAR: Piece = Self::scholar(Player::Black);
    pub const RED_PAWN: Piece = Self::pawn(Player::Red);
    pub const BLACK_PAWN: Piece = Self::pawn(Player::Black);
    pub const RED_HORSE: Piece = Self::horse(Player::Red);
    pub const BLACK_HORSE: Piece = Self::horse(Player::Black);
    pub const RED_ROOK: Piece = Self::rook(Player::Red);
    pub const BLACK_ROOK: Piece = Self::rook(Player::Black);

    /// The standard 16-piece red set, used as the default red pool.
    pub const RED_PLAYER_PIECES: [Piece; 16] = [
        Piece::RED_GENERAL,
        Piece::RED_STRATAGEM,
        Piece::RED_MOMENTUM,
        Piece::RED_ADAPTATION,
        Piece::RED_WIND,
        Piece::RED_FOREST,
        Piece::RED_FIRE,
        Piece::RED_MOUNTAIN,
        Piece::RED_SPEAR,
        Piece::RED_SHIELD,
        Piece::RED_SHELL,
        Piece::RED_MINE,
        Piece::RED_SCHOLAR,
        Piece::RED_PAWN,
        Piece::RED_HORSE,
        Piece::RED_ROOK,
    ];

    /// The standard 16-piece black set, used as the default black pool.
    pub const BLACK_PLAYER_PIECES: [Piece; 16] = [
        Piece::BLACK_GENERAL,
        Piece::BLACK_STRATAGEM,
        Piece::BLACK_MOMENTUM,
        Piece::BLACK_ADAPTATION,
        Piece::BLACK_WIND,
        Piece::BLACK_FOREST,
        Piece::BLACK_FIRE,
        Piece::BLACK_MOUNTAIN,
        Piece::BLACK_SPEAR,
        Piece::BLACK_SHIELD,
        Piece::BLACK_SHELL,
        Piece::BLACK_MINE,
        Piece::BLACK_SCHOLAR,
        Piece::BLACK_PAWN,
        Piece::BLACK_HORSE,
        Piece::BLACK_ROOK,
    ];

    /// The canonical piece for a name and color, or None for an unknown
    /// combination.
    pub fn lookup(name: char, player: Player) -> Option<Piece> {
        let piece = match (name, player) {
            ('将', Player::Red) => Piece::RED_GENERAL,
            ('将', Player::Black) => Piece::BLACK_GENERAL,
            ('计', Player::Red) => Piece::RED_STRATAGEM,
            ('计', Player::Black) => Piece::BLACK_STRATAGEM,
            ('势', Player::Red) => Piece::RED_MOMENTUM,
            ('势', Player::Black) => Piece::BLACK_MOMENTUM,
            ('变', Player::Red) => Piece::RED_ADAPTATION,
            ('变', Player::Black) => Piece::BLACK_ADAPTATION,
            ('风', Player::Red) => Piece::RED_WIND,
            ('风', Player::Black) => Piece::BLACK_WIND,
            ('林', Player::Red) => Piece::RED_FOREST,
            ('林', Player::Black) => Piece::BLACK_FOREST,
            ('火', Player::Red) => Piece::RED_FIRE,
            ('火', Player::Black) => Piece::BLACK_FIRE,
            ('山', Player::Red) => Piece::RED_MOUNTAIN,
            ('山', Player::Black) => Piece::BLACK_MOUNTAIN,
            ('矛', Player::Red) => Piece::RED_SPEAR,
            ('矛', Player::Black) => Piece::BLACK_SPEAR,
            ('盾', Player::Red) => Piece::RED_SHIELD,
            ('盾', Player::Black) => Piece::BLACK_SHIELD,
            ('弹', Player::Red) => Piece::RED_SHELL,
            ('弹', Player::Black) => Piece::BLACK_SHELL,
            ('雷', Player::Red) => Piece::RED_MINE,
            ('雷', Player::Black) => Piece::BLACK_MINE,
            ('士', Player::Red) => Piece::RED_SCHOLAR,
            ('士', Player::Black) => Piece::BLACK_SCHOLAR,
            ('卒', Player::Red) => Piece::RED_PAWN,
            ('卒', Player::Black) => Piece::BLACK_PAWN,
            ('马', Player::Red) => Piece::RED_HORSE,
            ('马', Player::Black) => Piece::BLACK_HORSE,
            ('车', Player::Red) => Piece::RED_ROOK,
            ('车', Player::Black) => Piece::BLACK_ROOK,
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
    /// PUSH_FRIEND **or** target FRIEND_PUSH; different colors need mover
    /// PUSH_ENEMY **and** target ENEMY_PUSH.
    pub fn can_push(&self, target: Piece) -> bool {
        if self.player == target.player {
            return self.ability.has(Ability::PUSH_FRIEND)
                || target.ability.has(Ability::FRIEND_PUSH);
        }
        self.ability.has(Ability::PUSH_ENEMY) && target.ability.has(Ability::ENEMY_PUSH)
    }

    /// Whether this piece can pull `target`: same color needs mover
    /// PULL_FRIEND **or** target FRIEND_PULL; different colors need mover
    /// PULL_ENEMY **and** target ENEMY_PULL.
    pub fn can_pull(&self, target: Piece) -> bool {
        if self.player == target.player {
            return self.ability.has(Ability::PULL_FRIEND)
                || target.ability.has(Ability::FRIEND_PULL);
        }
        self.ability.has(Ability::PULL_ENEMY) && target.ability.has(Ability::ENEMY_PULL)
    }

    /// Whether this piece can capture `target` normally or through
    /// mutual-destruction bypass, regardless of color, and either
    /// (i) attacker has FORCE_CAPTURE (sacrifice, bypasses target's
    /// CAPTURED), (ii) target has COUNTER_CAPTURE (retaliation, bypasses
    /// attacker's CAPTURE), or (iii) attacker has CAPTURE and target has
    /// CAPTURED. Path rules are the caller's concern.
    pub fn can_capture(&self, target: Piece) -> bool {
        if self.ability.has(Ability::FORCE_CAPTURE) || target.ability.has(Ability::COUNTER_CAPTURE)
        {
            return true;
        }
        self.ability.has(Ability::CAPTURE) && target.ability.has(Ability::CAPTURABLE)
    }

    /// Whether player may command this piece, per its INITIATIVE or PASSIVITY
    /// abilities relative to the piece's owning player.
    pub fn can_controlled_by(&self, player: Player) -> bool {
        if player == self.player {
            return self.ability.has(Ability::INITIATIVE);
        }
        self.ability.has(Ability::PASSIVITY)
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
