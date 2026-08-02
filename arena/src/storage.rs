use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::iter::FusedIterator;
use std::path::Path;
use std::path::PathBuf;

use crate::AgentDescriptor;
use crate::GameRun;
use crate::GameTermination;
use crate::record::AgentDescriptorRecord;
use crate::record::ArenaManifest;
use crate::record::GameRecord;
use crate::record::RECORD_SCHEMA_VERSION;
use crate::record::RecordError;
use crate::record::STATE_HASH_ALGORITHM;
use crate::record::ScheduleRecord;

pub const MANIFEST_FILE_NAME: &str = "manifest.json";
pub const GAMES_FILE_NAME: &str = "games.jsonl";

/// Streams and structurally validates one persisted Arena dataset.
///
/// Opening validates `manifest.json`. Iteration reads exactly one
/// `games.jsonl` line at a time and validates the record schema, zero-based
/// contiguous game IDs, and final record count. It intentionally does not
/// replay actions or calculate statistics.
pub struct JsonlDatasetReader {
    root: PathBuf,
    manifest: ArenaManifest,
    games: io::Lines<BufReader<File>>,
    next_line_number: u64,
    next_game_id: u64,
    finished: bool,
}

impl JsonlDatasetReader {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, DatasetError> {
        let root = root.as_ref().to_path_buf();
        let manifest_file = File::open(root.join(MANIFEST_FILE_NAME))?;
        let manifest = serde_json::from_reader(BufReader::new(manifest_file))?;
        validate_manifest(&manifest)?;
        let games_file = File::open(root.join(GAMES_FILE_NAME))?;

        Ok(Self {
            root,
            manifest,
            games: BufReader::new(games_file).lines(),
            next_line_number: 1,
            next_game_id: 0,
            finished: false,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &ArenaManifest {
        &self.manifest
    }

    pub fn read_games(&self) -> u64 {
        self.next_game_id
    }

    /// Consume all unread records and validate the final record count.
    pub fn finish(mut self) -> Result<(), DatasetError> {
        for record in &mut self {
            record?;
        }
        Ok(())
    }

    fn invalid_line(&mut self, line_number: u64, message: String) -> DatasetError {
        self.finished = true;
        DatasetError::InvalidDataset(format!("{GAMES_FILE_NAME} line {line_number}: {message}"))
    }
}

impl Iterator for JsonlDatasetReader {
    type Item = Result<GameRecord, DatasetError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let line_number = self.next_line_number;
        let line = match self.games.next() {
            Some(Ok(line)) => line,
            Some(Err(error)) => {
                self.finished = true;
                return Some(Err(DatasetError::Io(error)));
            },
            None => {
                self.finished = true;
                let expected = self.manifest.total_games();
                return (self.next_game_id != expected).then_some(Err(
                    DatasetError::IncompleteDataset { expected, written: self.next_game_id },
                ));
            },
        };
        self.next_line_number += 1;

        if self.next_game_id >= self.manifest.total_games() {
            let message = format!(
                "unexpected game record; manifest declares {} games",
                self.manifest.total_games()
            );
            return Some(Err(self.invalid_line(line_number, message)));
        }

        let record = match serde_json::from_str::<GameRecord>(&line) {
            Ok(record) => record,
            Err(source) => {
                self.finished = true;
                return Some(Err(DatasetError::JsonLine { line_number, source }));
            },
        };
        if record.schema_version != RECORD_SCHEMA_VERSION {
            return Some(Err(self.invalid_line(
                line_number,
                format!("unsupported schema version {}", record.schema_version),
            )));
        }
        if record.game_id != self.next_game_id {
            let message = format!("expected game id {}, got {}", self.next_game_id, record.game_id);
            return Some(Err(self.invalid_line(line_number, message)));
        }

        self.next_game_id += 1;
        Some(Ok(record))
    }
}

impl FusedIterator for JsonlDatasetReader {}

/// Writes one deterministic Arena dataset into a caller-selected directory.
///
/// The directory must not already exist. Each game is serialized fully before
/// it is appended as one JSON Lines record, so quotes and newlines in agent
/// errors are escaped by serde_json rather than written as ad-hoc log text.
pub struct JsonlDatasetWriter {
    root: PathBuf,
    manifest: ArenaManifest,
    games: BufWriter<File>,
    next_game_id: u64,
}

impl JsonlDatasetWriter {
    pub fn create(root: impl AsRef<Path>, manifest: ArenaManifest) -> Result<Self, DatasetError> {
        validate_manifest(&manifest)?;
        let root = root.as_ref().to_path_buf();
        let manifest_json = serde_json::to_vec_pretty(&manifest)?;
        if let Some(parent) = root.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        match fs::create_dir(&root) {
            Ok(()) => {},
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(DatasetError::OutputExists(root));
            },
            Err(error) => return Err(DatasetError::Io(error)),
        }
        let mut manifest_file =
            OpenOptions::new().write(true).create_new(true).open(root.join(MANIFEST_FILE_NAME))?;
        manifest_file.write_all(&manifest_json)?;
        manifest_file.write_all(b"\n")?;
        manifest_file.flush()?;
        let games_file =
            OpenOptions::new().write(true).create_new(true).open(root.join(GAMES_FILE_NAME))?;
        let games = BufWriter::new(games_file);

