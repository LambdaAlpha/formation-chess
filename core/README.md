# formation-chess-core

`formation-chess-core` is the dependency-free Rust rules engine for
[Formation Chess (阵棋)](../README.md). It implements the board, pieces,
formations, abilities, action validation, result tracking, and the Chinese
text protocol. It does not provide an AI, a user interface, networking, file
I/O, or game persistence.

The [game rules](../docs/rules.md) describe the player-facing rules. This file
focuses on the crate's boundaries and the APIs that a frontend or server uses.

## What the engine models

- **Board and pieces.** A rectangular board of at most 16×16, with Red and
  Black pieces.
- **Effective abilities.** A piece's stored definition is combined with the
  formations of its neighboring pieces when the board is queried. Use
  `Board::effective` when a caller needs the abilities that apply in the
  current position.
- **Actions.** Placement, ordinary movement, capture, push, pull, draw, pass,
  and resign. An invalid action returns an error without changing the
  game.
- **Results.** `Unfinished`, `RedWin`, `BlackWin`, and `Draw` are persistent;
  once a game is decided, further actions are rejected.
- **Notation.** Snapshots, actions, reactions, and position changes use the
  Chinese text format documented in [`docs/notation.md`](../docs/notation.md).

## Quick start

Run the included examples from the workspace root:

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

The first example starts a standard game and performs two placement actions:

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

After those two placements, the remaining pool lines are:

```text
红方：[军 间 谍 风 山 火 林 矛 盾 弹 雷 士 卒 马 车]
黑方：[军 间 谍 风 山 火 林 矛 盾 弹 雷 士 卒 马 车]
```

The resolver must be built from the board **before** an action. This matters
when formatting or resolving a reaction, because change entries refer to the
same pre-action snapshot.

## Text protocol at a glance

Coordinates in the text protocol are 1-based Chinese numerals, column first.
The meaning of an unsuffixed piece-plus-position expression depends on the
phase: it places a named pool piece during placement, and moves an on-board
piece during movement.

| Text | Meaning |
|---|---|
| `红将五十` | Place or move the Red General to column 5, row 10, depending on phase |
| `红车平五` | Move the named Red Rook horizontally to column 5 |
| `黑车进二` | Move the Black Rook forward two rows |
| `一二三二` | Move the piece currently at column 1, row 2 to column 3, row 2 |
| `红马四五捉` | Declare a capture at the destination |
| `红风五四推` | Declare a push at the destination |
| `红风平四拉` | Move to column 4 and pull the piece behind the origin |
| `红将按兵` / `黑将认负` | Pass / resign using the vital piece notation |

An action result is either a success:

```text
变化：[红风平二 黑马进一]
胜负：未分
```

or one error line:

```text
错误：{single-line message}
```

`NotationResolver` provides `parse_action`, `fmt_action`, `parse_reaction`,
and `fmt_reaction`. It resolves names, coordinates, and relative positions
against the game supplied at construction time. For reactions, pass the
pre-action `Game`; the resolver uses its board and placement pools to rebuild
the complete reversible `Reaction`.

## Custom positions

`Game` implements `Display` and `FromStr` for complete snapshots, and
`GameConfig` can be constructed directly. Custom configurations may use any
rectangular board supported by `Board`, arbitrary board contents, and explicit
piece pools. `Game::new` validates the configuration before returning a game,
including:

- pool colors and turn alternation during placement;
- the placement half occupied by each color while pools remain;
- at most one vital piece per side;
- consistency between the declared result and the vital pieces still in the
  board or pools.

For example, this valid snapshot is still in the placement phase because both
pools are non-empty and Black is next:

```text
行棋方：黑
红方：[弹 马]
黑方：[将 士 盾]
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
```

The parser's snapshot example is also available as
`core/examples/readme_custom.rs`.

## API map

| Module | Main contents |
|---|---|
| `game` | `Game`, `GameConfig`, phases, turn flow, validation, result tracking, and `Game::undo(Reaction)` |
| `action` | `Action`, `Move`, `Place`, `Reaction`, `PositionChange`, `PoolChange`, `GameResult`; placement actions use `PieceId`, while reversible reactions carry complete `Piece` values |
| `board` | board geometry, effective pieces, movement, push, pull, capture, draw, and legal-action enumeration |
| `piece` | `Piece`, lightweight `PieceId`, `Player`, canonical `Piece` constants, and `Piece::id()` for identity conversion |
| `formation` | active neighbor patterns and ability-rewriting effects |
| `ability` | the ability bitmask and `AbilityConfig` builder |
| `notation` | notation data types and `NotationResolver` |

## Tests and examples

From the workspace root:

```sh
cargo test -p formation-chess-core
cargo test --workspace
```

The crate's tests combine API tests with data-driven scenarios in
`core/tests/*.txt`. The scenarios are useful as executable examples of edge
cases such as blocked pushes, pulls, capture conversion, mutual destruction,
placement validation, and draw exchanges.

## License

Licensed under either the Apache License, Version 2.0 or the MIT license, at
your option. See [`LICENSE-APACHE`](../LICENSE-APACHE) and
[`LICENSE-MIT`](../LICENSE-MIT).
