# formation-chess-web

`formation-chess-web` is a local browser client for
[Formation Chess (阵棋)](..). Axum serves the static HTML, CSS, JavaScript,
and Chinese rule text directly from the compiled binary; no frontend build
step is required.

The server binds to `127.0.0.1` and keeps one in-memory game session. Red and
Black each own an independent `MinAgent::best()` instance. The resolved
`MinConfig::versioned_id()` is exposed as a display name such as
`Min AI best-v1`; the Web application exposes no random baseline. A side's
control mode decides whether its Min agent only analyzes candidates or also
executes the top-one candidate.

## Run from the workspace

```sh
cargo run -p formation-chess-web
```

With no argument the server asks the operating system for an available port,
prints the URL, and attempts to open it in the default browser. To request a
specific port:

```sh
cargo run -p formation-chess-web -- 4000
```

Restarting the process starts a fresh game; there is no durable storage or
authentication. Refreshing the browser uses `GET /api/state` and keeps the
current in-memory game.

## UI features

- independent Human or AI control for Red and Black;
- one current Min best instance per side, also available as a top-five hint provider on human turns;
- manual one-action advancement when the current side is AI;
- automatic single AI replies after a human action in mixed Human/AI games;
- AI-vs-AI games that advance only when the user clicks `下一步`;
- standard and custom boards from 1×1 through 16×16;
- backend-generated random placement that returns only the completed layout;
- notation-loaded game snapshots;
- legal movement targets for a selected piece;
- candidate preview by highlighting only the action's origin and destination;
- pass, resign, and undo.

In mixed Human/AI games, undo restores the state before the latest human action
and its automatic AI reply. Human-vs-Human and AI-vs-AI games undo one action.
Creating a new game clears the undo history.

## HTTP API

Coordinates are 0-based `[x, y]` pairs. State-changing and analysis requests
carry the state `revision` and current `side`; stale requests are rejected.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/` | Serve the web UI |
| `GET` | `/api/state` | Return the current session state |
| `POST` | `/api/action` | Submit one action for a human-controlled side |
| `POST` | `/api/legal-actions` | Return legal actions for one board origin |
| `POST` | `/api/agent/analyze` | Return up to `top_k` ranked Agent candidates |
| `POST` | `/api/agent/step` | Execute the current AI side's top-one candidate |
| `POST` | `/api/new` | Create or notation-load a game |
| `POST` | `/api/undo` | Undo one action or one human-plus-AI round |
| `GET` | `/api/rules` | Return the embedded Chinese rules text |

### State and controllers

The current side is explicit even when both Agent instances have the same
name:

```json
{
  "revision": 17,
  "player": "Red",
  "phase": "movement",
  "controllers": {
    "red": { "control": "agent", "agent": "Min AI best-v1" },
    "black": { "control": "agent", "agent": "Min AI best-v1" }
  },
  "current_controller": {
    "side": "Red",
    "control": "agent",
    "agent": "Min AI best-v1"
  }
}
```

### Human actions

```json
{
  "revision": 17,
  "side": "Red",
  "action": { "type": "move", "from": [4, 9], "to": [4, 8] }
}
```

Other movement types are `capture`, `push`, `draw`, and `divide`. Placement
uses `{ "type": "place", "piece": { ... }, "to": [x, y] }`.

### Agent analysis

```json
{ "revision": 17, "side": "Red", "top_k": 5 }
```

Candidates are ordered best-first. Each candidate contains the complete action,
its notation, and its finite Agent score. Array order is the rank. The hint
endpoint preserves caller-selected top-K analysis; the step endpoint has no
`top_k` field and executes Min's top-one candidate.

### New games and random placement

```json
{
  "revision": 17,
  "side": "Red",
  "controllers": { "red": "human", "black": "agent" },
  "board": { "width": 9, "height": 10 },
  "random_placement": true
}
```

When `random_placement` is true, the backend alternates the two Min best agents
until the placement phase is complete and atomically replaces the session with
the final layout. This setup-only path uses independent standard rank-Softmax
selectors to retain layout variation; normal Agent steps remain top-one. No
intermediate placements are returned.
