# formation-chess-web

A local web UI for [Formation Chess (阵棋)](..), built with [Axum](https://github.com/tokio-rs/axum) and vanilla JavaScript. No build tools, no framework — Rust serving a static frontend embedded in the binary.

## Quick start

**From crates.io:**

```sh
cargo install formation-chess-web
formation-chess-web
```

**From source:**

```sh
cargo run -p formation-chess-web
```

Opens the browser automatically at an available local port. To specify a port:

```sh
formation-chess-web 4000          # after install
cargo run -p formation-chess-web -- 4000  # from source
```

## API endpoints

| Method | Path | Description |
|:--|:--|:--|
| `GET` | `/` | Web UI |
| `GET` | `/api/state` | Full game state |
| `POST` | `/api/action` | Submit an action |
| `POST` | `/api/hints` | Legal moves or white placements |
| `POST` | `/api/new` | Create a game from config |
| `GET` | `/api/rules` | Chinese rules text |

All request/response bodies are JSON. Coordinates are 0-based `[x, y]`. Enum values are serialised in English (`"Red"`, `"BlackWin"`, etc.).
