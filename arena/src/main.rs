use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::num::NonZeroU32;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::process::ExitCode;

use formation_chess_agent::ActionSelectionPolicy;
use formation_chess_arena::BatchHarness;
use formation_chess_arena::DatasetAnalyzer;
use formation_chess_arena::GameRunConfig;
use formation_chess_arena::JsonlDatasetReader;
use formation_chess_arena::MatchRunner;
use formation_chess_arena::Matchup;
use formation_chess_arena::ParticipantId;
use formation_chess_arena::RandomAgentFactory;
use formation_chess_arena::ReplayVerifier;
use formation_chess_arena::Schedule;
use formation_chess_arena::ScheduleMode;

const PROGRAM_NAME: &str = "formation-chess-arena";
const DEFAULT_FLUSH_EVERY_GAMES: NonZeroU64 = NonZeroU64::MIN;
const HELP: &str = "Formation Chess Arena

Usage:
  formation-chess-arena run --output <DIR> --seed <U64> (--fixed <GAMES> | --paired <PAIRS>) --movement-limit <ACTIONS> --participant-a <ID> --participant-b <ID> [--flush-every <GAMES>]
  formation-chess-arena verify <DATASET_DIR>
  formation-chess-arena stats <DATASET_DIR>
  formation-chess-arena --help
  formation-chess-arena --version

Commands:
  run       Run a deterministic Random-vs-Random schedule and write a dataset.
  verify    Structurally validate and strictly replay every recorded game.
  stats     Verify the dataset and write game_metrics.csv plus summary.json.

Run options:
  --output <DIR>          New dataset directory; it must not already exist.
  --seed <U64>            Root seed used to derive scenario and agent seeds.
  --fixed <GAMES>         Fixed seats: participant A is always Red.
  --paired <PAIRS>        Color-swapped pairs; each pair writes two games.
  --movement-limit <N>    Nonzero maximum movement actions per game.
  --participant-a <ID>    First participant identity.
  --participant-b <ID>    Second, distinct participant identity.
  --flush-every <GAMES>   Nonzero flush interval; defaults to 1.

Participant IDs must be non-empty and contain no whitespace. The current CLI
registers only the deterministic RandomAgent implementation.
";

enum Invocation {
    Command(Command),
    Help,
    Version,
}

enum Command {
    Run(RunOptions),
    Verify(PathBuf),
    Stats(PathBuf),
}

struct RunOptions {
    output_root: PathBuf,
    root_seed: u64,
    schedule: ScheduleSelection,
    movement_limit: NonZeroU32,
    matchup: Matchup,
    flush_every_games: NonZeroU64,
}

#[derive(Debug, Copy, Clone)]
enum ScheduleSelection {
    Fixed(NonZeroU32),
    Paired(NonZeroU32),
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match parse_invocation(&arguments) {
        Ok(Invocation::Help) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        },
        Ok(Invocation::Version) => {
            println!("{PROGRAM_NAME} {}", formation_chess_arena::VERSION);
            ExitCode::SUCCESS
        },
        Ok(Invocation::Command(command)) => match execute(command) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            },
        },
        Err(message) => {
            eprintln!("error: {message}\n\n{HELP}");
            ExitCode::from(2)
        },
    }
}

fn parse_invocation(arguments: &[OsString]) -> Result<Invocation, String> {
    let Some(command) = arguments.first() else {
        return Err("missing command".to_owned());
    };
    let command = utf8_argument(command, "command")?;

    match command {
        "help" | "-h" | "--help" => ensure_global_flag(arguments, Invocation::Help),
        "version" | "-V" | "--version" => ensure_global_flag(arguments, Invocation::Version),
        "run" => {
            let command_arguments = &arguments[1 ..];
            if contains_help(command_arguments) {
                Ok(Invocation::Help)
            } else {
                parse_run(command_arguments)
                    .map(|options| Invocation::Command(Command::Run(options)))
            }
        },
        "verify" => parse_dataset_command(arguments, Command::Verify),
        "stats" => parse_dataset_command(arguments, Command::Stats),
        _ => Err(format!("unknown command `{command}`")),
    }
}

