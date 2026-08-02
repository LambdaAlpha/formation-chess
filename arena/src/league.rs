use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::num::NonZeroU32;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::AgentFactory;
use crate::BatchError;
use crate::BatchHarness;
use crate::GameRunConfig;
use crate::MatchRunner;
use crate::Matchup;
use crate::ParticipantId;
use crate::Schedule;
use crate::ScheduleError;
use crate::ScheduleMode;
use crate::record::AgentDescriptorRecord;
use crate::record::GameRunConfigRecord;

/// File containing the complete plan for one round-robin league.
pub const LEAGUE_MANIFEST_FILE_NAME: &str = "league.json";
/// Version of the persisted league manifest schema.
pub const LEAGUE_SCHEMA_VERSION: u32 = 1;
/// Version of deterministic per-matchup seed derivation.
pub const LEAGUE_SEED_DERIVATION_VERSION: u32 = 1;

const MATCHUP_SEED_DOMAIN: u64 = 0x4c45_4147_5545_4d41;

/// One stable league identity bound to an agent factory.
pub struct RoundRobinParticipant {
    id: ParticipantId,
    factory: Box<dyn AgentFactory>,
}

impl RoundRobinParticipant {
    pub fn new(id: ParticipantId, factory: Box<dyn AgentFactory>) -> Self {
        Self { id, factory }
    }

    pub fn id(&self) -> &ParticipantId {
        &self.id
    }
}

/// Executes every unordered participant pair as a color-swapped Arena dataset.
pub struct RoundRobinLeague {
    participants: Vec<RoundRobinParticipant>,
    pairs_per_matchup: NonZeroU32,
    root_seed: u64,
    game_run_config: GameRunConfig,
}

impl RoundRobinLeague {
    pub fn new(
        participants: Vec<RoundRobinParticipant>, pairs_per_matchup: NonZeroU32, root_seed: u64,
        game_run_config: GameRunConfig,
    ) -> Result<Self, LeagueError> {
        if participants.len() < 2 {
            return Err(LeagueError::TooFewParticipants { actual: participants.len() });
        }

        let mut participant_ids = BTreeSet::new();
        for participant in &participants {
            if !participant_ids.insert(participant.id.as_str()) {
                return Err(LeagueError::DuplicateParticipantId(participant.id.clone()));
            }
        }

        Ok(Self { participants, pairs_per_matchup, root_seed, game_run_config })
    }

    pub fn participants(&self) -> &[RoundRobinParticipant] {
        &self.participants
    }

    pub fn matchup_count(&self) -> u64 {
        let participant_count = self.participants.len() as u64;
        participant_count * (participant_count - 1) / 2
    }

    pub fn total_games(&self) -> u64 {
        self.matchup_count() * u64::from(self.pairs_per_matchup.get()) * 2
    }

    pub fn manifest(&self) -> LeagueManifest {
        let participants = self
            .participants
            .iter()
            .map(|participant| LeagueParticipantRecord {
                id: participant.id.as_str().to_owned(),
                agent: AgentDescriptorRecord::from(&participant.factory.descriptor()),
            })
            .collect();
        let mut matchups = Vec::with_capacity(self.matchup_count() as usize);
        let mut matchup_index = 0_u64;
        for participant_a_index in 0 .. self.participants.len() {
            for participant_b_index in participant_a_index + 1 .. self.participants.len() {
                matchups.push(LeagueMatchupRecord {
                    matchup_index,
                    participant_a: self.participants[participant_a_index].id.as_str().to_owned(),
                    participant_b: self.participants[participant_b_index].id.as_str().to_owned(),
                    root_seed: derive_matchup_seed(self.root_seed, matchup_index),
                    dataset_directory: format!("matchup-{matchup_index:06}"),
                });
                matchup_index += 1;
            }
        }

        LeagueManifest {
            schema_version: LEAGUE_SCHEMA_VERSION,
            arena_version: crate::VERSION.to_owned(),
            core_version: formation_chess_core::VERSION.to_owned(),
            agent_version: formation_chess_agent::VERSION.to_owned(),
            seed_derivation_version: LEAGUE_SEED_DERIVATION_VERSION,
            root_seed: self.root_seed,
            pairs_per_matchup: self.pairs_per_matchup.get(),
            matchup_count: self.matchup_count(),
            total_games: self.total_games(),
            game_run_config: GameRunConfigRecord::from(self.game_run_config),
            participants,
            matchups,
        }
    }

