use std::error::Error;
use std::fmt::Display;
use std::num::NonZeroU32;

/// Version of the deterministic seed derivation used by Schedule.
pub const SEED_DERIVATION_VERSION: u32 = 1;

const FIXED_GAME_DOMAIN: u64 = 0x4649_5845_445f_4741;
const PAIR_DOMAIN: u64 = 0x5041_4952_5f42_4153;
const SCENARIO_DOMAIN: u64 = 0x5343_454e_4152_494f;
const PARTICIPANT_A_DOMAIN: u64 = 0x4147_454e_545f_415f;
const PARTICIPANT_B_DOMAIN: u64 = 0x4147_454e_545f_425f;

/// Stable identity of one participant in an arena matchup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParticipantId(String);

impl ParticipantId {
    pub fn new(value: impl Into<String>) -> Result<Self, ScheduleError> {
        let value = value.into();
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err(ScheduleError::InvalidParticipantId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ParticipantId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Two distinct participant identities whose agents may share an implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matchup {
    participant_a: ParticipantId,
    participant_b: ParticipantId,
}

impl Matchup {
    pub fn new(
        participant_a: ParticipantId, participant_b: ParticipantId,
    ) -> Result<Self, ScheduleError> {
        if participant_a == participant_b {
            return Err(ScheduleError::DuplicateParticipantId(participant_a));
        }
        Ok(Self { participant_a, participant_b })
    }

    pub fn participant_a(&self) -> &ParticipantId {
        &self.participant_a
    }

    pub fn participant_b(&self) -> &ParticipantId {
        &self.participant_b
    }
}

/// How many fixed-seat games or color-swapped pairs to generate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScheduleMode {
    /// Participant A is always Red and participant B is always Black.
    Fixed { games: NonZeroU32 },
    /// Each pair contains A-Red/B-Black followed by B-Red/A-Black.
    Paired { pairs: NonZeroU32 },
}

impl ScheduleMode {
    pub fn total_games(self) -> u64 {
        match self {
            Self::Fixed { games } => u64::from(games.get()),
            Self::Paired { pairs } => u64::from(pairs.get()) * 2,
        }
    }
}

/// One deterministic game assignment produced by a Schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePlan {
    pub game_id: u64,
    pub pair_id: Option<u64>,
    pub game_in_pair: Option<u8>,
    pub red: ParticipantId,
    pub black: ParticipantId,
    /// Shared by both games in a color-swapped pair.
    pub scenario_seed: u64,
    pub red_agent_seed: u64,
    pub black_agent_seed: u64,
}

/// Lazy deterministic iterator over game assignments.
#[derive(Debug, Clone)]
pub struct Schedule {
    matchup: Matchup,
    mode: ScheduleMode,
    root_seed: u64,
    next_game_id: u64,
}

impl Schedule {
    pub fn new(matchup: Matchup, mode: ScheduleMode, root_seed: u64) -> Self {
        Self { matchup, mode, root_seed, next_game_id: 0 }
    }

    pub fn mode(&self) -> ScheduleMode {
        self.mode
    }

    pub(crate) fn matchup(&self) -> &Matchup {
        &self.matchup
    }

    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }

    pub fn total_games(&self) -> u64 {
        self.mode.total_games()
    }

    fn fixed_plan(&self, game_id: u64) -> GamePlan {
        let game_seed = derive_seed(self.root_seed, FIXED_GAME_DOMAIN, game_id);
        GamePlan {
            game_id,
            pair_id: None,
            game_in_pair: None,
            red: self.matchup.participant_a.clone(),
            black: self.matchup.participant_b.clone(),
            scenario_seed: derive_seed(game_seed, SCENARIO_DOMAIN, 0),
            red_agent_seed: derive_seed(game_seed, PARTICIPANT_A_DOMAIN, 0),
            black_agent_seed: derive_seed(game_seed, PARTICIPANT_B_DOMAIN, 0),
        }
    }

    fn paired_plan(&self, game_id: u64) -> GamePlan {
        let pair_id = game_id / 2;
        let game_in_pair = (game_id % 2) as u8;
        let pair_seed = derive_seed(self.root_seed, PAIR_DOMAIN, pair_id);
        let scenario_seed = derive_seed(pair_seed, SCENARIO_DOMAIN, 0);
        let participant_a_seed = derive_seed(pair_seed, PARTICIPANT_A_DOMAIN, 0);
        let participant_b_seed = derive_seed(pair_seed, PARTICIPANT_B_DOMAIN, 0);

        let (red, black, red_agent_seed, black_agent_seed) = if game_in_pair == 0 {
            (
                self.matchup.participant_a.clone(),
                self.matchup.participant_b.clone(),
                participant_a_seed,
                participant_b_seed,
            )
        } else {
            (
                self.matchup.participant_b.clone(),
                self.matchup.participant_a.clone(),
                participant_b_seed,
                participant_a_seed,
            )
        };

        GamePlan {
            game_id,
            pair_id: Some(pair_id),
            game_in_pair: Some(game_in_pair),
            red,
            black,
            scenario_seed,
            red_agent_seed,
            black_agent_seed,
        }
    }
}

impl Iterator for Schedule {
    type Item = GamePlan;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_game_id >= self.total_games() {
            return None;
        }

        let game_id = self.next_game_id;
        self.next_game_id += 1;
        Some(match self.mode {
            ScheduleMode::Fixed { .. } => self.fixed_plan(game_id),
            ScheduleMode::Paired { .. } => self.paired_plan(game_id),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_games() - self.next_game_id;
        match usize::try_from(remaining) {
            Ok(remaining) => (remaining, Some(remaining)),
            Err(_) => (usize::MAX, None),
        }
    }
}

/// Invalid arena participant or matchup identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    InvalidParticipantId(String),
    DuplicateParticipantId(ParticipantId),
}

impl Display for ScheduleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParticipantId(value) => {
                write!(formatter, "invalid participant id: {value:?}")
            },
            Self::DuplicateParticipantId(value) => {
                write!(formatter, "matchup uses participant id {value} twice")
            },
        }
    }
}

impl Error for ScheduleError {}

fn derive_seed(root_seed: u64, domain: u64, index: u64) -> u64 {
    splitmix64(root_seed ^ splitmix64(domain) ^ splitmix64(index))
}

fn splitmix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
