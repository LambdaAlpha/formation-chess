# Formation Chess Agent Framework

formation-chess-agent defines one ranked-analysis interface for AI players and
provides a random baseline implementation.

## Agent interface

Every Agent implements analyze and returns at most top_k complete ScoredAction
values ordered from best to worst. Scores are finite, use the current player's
perspective, and treat higher values as better. Equal scores use list order as
the tie-break order.

The framework prepares phase-specific AgentInput values:

- Placement receives the current Game and a compact geometric PlacementArea.
  The agent reads the current player's pool and board occupancy directly from
  Game, so the framework does not materialize every piece-position action.
- Movement receives the current Game and an explicit slice containing every
  legal movement action plus Pass. Resign is excluded and remains a
  higher-level policy decision.

analyze_agent validates the returned count, ordering, uniqueness, scores, and
legality without modifying the game. play_agent_turn calls the same analysis
path with top_k equal to one and executes the first candidate through the core
engine.

## Placement area

For an even-height board, Black uses the top half and Red uses the bottom half.
For an odd-height board, the middle row belongs to neither placement area, in
accordance with the core rules. PlacementArea::positions includes occupied
points because the agent already receives the complete Game and can filter
those points directly.

## Random baseline

RandomAgent::new() uses a fresh random seed. RandomAgent::with_seed(seed) is
available for replayable tests and simulations. Placement uses reservoir
sampling over a lazy iterator of legal piece-position combinations, avoiding a
materialized Cartesian product. Movement samples distinct actions from the
legal-action slice. All random candidates use score zero, with sampled list
order acting as their tie-break order.

The random baseline validates analysis and execution paths; it does not evaluate
positions or attempt to play well.

## Scope

This crate contains the agent framework and random baseline. Web integration,
arena orchestration, recording, statistics, and stronger AI implementations
belong to later phases.