    /// Create the league root, persist its plan, and run each matchup dataset.
    pub fn run(
        self, output_root: impl AsRef<Path>, flush_every_games: NonZeroU64,
    ) -> Result<LeagueReport, LeagueError> {
        let manifest = self.manifest();
        let output_root = output_root.as_ref().to_path_buf();
        create_league_root(&output_root, &manifest)?;

        let mut matchup_index = 0_usize;
        let mut games_written = 0_u64;
        for participant_a_index in 0 .. self.participants.len() {
            for participant_b_index in participant_a_index + 1 .. self.participants.len() {
                let participant_a = &self.participants[participant_a_index];
                let participant_b = &self.participants[participant_b_index];
                let matchup_record = &manifest.matchups[matchup_index];
                let matchup = Matchup::new(participant_a.id.clone(), participant_b.id.clone())
                    .map_err(|source| LeagueError::Schedule {
                        matchup_index: matchup_record.matchup_index,
                        source,
                    })?;
                let schedule = Schedule::new(
                    matchup.clone(),
                    ScheduleMode::Paired { pairs: self.pairs_per_matchup },
                    matchup_record.root_seed,
                );
                let runner = MatchRunner::new(
                    matchup,
                    participant_a.factory.as_ref(),
                    participant_b.factory.as_ref(),
                    self.game_run_config,
                );
                let dataset_root = output_root.join(&matchup_record.dataset_directory);
                let report = BatchHarness::new(schedule, runner)
                    .run(&dataset_root, flush_every_games)
                    .map_err(|source| LeagueError::Matchup {
                        matchup_index: matchup_record.matchup_index,
                        participant_a: matchup_record.participant_a.clone(),
                        participant_b: matchup_record.participant_b.clone(),
                        source,
                    })?;
                games_written += report.games_written;
                matchup_index += 1;
            }
        }

        Ok(LeagueReport { output_root, matchups_written: matchup_index as u64, games_written })
    }
}

/// Persisted, reproducible plan for a complete round-robin league.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeagueManifest {
    pub schema_version: u32,
    pub arena_version: String,
    pub core_version: String,
    pub agent_version: String,
    pub seed_derivation_version: u32,
    pub root_seed: u64,
    pub pairs_per_matchup: u32,
    pub matchup_count: u64,
    pub total_games: u64,
    pub game_run_config: GameRunConfigRecord,
    pub participants: Vec<LeagueParticipantRecord>,
    pub matchups: Vec<LeagueMatchupRecord>,
}

/// Participant identity and complete descriptor recorded once per league.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeagueParticipantRecord {
    pub id: String,
    pub agent: AgentDescriptorRecord,
}

/// One planned child dataset in the round-robin league.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueMatchupRecord {
    pub matchup_index: u64,
    pub participant_a: String,
    pub participant_b: String,
    pub root_seed: u64,
    pub dataset_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueReport {
    pub output_root: PathBuf,
    pub matchups_written: u64,
    pub games_written: u64,
}

#[derive(Debug)]
pub enum LeagueError {
    TooFewParticipants { actual: usize },
    DuplicateParticipantId(ParticipantId),
    OutputExists(PathBuf),
    Schedule { matchup_index: u64, source: ScheduleError },
    Matchup { matchup_index: u64, participant_a: String, participant_b: String, source: BatchError },
    Json(serde_json::Error),
    Io(io::Error),
}

impl Display for LeagueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewParticipants { actual } => {
                write!(
                    formatter,
                    "round-robin league requires at least 2 participants, got {actual}"
                )
            },
            Self::DuplicateParticipantId(participant) => {
                write!(formatter, "round-robin league uses participant id {participant} twice")
            },
            Self::OutputExists(path) => {
                write!(formatter, "league output already exists: {}", path.display())
            },
            Self::Schedule { matchup_index, source } => {
                write!(formatter, "league matchup {matchup_index} is invalid: {source}")
            },
            Self::Matchup { matchup_index, participant_a, participant_b, source } => write!(
                formatter,
                "league matchup {matchup_index} ({participant_a} vs {participant_b}) failed: {source}"
            ),
            Self::Json(error) => write!(formatter, "league JSON processing failed: {error}"),
            Self::Io(error) => write!(formatter, "league I/O failed: {error}"),
        }
    }
}

impl Error for LeagueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Schedule { source, .. } => Some(source),
            Self::Matchup { source, .. } => Some(source),
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::TooFewParticipants { .. }
            | Self::DuplicateParticipantId(_)
            | Self::OutputExists(_) => None,
        }
    }
}

impl From<serde_json::Error> for LeagueError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<io::Error> for LeagueError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn create_league_root(root: &Path, manifest: &LeagueManifest) -> Result<(), LeagueError> {
    let manifest_json = serde_json::to_vec_pretty(manifest)?;
    if let Some(parent) = root.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    match fs::create_dir(root) {
        Ok(()) => {},
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(LeagueError::OutputExists(root.to_path_buf()));
        },
        Err(error) => return Err(LeagueError::Io(error)),
    }

    let mut manifest_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(LEAGUE_MANIFEST_FILE_NAME))?;
    manifest_file.write_all(&manifest_json)?;
    manifest_file.write_all(b"\n")?;
    manifest_file.flush()?;
    Ok(())
}

fn derive_matchup_seed(root_seed: u64, matchup_index: u64) -> u64 {
    splitmix64(root_seed ^ splitmix64(MATCHUP_SEED_DOMAIN) ^ splitmix64(matchup_index))
}

fn splitmix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