fn ensure_global_flag(
    arguments: &[OsString], invocation: Invocation,
) -> Result<Invocation, String> {
    if arguments.len() == 1 {
        Ok(invocation)
    } else {
        Err("global help and version flags do not accept arguments".to_owned())
    }
}

fn parse_dataset_command(
    arguments: &[OsString], constructor: fn(PathBuf) -> Command,
) -> Result<Invocation, String> {
    let command_arguments = &arguments[1 ..];
    if contains_help(command_arguments) {
        return Ok(Invocation::Help);
    }
    if command_arguments.len() != 1 {
        return Err(format!(
            "command `{}` requires exactly one dataset directory",
            utf8_argument(&arguments[0], "command")?
        ));
    }

    let dataset_root = nonempty_path(&command_arguments[0], "dataset directory")?;
    Ok(Invocation::Command(constructor(dataset_root)))
}

fn parse_run(arguments: &[OsString]) -> Result<RunOptions, String> {
    let mut output_root = None;
    let mut root_seed = None;
    let mut schedule = None;
    let mut movement_limit = None;
    let mut participant_a = None;
    let mut participant_b = None;
    let mut flush_every_games = None;
    let mut index = 0;

    while index < arguments.len() {
        let option = utf8_argument(&arguments[index], "run option")?;
        let value = next_option_value(arguments, &mut index, option)?;
        match option {
            "--output" => {
                set_once(&mut output_root, nonempty_path(value, "--output")?, "--output")?;
            },
            "--seed" => {
                set_once(&mut root_seed, parse_u64(value, "--seed")?, "--seed")?;
            },
            "--fixed" => set_schedule(
                &mut schedule,
                ScheduleSelection::Fixed(parse_nonzero_u32(value, "--fixed")?),
            )?,
            "--paired" => set_schedule(
                &mut schedule,
                ScheduleSelection::Paired(parse_nonzero_u32(value, "--paired")?),
            )?,
            "--movement-limit" => set_once(
                &mut movement_limit,
                parse_nonzero_u32(value, "--movement-limit")?,
                "--movement-limit",
            )?,
            "--participant-a" => set_once(
                &mut participant_a,
                utf8_argument(value, "--participant-a value")?.to_owned(),
                "--participant-a",
            )?,
            "--participant-b" => set_once(
                &mut participant_b,
                utf8_argument(value, "--participant-b value")?.to_owned(),
                "--participant-b",
            )?,
            "--flush-every" => set_once(
                &mut flush_every_games,
                parse_nonzero_u64(value, "--flush-every")?,
                "--flush-every",
            )?,
            _ => return Err(format!("unknown run option `{option}`")),
        }
        index += 1;
    }

    let participant_a =
        parse_participant(required(participant_a, "--participant-a")?, "--participant-a")?;
    let participant_b =
        parse_participant(required(participant_b, "--participant-b")?, "--participant-b")?;
    let matchup = Matchup::new(participant_a, participant_b)
        .map_err(|error| format!("invalid participant matchup: {error}"))?;

    Ok(RunOptions {
        output_root: required(output_root, "--output")?,
        root_seed: required(root_seed, "--seed")?,
        schedule: schedule.ok_or_else(|| "missing one of `--fixed` or `--paired`".to_owned())?,
        movement_limit: required(movement_limit, "--movement-limit")?,
        matchup,
        flush_every_games: flush_every_games.unwrap_or(DEFAULT_FLUSH_EVERY_GAMES),
    })
}

fn contains_help(arguments: &[OsString]) -> bool {
    arguments.iter().any(|argument| {
        argument.as_os_str() == OsStr::new("-h") || argument.as_os_str() == OsStr::new("--help")
    })
}

fn next_option_value<'argument>(
    arguments: &'argument [OsString], index: &mut usize, option: &str,
) -> Result<&'argument OsStr, String> {
    *index += 1;
    arguments
        .get(*index)
        .map(OsString::as_os_str)
        .ok_or_else(|| format!("option `{option}` requires a value"))
}

