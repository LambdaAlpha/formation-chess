# Formation Chess

[简体中文](README.zh-Hans.md)

Formation Chess (阵棋) is a two-player abstract strategy game about local
influence. It uses a 9×10 board and Xiangqi-inspired piece names, but it has no
palace, river, check, checkmate, or fixed opening.

Each piece projects a **formation** onto selected neighboring points. A piece
standing in those points may gain or lose movement, range, control, push, pull,
capture, or special conversion abilities. Actions always use the resulting
effective abilities, so the same named piece can behave very differently as the
position changes.

The standard game starts on an empty board. Red and Black each place 16 unique
pieces in their own half, then alternate movement-phase actions. The four piece
groups are:

```text
Strategy:        将 计 势 变
Restraint:       风 林 火 山
Offense/Defense: 矛 盾 弹 雷
Mobility:        士 卒 马 车
```

Movement play supports ordinary moves, allied or opposing captures, pushes,
pulls, Vital-piece draw exchanges, pass, and target-based resignation.

## Learn the game

- **[Game rules](docs/rules.md)** — setup, abilities, formations, all action
  types, the current piece groups, and end conditions.
- **[Text notation](docs/notation.md)** — canonical snapshots, actions,
  reactions, and the documented whole-game record convention.
- **[Chinese Web rules](web/static/rules.zh-Hans.md)** — the Chinese rule text
  embedded in the local browser client.

## Workspace crates

- **`core/` — `formation-chess-core`**: dependency-free rules engine, legal
  action enumeration, undo, snapshots, and Chinese text notation.
- **`agent/` — `formation-chess-agent`**: phase-specific ranked analysis,
  validated turn execution, and seedable Random, pure MCTS, and Min agents.
- **`arena/` — `formation-chess-arena`**: reproducible schedules, JSONL game
  records, strict replay verification, metrics, and dataset analysis.
- **`tui/` — `formation-chess-tui`**: interactive terminal client with standard,
  random-layout, and snapshot-loading modes.
- **`web/` — `formation-chess-web`**: local Axum server and embedded browser UI
  with independent Human or Min AI control for Red and Black.
- **`docs/`**: the source rulebook and notation specification.

The TUI and Web clients are local reference interfaces. The repository does not
provide network matchmaking, an online service, authentication, or durable Web
game storage. Arena datasets are written only when the Arena CLI is given an
explicit output directory.

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

The Web server binds to `127.0.0.1`, chooses an available port when none is
provided, and attempts to open the default browser. To request a port:

```sh
cargo run -p formation-chess-web -- 4000
```

Inspect the Arena command line:

```sh
cargo run -p formation-chess-arena -- --help
```

The core crate also ships two executable examples:

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

The first starts a standard game and plays two placements. The second loads and
validates a custom text snapshot.

## Minimal engine session

Notation is resolved against the current game because phase, board identity,
relative movement, pass, and targeted resignation all depend on that snapshot.

```rust
use formation_chess_core::game::{Game, GameConfig};
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let resolver = NotationResolver::new(&game);
        let action = resolver.parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
```

For public API boundaries, reversible reactions, and custom snapshots, see
[`core/README.md`](core/README.md).

## Custom boards

`GameConfig` and the snapshot protocol support rectangular boards up to 16×16
and internally consistent positions that need not be reachable from the standard
opening. Validation still checks pool ownership, placement halves, alternating
pool sizes, Vital-piece counts, and the declared result. See
[Text notation](docs/notation.md) for the accepted format.

## Development checks

The repository's full validation commands are:

```sh
cargo +nightly fmt --all -- --check
cargo +nightly test --workspace
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
```

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
