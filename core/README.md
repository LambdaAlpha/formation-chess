# formation-chess-core

`formation-chess-core` is the dependency-free Rust rules engine for
[Formation Chess (阵棋)](../README.md). It owns the board model, canonical
pieces, formations, effective abilities, action validation, result tracking,
undo, and Chinese text notation. It does not provide AI, networking, file I/O,
persistence, or a user interface.

The player-facing rules are in [`docs/rules.md`](../docs/rules.md). This README
focuses on crate boundaries and integration APIs.

## Engine model

- **Board:** rectangular geometry from 1×1 through 16×16.
- **Pieces:** Red and Black ownership, stable `PieceId` identity, canonical
  definitions, and position-dependent effective abilities.
- **Phases:** alternating placement while either pool is non-empty, followed by
  movement when both pools are empty.
- **Actions:** place, move, capture, push, pull, draw exchange, pass, and
  targeted resign.
- **Results:** `Unfinished`, `RedWin`, `BlackWin`, and `Draw`; decided games
  reject further actions.
- **Reactions:** reversible board changes plus the exact placement-pool change
  and resulting game status.
- **Notation:** snapshots, actions, and reactions documented in
  [`docs/notation.md`](../docs/notation.md).

`Board::effective` combines a piece's stored definition with every active
neighboring formation. Callers that display or analyze current capabilities
should use the effective piece rather than the raw board value.

## Quick start

Run the included examples from the workspace root:

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

The first starts a standard game and performs two placements:

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

The remaining pool lines are:

```text
红方：[计 势 变 风 林 火 山 矛 盾 弹 雷 士 卒 马 车]
黑方：[计 势 变 风 林 火 山 矛 盾 弹 雷 士 卒 马 车]
```

A `NotationResolver` is tied to one game snapshot. Format or parse a reaction
with the **pre-action** game because change entries identify departures and
placement-pool removals from that state.

## Text protocol at a glance

Coordinates are 1-based Chinese numerals, column first. An unsuffixed
piece-plus-position expression places a pool piece during placement and moves an
on-board piece during movement.

| Text | Meaning |
|---|---|
| `红将五十` | Place or move the Red General to column 5, row 10, depending on phase |
| `红车平五` | Move the unique Red Rook horizontally to column 5 |
| `黑车进二` | Advance the unique Black Rook two rows |
| `一二三二` | Move the piece at column 1, row 2 to column 3, row 2 |
| `红马四五捉` | Declare capture intent at the destination |
| `红火五四推` | Declare push intent at the destination |
| `红风平四拉` | Move the Wind and pull the piece behind its origin |
| `红将五一和` | Exchange with an opposing Vital piece and draw |
| `红将按兵` | Pass during movement |
| `黑将认负` | Resign the controlled Black General |

A formatted outcome is either a successful reaction:

```text
变化：[红火平二 红车平三]
胜负：未分
```

or one error line:

```text
错误：{single-line message}
```

`NotationResolver` provides `parse_action`, `fmt_action`, `parse_reaction`, and
`fmt_reaction`. Errors keep the game unchanged. Successful reactions can be
passed to `Game::undo` in strict last-in-first-out order.

## Legal action enumeration

- `Game::valid_moves(x, y)` returns all legal movement-phase actions for one
  piece controlled by the current player.
- `Game::all_valid_moves(&mut actions)` appends actions for every currently
  controlled board piece.
- The board enumeration includes `Move`, `Capture`, `Push`, `Pull`, `Draw`, and
  `Resign` for controlled Vital pieces.
- Placement and `Pass` are caller-level concerns; the Agent crate appends Pass
  to the core movement action list.

Every returned action is still executed through `Game::action`, which performs
the authoritative validation and produces the reversible reaction.

## Custom positions

`Game` implements `Display` and `FromStr` for complete snapshots, and
`GameConfig` can be constructed directly. Custom configurations may contain
arbitrary board contents and explicit Red and Black placement pools.

`Game::new` validates:

- pool piece ownership;
- placement halves while either pool remains;
- Red/Black placement alternation and the next player;
- at most one Vital piece per owner across board and pool; and
- consistency between the persistent result and the Vital pieces that remain.

This valid custom snapshot is still in placement because both pools are
non-empty and Black acts next:

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

The same example is executable in `core/examples/readme_custom.rs`.

## API map

| Module | Main contents |
|---|---|
| `game` | `Game`, `GameConfig`, `Phase`, action flow, configuration validation, result tracking, and `Game::undo` |
| `action` | `Action`, `Move`, `Place`, `Reaction`, `PositionChange`, `PoolChange`, and `GameResult` |
| `board` | geometry, raw/effective pieces, movement interactions, and legal-action enumeration |
| `piece` | `Piece`, `PieceId`, `Player`, canonical piece constants, and ownership/control helpers |
| `formation` | four local patterns, Black vertical mirroring, and ability-rewriting effects |
| `ability` | the `Ability` bitset and exhaustive `AbilityConfig` builder |
| `notation` | notation data types and the game-aware `NotationResolver` |

## Tests

From the workspace root:

```sh
cargo +nightly test -p formation-chess-core
cargo +nightly test --workspace
```

The crate combines API tests with data-driven scenarios in `core/tests/*.txt`.
They cover placement validation, formation overlap, same-owner capture, push and
pull permissions, conversion rules, mutual destruction, draw exchanges,
targeted resignation, undo, and notation round trips.

## License

Licensed under either the Apache License, Version 2.0 or the MIT license, at
your option. See [`LICENSE-APACHE`](../LICENSE-APACHE) and
[`LICENSE-MIT`](../LICENSE-MIT).