        Ok(Self { root, manifest, games, next_game_id: 0 })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &ArenaManifest {
        &self.manifest
    }

    pub fn written_games(&self) -> u64 {
        self.next_game_id
    }

    pub fn write_game(&mut self, run: &GameRun) -> Result<(), DatasetError> {
        if run.plan.game_id != self.next_game_id {
            return Err(DatasetError::InvalidDataset(format!(
                "expected game id {}, got {}",
                self.next_game_id, run.plan.game_id
            )));
        }
        if self.next_game_id >= self.manifest.total_games() {
            return Err(DatasetError::InvalidDataset(format!(
                "game id {} exceeds manifest schedule",
                run.plan.game_id
            )));
        }
        self.validate_run_metadata(run)?;
        let record = GameRecord::from_game_run(run)?;
        let line = serde_json::to_vec(&record)?;
        self.games.write_all(&line)?;
        self.games.write_all(b"\n")?;
        self.next_game_id += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), DatasetError> {
        self.games.flush()?;
        Ok(())
    }

    /// Flush the dataset and verify that every scheduled game was written.
    pub fn finish(mut self) -> Result<(), DatasetError> {
        self.games.flush()?;
        let expected = self.manifest.total_games();
        if self.next_game_id != expected {
            return Err(DatasetError::IncompleteDataset { expected, written: self.next_game_id });
        }
        Ok(())
    }

