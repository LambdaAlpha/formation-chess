# Formation Chess Arena

formation-chess-arena provides the reproducible orchestration layer for agent
matches. The current foundation contains stable participant identities, fresh
seeded AgentFactory instances, and lazy deterministic schedules.

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
SplitMix64-based algorithm. It does not create games or agents yet; execution,
termination, recording, replay, and statistics belong to subsequent layers.