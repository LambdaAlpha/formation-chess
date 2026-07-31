# Formation Chess

[简体中文](README.zh-Hans.md)

Formation Chess (阵棋) is a two-player abstract strategy game about local
influence. It uses a 9×10 board and a set of Xiangqi-inspired piece names,
but not Xiangqi's palace, river, check, or fixed opening.

Each colored piece projects a **formation** over selected neighboring points;
the neutral White piece projects no active formation. A piece standing in a
formation may gain or lose abilities: movement directions,
range, control, pushing, capturing, or one of the game's special interactions.
The same named piece can therefore play very differently from one position to
the next.

The standard game starts with an empty board. Red and Black each place their
16 unique pieces on their own half of the board, then alternate movement
actions. Captured pieces are recycled into a shared white-piece pool, and a
piece with the **Divide** ability can bring one back onto the board.

## Learn the game

- **[Game rules](docs/rules.md)** — the complete standard rulebook, including
  abilities, formations, movement, combat, white pieces, and end conditions.
- **[Text notation](docs/notation.md)** — the canonical text format for
  snapshots, actions, results, and the documented game-record format.
- **[Chinese rules (Web copy)](web/static/rules.zh-Hans.md)** — the Chinese
  rule text embedded by the local Web client.

## What is in this repository

- **`core/` — `formation-chess-core`**: the dependency-free Rust rules engine
  and text-notation implementation. It has no AI, persistence, or user
  interface.
- **`tui/` — `formation-chess-tui`**: a small terminal client with standard,
  random-layout, and snapshot-loading modes.
- **`web/` — `formation-chess-web`**: a local browser client and HTTP server.
  It serves the frontend from the binary and keeps one in-memory game.
- **`docs/`**: the source rulebook and notation specification.
- **`core/tests/`**: data-driven and API-level tests for the engine.

The clients are local reference interfaces. This repository does not include
network matchmaking, an online service, an AI opponent, or durable game
storage.

## Quick start

Build and test the workspace:

```sh
cargo build --workspace
cargo test --workspace
```

Run the terminal client:

```sh
cargo run -p formation-chess-tui
```

Run the browser client:

```sh
cargo run -p formation-chess-web
```

The Web server binds to loopback, chooses an available port when none is
given, and attempts to open the browser. To request a port explicitly:

```sh
cargo run -p formation-chess-web -- 4000
```

The core crate also ships two executable examples:

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

The first starts a standard game and plays two placement actions. The second
loads and validates a custom text snapshot.

## A minimal engine session

The engine accepts notation only after it has been resolved against the
current board and phase:

```rust
use formation_chess_core::game::{Game, GameConfig};
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let resolver = NotationResolver::new(game.board(), game.phase());
        let action = resolver.parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
```

For the public API, snapshot validation, and the text protocol, start with
[`core/README.md`](core/README.md).

## Custom boards

`GameConfig` and the snapshot protocol can describe rectangular boards up to
16×16 and positions that do not arise from a standard game. The engine still
validates colors, vital-piece counts, placement halves, pool alternation, and
the declared result. The [notation document](docs/notation.md) describes the
accepted snapshot format and its limits.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