    fn validate_run_metadata(&self, run: &GameRun) -> Result<(), DatasetError> {
        validate_plan_shape(&self.manifest, run)?;
        validate_seat(&self.manifest, run.plan.red.as_str(), &run.red_agent, "red")?;
        validate_seat(&self.manifest, run.plan.black.as_str(), &run.black_agent, "black")?;
        if let GameTermination::MovementActionLimit { limit } = run.termination
            && limit.get() != self.manifest.game_run_config.max_movement_actions
        {
            return Err(DatasetError::InvalidDataset(format!(
                "game {} movement limit differs from manifest",
                run.plan.game_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum DatasetError {
    OutputExists(PathBuf),
    InvalidDataset(String),
    IncompleteDataset { expected: u64, written: u64 },
    JsonLine { line_number: u64, source: serde_json::Error },
    Record(RecordError),
    Json(serde_json::Error),
    Io(io::Error),
}

impl Display for DatasetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutputExists(path) => {
                write!(formatter, "dataset output already exists: {}", path.display())
            },
            Self::InvalidDataset(message) => write!(formatter, "invalid dataset: {message}"),
            Self::IncompleteDataset { expected, written } => {
                write!(formatter, "incomplete dataset: expected {expected} games, found {written}")
            },
            Self::JsonLine { line_number, source } => {
                write!(formatter, "{GAMES_FILE_NAME} line {line_number} is invalid JSON: {source}")
            },
            Self::Record(error) => Display::fmt(error, formatter),
            Self::Json(error) => write!(formatter, "JSON processing failed: {error}"),
            Self::Io(error) => write!(formatter, "dataset I/O failed: {error}"),
        }
    }
}

impl Error for DatasetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::JsonLine { source, .. } => Some(source),
            Self::Record(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::OutputExists(_) | Self::InvalidDataset(_) | Self::IncompleteDataset { .. } => {
                None
            },
        }
    }
}

impl From<RecordError> for DatasetError {
    fn from(error: RecordError) -> Self {
        Self::Record(error)
    }
}

impl From<serde_json::Error> for DatasetError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<io::Error> for DatasetError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub(crate) fn validate_manifest(manifest: &ArenaManifest) -> Result<(), DatasetError> {
    if manifest.schema_version != RECORD_SCHEMA_VERSION {
        return Err(DatasetError::InvalidDataset(format!(
            "unsupported schema version {}",
            manifest.schema_version
        )));
    }
    if manifest.seed_derivation_version != crate::SEED_DERIVATION_VERSION {
        return Err(DatasetError::InvalidDataset(format!(
            "unsupported seed derivation version {}",
            manifest.seed_derivation_version
        )));
    }
    if manifest.state_hash_algorithm != STATE_HASH_ALGORITHM {
        return Err(DatasetError::InvalidDataset(format!(
            "unsupported state hash algorithm {:?}",
            manifest.state_hash_algorithm
        )));
    }
    for participant in [&manifest.participant_a, &manifest.participant_b] {
        crate::ParticipantId::new(participant.id.clone()).map_err(|error| {
            DatasetError::InvalidDataset(format!("manifest participant ID: {error}"))
        })?;
    }
    if manifest.participant_a.id == manifest.participant_b.id {
        return Err(DatasetError::InvalidDataset("participant IDs must be distinct".to_owned()));
    }
    if manifest.total_games() == 0 {
        return Err(DatasetError::InvalidDataset(
            "schedule must contain at least one game".to_owned(),
        ));
    }
    if manifest.game_run_config.max_movement_actions == 0 {
        return Err(DatasetError::InvalidDataset(
            "movement-action limit must be nonzero".to_owned(),
        ));
    }
    Ok(())
}

fn validate_plan_shape(manifest: &ArenaManifest, run: &GameRun) -> Result<(), DatasetError> {
    let plan = &run.plan;
    let participant_a = manifest.participant_a.id.as_str();
    let participant_b = manifest.participant_b.id.as_str();
    let valid = match manifest.schedule {
        ScheduleRecord::Fixed { .. } => {
            plan.pair_id.is_none()
                && plan.game_in_pair.is_none()
                && plan.red.as_str() == participant_a
                && plan.black.as_str() == participant_b
        },
        ScheduleRecord::Paired { .. } => {
            let pair_id = plan.game_id / 2;
            let game_in_pair = (plan.game_id % 2) as u8;
            let seats_match = if game_in_pair == 0 {
                plan.red.as_str() == participant_a && plan.black.as_str() == participant_b
            } else {
                plan.red.as_str() == participant_b && plan.black.as_str() == participant_a
            };
            plan.pair_id == Some(pair_id) && plan.game_in_pair == Some(game_in_pair) && seats_match
        },
    };
    if valid {
        Ok(())
    } else {
        Err(DatasetError::InvalidDataset(format!(
            "game {} plan does not match manifest schedule",
            plan.game_id
        )))
    }
}

fn validate_seat(
    manifest: &ArenaManifest, participant_id: &str, descriptor: &AgentDescriptor, color: &str,
) -> Result<(), DatasetError> {
    let participant = manifest.participant(participant_id).ok_or_else(|| {
        DatasetError::InvalidDataset(format!(
            "{color} participant {participant_id:?} is absent from manifest"
        ))
    })?;
    if AgentDescriptorRecord::from(descriptor) != participant.agent {
        return Err(DatasetError::InvalidDataset(format!(
            "{color} agent descriptor differs from manifest participant {participant_id}"
        )));
    }
    Ok(())
}
