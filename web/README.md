# formation-chess-web

`formation-chess-web` is a local browser client for
[Formation Chess (阵棋)](..). Axum serves the static HTML, CSS, JavaScript,
and Chinese rule text directly from the compiled binary; no frontend build
step is required.

The server binds to `127.0.0.1`, keeps one in-memory game, and is intended as a
small reference interface rather than a network multiplayer service.

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

The process accepts only this optional numeric port argument. Restarting the
process starts a fresh game; there is no durable storage or authentication.

## UI features

- standard 9×10 games from the placement phase;
- custom board dimensions from 1×1 through 16×16;
- random layouts for quick experiments;
- loading a game snapshot through the text notation format;
- legal-action hints for a selected piece;
- pass, resign, and one-step undo of the most recent successful action;
- an in-app panel containing the embedded Chinese rules text.

The undo buffer stores one previous `Game` snapshot. Creating a new game clears
that buffer, and undo cannot be chained.

## HTTP API

API requests and responses use JSON where applicable. Board coordinates are
**0-based** `[x, y]` pairs,
where `x` is the column and `y` is the row. Enum-like strings use the English
serialized values such as `"Red"`, `"Black"`, `"Unfinished"`, and
`"RedWin"`.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/` | Serve the web UI |
| `GET` | `/api/state` | Return the current state |
| `POST` | `/api/action` | Submit one placement or movement action |
| `POST` | `/api/hints` | Return legal movement actions for `{ "x": 0, "y": 0 }` |
| `POST` | `/api/new` | Create a standard, custom, random, or notation-loaded game |
| `POST` | `/api/undo` | Restore the one saved pre-action state |
| `GET` | `/api/rules` | Return the embedded Chinese Markdown as `{ "text": "..." }` |

### Action requests

`POST /api/action` uses a tagged object. The supported `type` values are:

```json
{ "type": "move", "from": [4, 9], "to": [4, 8] }
```

The other movement types are `capture`, `push`, `draw`, and `divide`, each
using `from` and `to`. Placement uses a piece reference and destination:

```json
{ "type": "place", "piece": { "name": "将", "color": "Red" }, "to": [4, 9] }
```

`pass` and `resign` have no additional fields. A successful request returns
the new state with `error: null`; a rejected request returns the unchanged
state with a human-readable `error` string.

### New-game requests

`POST /api/new` accepts either a `notation` snapshot or a JSON board
configuration. When no cells or pools are supplied, a standard pair of piece
pools is created automatically. Board dimensions are limited to 16×16.

```json
{ "board": { "width": 9, "height": 10 } }
```

For the exact snapshot format and the core validation rules, see
[`docs/notation.md`](../docs/notation.md) and
[`core/README.md`](../core/README.md).