fn set_once<T>(target: &mut Option<T>, value: T, option: &str) -> Result<(), String> {
    if target.replace(value).is_some() {
        Err(format!("option `{option}` was supplied more than once"))
    } else {
        Ok(())
    }
}

fn set_schedule(
    target: &mut Option<ScheduleSelection>, value: ScheduleSelection,
) -> Result<(), String> {
    if target.replace(value).is_some() {
        Err("exactly one of `--fixed` or `--paired` may be supplied".to_owned())
    } else {
        Ok(())
    }
}

fn required<T>(value: Option<T>, option: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("missing required option `{option}`"))
}

fn parse_participant(value: String, option: &str) -> Result<ParticipantId, String> {
    ParticipantId::new(value).map_err(|error| format!("invalid value for `{option}`: {error}"))
}

fn nonempty_path(value: &OsStr, description: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        Err(format!("{description} cannot be empty"))
    } else {
        Ok(PathBuf::from(value))
    }
}

fn utf8_argument<'argument>(
    value: &'argument OsStr, description: &str,
) -> Result<&'argument str, String> {
    value.to_str().ok_or_else(|| format!("{description} must be valid UTF-8"))
}

fn parse_u64(value: &OsStr, option: &str) -> Result<u64, String> {
    let value = utf8_argument(value, option)?;
    value.parse::<u64>().map_err(|error| format!("invalid value for `{option}`: {error}"))
}

fn parse_nonzero_u32(value: &OsStr, option: &str) -> Result<NonZeroU32, String> {
    let value = utf8_argument(value, option)?;
    let parsed =
        value.parse::<u32>().map_err(|error| format!("invalid value for `{option}`: {error}"))?;
    NonZeroU32::new(parsed).ok_or_else(|| format!("value for `{option}` must be nonzero"))
}

fn parse_nonzero_u64(value: &OsStr, option: &str) -> Result<NonZeroU64, String> {
    let value = utf8_argument(value, option)?;
    let parsed =
        value.parse::<u64>().map_err(|error| format!("invalid value for `{option}`: {error}"))?;
    NonZeroU64::new(parsed).ok_or_else(|| format!("value for `{option}` must be nonzero"))
}

fn execute(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Run(options) => run_batch(options),
        Command::Verify(dataset_root) => verify_dataset(dataset_root),
        Command::Stats(dataset_root) => analyze_dataset(dataset_root),
    }
}

fn run_batch(options: RunOptions) -> Result<(), Box<dyn Error>> {
    let schedule_mode = match options.schedule {
        ScheduleSelection::Fixed(games) => ScheduleMode::Fixed { games },
        ScheduleSelection::Paired(pairs) => ScheduleMode::Paired { pairs },
    };
    let schedule = Schedule::new(options.matchup.clone(), schedule_mode, options.root_seed);
    let participant_a_factory = RandomAgentFactory;
    let participant_b_factory = RandomAgentFactory;
    let runner = MatchRunner::new(
        options.matchup,
        &participant_a_factory,
        &participant_b_factory,
        GameRunConfig::with_action_selection(
            options.movement_limit,
            ActionSelectionPolicy::standard_rank_softmax(),
        ),
    );
    let report =
        BatchHarness::new(schedule, runner).run(&options.output_root, options.flush_every_games)?;

    println!("wrote {} games to {}", report.games_written, report.output_root.display());
    Ok(())
}

fn verify_dataset(dataset_root: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut reader = JsonlDatasetReader::open(&dataset_root)?;
    for record in &mut reader {
        ReplayVerifier::verify(&record?)?;
    }
    let games_verified = reader.read_games();

    println!("verified {games_verified} games in {}", dataset_root.display());
    Ok(())
}

fn analyze_dataset(dataset_root: PathBuf) -> Result<(), Box<dyn Error>> {
    let report = DatasetAnalyzer::analyze(&dataset_root)?;
    println!("analyzed {} games in {}", report.games_analyzed, report.dataset_root.display());
    println!("wrote {}", report.game_metrics_path.display());
    println!("wrote {}", report.summary_path.display());
    Ok(())
}
