# Formation Chess Agent Framework

formation-chess-agent defines one ranked-analysis interface for AI players and
provides random, pure MCTS, and depth-limited Min implementations.

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
  legal core movement action, including targeted Resign actions for controlled
  pieces with Leader.

prepare_turn creates a PreparedTurn that borrows the exact immutable Game and
owns any enumerated movement actions. Core enumeration order is preserved.
Orchestrators can inspect and reuse this
prepared input without enumerating legal actions a second time.
analyze_prepared validates an agent against that prepared input. analyze_agent
is the convenience wrapper that prepares and analyzes in one call.
play_agent_turn receives an ActionSelector, requests the selector's top_k, and
executes the selected candidate through the core engine. ActionSelectionPolicy::Best
preserves deterministic top-one play. RankSoftmax remains available for persisted
rank-based policies. ScoreSoftmax samples close candidates from their actual score
gaps and selects a clear best move deterministically. Agent analysis and hint
ordering remain deterministic. Selectors can use process entropy
for Web play or an explicit seed for reproducible Arena runs.

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

## Pure MCTS baseline

MctsAgent implements seeded UCT without static evaluation, action priors, or
hand-authored move ordering. Each node expands legal actions in a seeded random
order. Rollouts choose uniformly from every legal action, including Pull and
targeted Resign during movement. Only terminal game results are scored: win is
1, loss is -1, and draw is 0 from the root player's perspective.

MctsConfig::baseline() uses 128 iterations, exploration 0.7, and a hard
simulation limit of 128 actions from the analyzed root. Placement actions count
toward that limit, so a simulation from the standard initial state can include
all 32 placement actions and at most 96 later actions. Reaching the limit while
the game is unfinished returns neutral reward 0; this treats the cutoff as a
forced draw rather than adding a static estimate or an explicit per-step score.

The selected root policy is maximum visit count, with mean terminal reward as a
tie-break. Public ScoredAction values expose each returned root child's share of
all root visits. MctsAgent::with_seed and the versioned, hashable MctsConfig make
Arena runs reproducible; MctsStats exposes coverage, rollout termination, and
simulated-action counts for performance analysis.

## Min configuration

MinConfig::best() is the only bundled Min profile. It carries a schema version,
a frozen configuration version, deterministic placement and movement search
limits, a static-evaluation model version, and relative weights for oriented
feature groups. Config validation enforces the two-ply principal limit, bounded
search widths and node budgets, and keeps every non-terminal heuristic value strictly
inside the exact win/loss utility.

Feature-group weights are not independent game scores. Terminal outcomes and
searched tactical results remain non-compensable; the weights will combine
normalized, conditional features only at non-terminal leaves. With draw utility
fixed at zero, positive continuations naturally outrank a draw and a draw
naturally outranks negative continuations.

canonical_text() defines a stable, human-readable hash input. sha256() validates
the configuration before hashing it. Arena records should store the versioned
ID, the complete serialized configuration, the hash format version, and the
SHA-256 instead of storing only the moving best alias.

MinEvaluator implements the deterministic static evaluator used at future search
leaves. It evaluates both players from the same board, normalizes every feature
group to a fixed signed range, exposes each weighted contribution, and performs
all aggregation with integer arithmetic. Finished games receive exact utility;
non-terminal utility remains strictly inside that bound.

The evaluator covers Leader safety, inherent abilities, net formation changes,
control, attack-tested empty-destination mobility, actual resolved
capture/push/pull outcomes, low-weight Red/Black material, side-to-move tempo,
and control × ability × mobility interactions. Placement control also measures
row/column coverage, connected-component concentration, excessive neighbors,
and long-range ally blocking; formation value pays a cost for every adjacent
allied edge. Positive capture or blocked-push exchanges use up to two same-point
replies as a bounded static exchange evaluation, so an undefended tactical gain
can be rejected while a defended capture recovers value through its recapture
chain. Exchange pressure groups those gains by distinct target instead of
rewarding duplicate attacks on one hanging piece. Quiet mobility and Leader
escape counts exclude destinations where a currently reachable opponent capture
wins the game or produces a favorable exchange. Negative reply-adjusted
exchanges do not create mobility or action-effect bonuses. draw actions exchange
the two pieces with Leader and are scored only through their exact terminal utility of
zero. Targeted Resign actions receive the exact result implied by the selected
piece with Leader.

MinAgent implements deterministic placement analysis. It scans unique
piece-position combinations lazily rather than materializing the Cartesian
product, statically orders a bounded root beam, and minimizes selected opponent
placements. Every simulated placement probe consumes the configured node budget.
When a budget cannot scan
the full candidate space, deterministic spread sampling covers the complete
Cartesian range instead of exhausting one piece or board region first. The
remaining budget is divided across retained branches, favoring the current
static ordering only when indivisible remainder nodes exist.

Placement search stops when the configured depth is reached, the node budget is
exhausted, or the game enters movement. Exact terminal utility and bounded
static leaf utility remain shared with MinEvaluator.

MinAgent also implements deterministic movement search with selective third-ply
verification. It scans every unique action in the supplied root legal-action
slice, executes each on a cloned Game, and records its static ordering utility. Root actions are always
exhaustive and do not consume the movement node budget. Exact wins, draws, and
losses score 1, 0, and -1; unfinished leaves use the bounded static evaluator.

At depth two, the search minimizes selected opponent replies. At depth three,
the same two-ply search first covers every unfinished root, then the leading eight
roots are verified with a real selected response after each opponent reply.
Recursive movement actions consume the shared node budget. Roughly two thirds of
that budget cover every unfinished root; the remaining third plus unused
first-pass nodes fund the deeper verification. The best exact first-pass root
establishes an alpha bound; later roots stop once an opponent reply proves they
cannot beat it. Cutoff scores remain
bounds, while the leading roots are re-searched with a full window. Before full
evaluation, every legal reply receives a cheap PositionChanges preview. Tactical
replies and recaptures are retained in a bounded shortlist, and only that
shortlist runs the full evaluator. The shortlist keeps a half-width reserve beyond
the retained beam; the saved probes fund verification of more competitive roots
instead of repeatedly re-evaluating marginal replies.
Opponent width retains the statically worst branches.

A capture, blocked-push capture, or immediate Leader threat present after an
opponent reply can enter up to five alternating tactical response plies. The
shared node budget and response width bound every continuation; remaining nodes
are divided only among branches that stay tactically noisy. Actions returning to
the capture point are probed first so recapture chains are not lost to
enumeration order. Non-verified quiet leaves remain two-ply. An immediate terminal result at
the preferred minimax bound stops that branch early. Final
candidates are ordered by searched utility, exact scores before equal cutoff
bounds, then root static utility and original legal-action order before
truncation to top_k. Setting max_depth to one preserves pure static root analysis;
two enables the opponent reply and tactical response extension; three adds the
selective real response for the leading verified roots.

## Scope

This crate contains the agent framework, random baseline, pure UCT baseline,
versioned Min configuration, static evaluator, placement search, and selective
movement search with tactical response extension. Web integration, arena
orchestration, recording, and
statistics belong to their respective crates.
