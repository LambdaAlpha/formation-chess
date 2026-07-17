# Formation Chess

[简体中文](README.zh-Hans.md)

Formation Chess (阵棋) is a two-player strategy board game. It borrows its
9×10 board and its piece names from Xiangqi (Chinese chess), then changes
one thing that changes everything: every piece projects a small zone of
influence around itself, called a **formation**, which rewrites what nearby
pieces — friend and foe alike — are able to do.

No piece is ever just itself. A pawn standing beside a rook can suddenly
sweep across the whole board; a cannon that strays into an enemy cannon's
formation loses the ability to fire. And there is no fixed starting
position: each player freely places all 16 of their pieces on their own
half of the board before the first move, so every game begins with a
design of your own making.

## Learn to Play

- **[Game Rules](docs/rules.md)** — the complete rulebook: the board, the
  pieces, abilities, formations, and how a game is played and won.
- **[Text Notation](docs/notation.md)** — how positions and moves are
  written down as plain text, for recording and sharing games.

## Project Status

This repository currently contains the game's **rules engine**: a library
that implements the full rules and the text notation. Playable frontends
are planned but not built yet — for now the game can only be driven
through the engine's programming interface or its text protocol.

## For Developers

The engine is the Rust crate `formation-chess-core` in [`core/`](core),
with no external dependencies. Build and test with:

```sh
cargo build
cargo test
```

A minimal session — start a standard game and play the first two placement
moves ([core/examples/readme.rs](core/examples/readme.rs), runnable with
`cargo run --example readme`):

```rust
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let action = NotationResolver::new(game.board()).parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
```

Beyond the standard setup, the engine also accepts custom configurations —
board sizes up to 16×16 and arbitrary starting positions — supplied as
`GameConfig` values or as snapshots in the text protocol; see the
[notation document](docs/notation.md).

Repository layout:

```
core/          the rules engine (crate formation-chess-core)
core/tests/    data-driven test suites (*.txt files)
docs/          game documentation (rules, notation)
```

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
