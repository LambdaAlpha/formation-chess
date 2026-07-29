# formation-chess-core

The rules engine and text notation for **Formation Chess** (阵棋), a
two-player strategy board game. It borrows its 9×10 board and its piece
names from Xiangqi (Chinese chess), then changes one thing that changes
everything: every piece projects a small zone of influence around itself,
called a **formation**, which rewrites what nearby pieces — friend and
foe alike — are able to do.

No piece is ever just itself. A pawn standing beside a rook can suddenly
sweep across the whole board; a rook standing in an enemy rook's formation sees
its range collapse to a single step. And there is no fixed starting
position: each player freely places all 16 of their pieces on their own
half of the board before the first move, so every game begins with a
design of your own making.

This crate is a complete implementation of the rules and of the game's
Chinese text protocol, with **zero dependencies**. It contains no AI, no
UI, and no I/O: it validates and executes actions, tracks the game
result, and converts everything from and to plain text — a foundation
for building frontends, servers, and engines.

## The game in a nutshell

- **Free setup.** The board starts empty. Beginning with Red, the
  players alternate placing one piece per turn onto any empty point of
  their own half. Movement begins only when both armies are fully
  placed.
- **Abilities.** Every piece carries a set of boolean abilities: its
  movement directions and range, whether it can capture or be captured,
  push or be pushed, whether its
  loss loses the game (*vital*), and more.
- **Formations.** Every piece projects a fixed pattern over some of its
  eight neighboring points. Pieces standing on covered points have
  their abilities rewritten — a rook grants allies its unlimited range,
  a shield makes its neighbors uncapturable, an enemy spear strips your
  pieces of the ability to capture. Effects combine order-independently,
  and a denial always beats a grant.
- **Turns.** After placement, a turn is one of: move to an empty point,
  **capture** an occupied one, **push** its occupant one point onward
  (a blocked push may escalate into a capture if either piece has the
  escalation ability), place a white piece
  through a wizard, pass, or resign.
- **White pieces.** Captured pieces are recycled into a shared neutral
  pool. A wizard can drop them back onto the board as weak, colorless
  blockers, commanded only through a wizard's formation.
- **Game end.** Lose your vital piece — the general — and you lose the
  game. Both vitals perishing in the same action is a draw, as are two
  generals standing inside each other's formation.

Each side fields 16 distinct pieces in four groups:

| Group | Pieces | Formation theme |
|---|---|---|
| Control | 将 General (vital), 巫 Wizard, 间 Agent, 谍 Spy | who commands whom |
| Movement | 车 Rook, 卒 Pawn, 士 Scholar, 马 Horse | grant allies their movement, strip it from enemies |
| Push | 风 Wind, 山 Mountain, 火 Fire, 林 Forest | shoving, push escalation, and capture demotion |
| Capture | 矛 Spear, 盾 Shield, 弹 Shell, 雷 Mine | capture, immunity, sacrifice, and retaliation |

## Quick start

Start a standard game and play the first two placement moves (shipped as
an example, runnable with `cargo run --example readme`):

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

This prints the result of each action, then the position as a snapshot
of the text protocol:

```text
红将五十 → 未分
黑将五一 → 未分
行棋方：红
红方：[雷 巫 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹]
黑方：[雷 巫 间 谍 士 卒 车 马 风 山 火 林 矛 盾 弹]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路 六路 七路 八路 九路]
一[一一 一一 一一 一一 黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一 一一 一一 一一 一一]
四[一一 一一 一一 一一 一一 一一 一一 一一 一一]
五[一一 一一 一一 一一 一一 一一 一一 一一 一一]
六[一一 一一 一一 一一 一一 一一 一一 一一 一一]
七[一一 一一 一一 一一 一一 一一 一一 一一 一一]
八[一一 一一 一一 一一 一一 一一 一一 一一 一一]
九[一一 一一 一一 一一 一一 一一 一一 一一 一一]
十[一一 一一 一一 一一 红将 一一 一一 一一 一一]
```

## The text protocol

Games can be driven entirely through plain text — every action, every
result, and every position has a canonical textual form, so a game
server or a human-readable game record needs nothing beyond this crate.

Coordinates are 1-based Chinese numerals, column first: `五十` is
column 5, row 10. A piece is referred to by color-prefixed name
(`红车`) while it is unique, or by the coordinates of its current point
(`一二`) when several identical pieces are on the board.

Actions:

| Notation | Meaning |
|---|---|
| `红将五十` | the Red General enters at / moves to column 5, row 10 |
| `红车平五` | the Red Rook moves horizontally to column 5 |
| `黑将进二` | the Black General advances two steps |
| `一二三二` | the piece standing on (1,2) moves to (3,2) |
| `红马四五捉` | the Red Horse moves to (4,5) and **captures** the piece there |
| `红风五四推` | the Red Wind moves to (5,4) and **pushes** its occupant one point onward |
| `白子二四占` | a white piece from the pool is **placed** on (2,4) |
| `红按兵` / `黑认负` | Red passes / Black resigns |

Every action answers with either a success — the list of piece changes
plus the game result — or a single error line. The push above, for
example, answers:

```text
变化：[红风进一 黑马退一]
胜负：未分
```

```text
错误：{single-line message}
```

`NotationResolver` converts both directions: `parse_action` /
`fmt_action` for actions and `parse_reaction` / `fmt_reaction` for
results, resolving names and 1-based coordinates against the current
board.

## Custom positions

`Game` implements `Display` and `FromStr` for full snapshots — the same
six-section format printed above — and `GameConfig` is a plain struct
you can build by hand. Boards go up to 16×16, pools and positions are
arbitrary, and `Game::new` validates whatever you supply (piece colors,
vital-piece counts, placement halves, pool alternation, declared
result). Runnable with `cargo run --example readme_custom`:

```rust
use formation_chess_core::game::Game;

fn main() -> Result<(), String> {
    let game: Game = "行棋方：黑
红方：[弹 马]
黑方：[将 士 盾]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
"
    .parse()?;

    assert!(game.is_placement_phase(), "pools are not empty so phase must be placement");
    Ok(())
}
```

## API overview

| Module | Contents |
|---|---|
| `game` | `Game` and `GameConfig`: validation, turn flow, result tracking, snapshot `Display`/`FromStr` |
| `action` | plain data types: `Action`, `Reaction`, `PositionChange`, `GameResult` |
| `board` | `Board`: geometry, movement/push/capture execution, formation-effective piece lookup |
| `piece` | `Piece`, `Color`, `Player`, and every canonical piece as a constant (`Piece::RED_GENERAL`, …) |
| `formation` | `Formation`: the neighbor pattern and its ability-rewriting effect |
| `ability` | the `Ability` bitmask and the readable `AbilityConfig` builder |
| `notation` | `NotationResolver` and the notation data types for actions, reactions, and changes |

The complete rulebook and the full notation specification (in English
and Simplified Chinese) live in the repository:
<https://github.com/LambdaAlpha/formation-chess>.

## License

Licensed under either of

- Apache License, Version 2.0 (<http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (<http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.
