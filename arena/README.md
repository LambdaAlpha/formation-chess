# Formation Chess Arena

formation-chess-arena provides the reproducible orchestration layer for agent
matches: stable participant identities, fresh seeded AgentFactory instances,
lazy deterministic schedules, and bounded single-game execution.

## Agent factories

AgentFactory separates an agent implementation from one mutable game instance.
Every arena seat receives a fresh agent created from an explicit seed. The
AgentDescriptor records a stable implementation kind, display name, version,
and configuration parameters. Two participants may use identical descriptors
while retaining distinct participant IDs.

RandomAgentFactory wraps the bundled RandomAgent baseline and forwards the
arena-provided seed to RandomAgent::with_seed.

## Schedules

ScheduleMode::Fixed keeps participant A as Red and participant B as Black.
ScheduleMode::Paired emits two games per pair: A-Red/B-Black followed by
B-Red/A-Black. Each participant keeps the same agent seed across the pair, and
both games share one scenario seed for future externally randomized setups.

The schedule is lazy and derives every seed from the root seed with a versioned
SplitMix64-based algorithm.

## Game execution

MatchRunner binds the two participant IDs to their AgentFactory implementations,
maps color-swapped plans back to the correct factory, and creates fresh agents
from the plan's color-specific seeds. The caller supplies the initial Game, so a
future scenario generator can use the plan's scenario seed without coupling it
to the runner.

Every successful action retains its player, phase, action, score, reaction, and
movement legal-action count. Technical decision timing is intentionally omitted.
A run ends with a core game result, a configured movement-action limit, or the
exact AgentError produced by a failed analysis. Persistent recording, strict
replay verification, and per-game descriptive metrics are provided below.

## Persistent records

JsonlDatasetWriter writes to an explicit caller-selected directory and refuses
to reuse an existing path. The crate therefore does not place generated data in
the source repository unless the caller deliberately chooses such a path.

Each dataset contains:

- `manifest.json`: schema, Arena and Core versions, seed-derivation version,
  SHA-256 state-hash algorithm, root seed, schedule, movement limit, participant
  IDs, and full agent descriptors.
- `games.jsonl`: one complete game per JSON line. Each line contains plan and
  seat seeds, canonical initial/final states and hashes, final result,
  termination or exact agent error, separate Red/Black action counts, and the
  ordered action list.

Each executed action records its zero-based index, player, phase, structured
0-based coordinates, human-readable notation, agent score, movement legal-action
count, structured reaction, reaction notation, and post-action state hash. The
full movement legal-action list is deterministic from the recorded state and is
therefore recomputed during analysis rather than duplicated in the dataset;
placement records have no legal-action count. The initial hash plus each
post-action hash verifies deterministic replay without duplicating full state at
every step; hashes are integrity metadata, not gameplay metrics. JSON
serialization handles quotes, newlines, and other escaping inside agent error
messages.

JsonlDatasetReader validates the manifest when opened and then streams one
`games.jsonl` record at a time without loading the dataset into memory. It
reports one-based JSON line numbers, requires the current record schema and
zero-based contiguous game IDs, and checks the declared game count at EOF. It
does not replay actions or calculate statistics; those remain separate stages.
## Replay verification

ReplayVerifier strictly replays an immutable GameRecord from its canonical
initial state. It verifies the record schema, initial and final state hashes,
zero-based action indices, player and phase, finite scores, movement legal-action
counts, core action legality, action and reaction notation, structured reactions,
post-action hashes, Red/Black action counts, final state and result, and the
recorded termination context.

Verification returns the first ReplayError with game and optional action context.
It never repairs or migrates data. Agent scores are not reproducible and are only
checked for finiteness; an AgentFailure payload is retained verbatim while its
player and phase are checked against the final replay state.

## Batch execution

BatchHarness consumes a fresh Schedule, a matching MatchRunner, and a
ScenarioFactory, then executes and writes games sequentially. BatchHarness::new
uses DefaultScenarioFactory, which returns Game::default() for every scenario
seed; BatchHarness::with_scenario accepts a deterministic custom factory.

BatchHarness::run requires a nonzero game flush interval and calls the dataset
writer's flush method after every N successfully written games. Successful
completion also flushes the final shorter interval. If the process is interrupted,
complete records through the latest flush checkpoint remain available; later
buffered data may be absent. The resulting prefix is intentionally not resumable
and will fail the reader's final game-count validation.

AgentFailure is a recorded game termination, so the harness writes that game and
continues with the next plan. Scenario generation, matchup, record conversion,
and dataset I/O errors stop the batch immediately. The harness does not run
matches in parallel.

## Per-game metrics

GameMetrics::from_record strictly replays the complete GameRecord before
returning any values. Invalid or tampered records return MetricsError instead of
producing partial metrics. Agent failure diagnostic text remains in the raw
record and is deliberately excluded from metrics; its termination category,
player, and phase are retained.

Each GameMetrics value contains:

- game, pair, seat, participant, result, termination, and last-action dimensions;
- separate Red and Black placement/movement action counts;
- counts and all-action ratios for placement, move, capture, push, draw, divide,
  pass, and resignation actions;
- count, minimum, maximum, mean, median, P25, P75, P90, and P95 of movement
  legal-action counts;
- occupancy additions, removals, and replacements across reactions;
- final board counts by color plus Red, Black, and White pool counts; and
- total state visits, unique states, repeated visits, and unique-state ratio.

Action-type ratios use all actions in the game as their denominator. A zero-action
game stores null ratios instead of claiming that every action type occurred at
zero percent. Empty legal-action distributions likewise store null summary
values. Percentiles use the Type-7 definition with index `(n - 1) * p` and linear
interpolation between adjacent sorted observations.

State visits include the initial state and every post-action state. Unique states
are identified by replay-verified SHA-256 state hashes, repeated visits equal
total visits minus unique states, and the unique-state ratio divides unique
states by total visits. Reaction counts classify each verified position change
from its pre-action occupancy to its resulting occupancy.

These metrics are descriptive. They do not estimate position value, advantage,
fairness, difficulty, depth, quality, or interestingness, and they contain no
technical timing, stability, or human-player measurements.