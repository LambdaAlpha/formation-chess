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
exact AgentError produced by a failed analysis. Persistent recording, replay,
and statistics belong to subsequent layers.

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
