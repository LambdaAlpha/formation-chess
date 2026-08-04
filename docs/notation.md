# Formation Chess — Text Notation

[简体中文](notation.zh-Hans.md) · [Game rules](rules.md) · [Project overview](../README.md)

This specification defines Formation Chess's Chinese-character text protocol. It covers four kinds of text: **game-state snapshots**, **actions**, **action results**, and **game records**.

Snapshots, actions, and action results describe positions, requests, and responses. A game record combines an optional starting snapshot with actions in play order.

## Format conventions

- Text is UTF-8.
- Fixed labels use the full-width colon `：`.
- List entries in actions and results are separated by one space. Writers should emit the canonical form without extra whitespace.
- Snapshots are line-oriented. Readers accept both `\n` and `\r\n`; writers normally emit `\n`.
- Coordinates, row labels, and step counts use Chinese numerals. White-pool counts and game-record round numbers use Arabic digits.

A structurally valid string is not necessarily legal in a particular position. An action must also satisfy the current board, phase, player, abilities, path, target, and game-result constraints.

## Numerals and coordinates

The protocol uses these numeral characters:

| Value | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Character | 一 | 二 | 三 | 四 | 五 | 六 | 七 | 八 | 九 | 十 | 甲 | 乙 | 丙 | 丁 | 戊 | 己 |

The standard game uses a 9×10 board; the protocol supports boards up to 16×16. A coordinate is always **column first, then row**: `三二` means column 3, row 2. Columns increase from left to right and rows from top to bottom. Red advances upward; Black advances downward.

## Game-state snapshots

A complete snapshot contains five state lines, a `棋盘：` marker, and the board grid:

```text
行棋方：黑
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
```

### State lines

- `行棋方：红` or `行棋方：黑` identifies the player who acts next.
- `红方：[...]` and `黑方：[...]` list pieces not yet placed. Pool entries use the one-character piece name without a color prefix, are separated by one space, and use `[]` for an empty pool.
- `白方：0` records the number of pieces currently available in the shared White pool.
- `胜负：未分`, `红胜`, `黑胜`, or `和棋` records the persistent game result. Once the value is not `未分`, the game accepts no further action.

### Board grid

After `棋盘：`, the first grid line is the column header. It begins with `零[` and gives every column a `numeral + 路` label, such as `一路`.

Each following line starts with its row numeral and contains one entry per point:

- an empty point is `一一`;
- a Red or Black piece is its color prefix plus one-character name, such as `红车` or `黑将`;
- a White piece is `白子`.

Every cell is exactly two characters wide. The header width and every row width must agree. Neither dimension may exceed 16.

### Custom snapshots and validity

A snapshot may describe a position that normal play from the standard start could never reach. If either player pool is non-empty, the snapshot is still in the placement phase and must respect each side's half of the board. For a custom board of height `h`, Red may occupy placement rows numbered greater than `⌈h/2⌉`, and Black rows numbered at most `⌊h/2⌋`. On an odd-height board, the center row belongs to neither placement half.

A snapshot is invalid if it violates any of these conditions:

- either side has more than one piece with the Vital ability;
- a placement-phase piece lies outside its side's half;
- while pools remain, their sizes and the player to move could not result from alternating Red and Black placements;
- a snapshot declared `未分` does not leave both sides a Vital piece, or a declared Red or Black win does not leave the winner a Vital piece.

A snapshot only needs internally consistent structure and a consistent declared result. The position need not be reachable from the standard opening.

## Actions

Most actions have this form:

```text
piece reference + destination + optional suffix
```

Pass and resign are separate fixed phrases.

### Piece references

A piece on the board may be identified by a color-prefixed name, such as `红车`, `黑将`, or `白子`. A name is sufficient only when exactly one matching piece is on the board. If several matching pieces exist, identify the acting piece by its pre-action coordinate; for example, `一二` means the piece at column 1, row 2.

Placement must use a color-prefixed piece name because the piece comes from a player pool. A coordinate cannot identify a piece that is not yet on the board.

### Destinations

An **absolute destination** is a column numeral followed by a row numeral, such as `三二`.

A **relative destination** uses one of four operators:

- `平` + column: move to that column on the same row;
- `直` + row: move to that row in the same column;
- `进` + steps: move forward—toward smaller row numbers for Red, larger row numbers for Black;
- `退` + steps: move backward.

`进` and `退` require a color-prefixed name because the direction depends on the piece's color. A coordinate-identified piece may use only `平`, `直`, or an absolute destination. White has no forward direction and cannot use `进` or `退`.

### Suffixes and action intent

| Suffix | Intent | Destination must be |
|---|---|---|
| none | placement during the placement phase; ordinary movement during the movement phase | empty |
| `捉` | capture | occupied |
| `推` | push | occupied |
| `和` | draw action | occupied by an opposing Vital piece |
| `分` | divide, spending one White piece from the pool and leaving it at the origin | empty |

A suffix declares intent; it does not prove legality. For example, `红车三四捉` may be structurally valid, but whether it can be played still depends on control, movement, path, capture ability, and the target's effective abilities in the current position.

### Placement versus movement

The current phase determines the meaning of an unsuffixed expression:

- **Placement phase:** a color-prefixed name plus an absolute destination, such as `红车三四`, places that piece from Red's pool.
- **Movement phase:** an unsuffixed expression moves a piece already on the board to an empty point.

Expressions ending in `捉`, `推`, `和`, or `分` always describe movement-phase actions and cannot place a piece.

### Pass and resign

- `红按兵` / `黑按兵`: the current player passes without changing the board. Passing is allowed only in the movement phase.
- `红认负` / `黑认负`: the current player resigns and the opponent wins immediately. Resignation is allowed in either phase.

The color must match the player to move. Again, a phrase may parse successfully while being illegal in the current game.

### Action examples

Move the Red Advisor from column 2, row 3 to column 3, row 4:

```text
红士三四
```

Advance the Black General two steps:

```text
黑将进二
```

When two Red Rooks exist, move the one at column 1, row 2 horizontally to column 3:

```text
一二三二
```

Move the Red Commander straight to row 4 and divide:

```text
红军直四分
```

White has no forward direction, so it uses `平`, `直`, or an absolute
destination. For example, move a White piece to row 2:

```text
白子直二
```

```text
红按兵
黑认负
```

## Action results

An action result is either a successful reaction or a one-line error. A successful reaction always has two lines:

```text
变化：[{change entry} {change entry} ...]
胜负：{result}
```

### Change entries

Every entry is interpreted against the **same pre-action board snapshot**. Entries do not carry the action suffixes `捉`, `推`, `和`, or `分`:

- `piece + destination`: the piece now occupies that point. Only during the
  placement phase does a color-prefixed name plus an absolute destination mean
  placement from a pool; during the movement phase it means an on-board move.
- `piece + 失`: the piece leaves the board and no replacement occupies its origin.
- `piece + destination + 占`: explicitly states that a piece entered from off-board at that destination.

Ordinary movement and capture usually need only the arriving piece; an original occupant of the destination is overwritten and need not also receive a `失` entry. If both pieces leave the board, each receives its own `失` entry.

Entry order does not affect meaning. A reader first resolves every entry against the pre-action board, then combines the resulting clears and occupancies by point. If the same point is both cleared and occupied, occupancy wins. This permits cyclic moves and position swaps to be represented without applying entries sequentially.

The core `NotationResolver` is constructed with the complete **pre-action `Game`**, not only its board. The board resolves departures and arrivals; the current placement pool restores the exact removed piece and index, and the game's white-piece definition restores divide results. Therefore the unchanged text protocol can reconstruct a complete reversible `Reaction`.

Examples follow.

Ordinary movement or capture:

```text
变化：[红车平五]
胜负：未分
```

The pushing piece enters the target point and the target moves one step farther:

```text
变化：[红风平二 红车平三]
胜负：未分
```

A piece enters from a pool or another off-board source:

```text
变化：[红车四四占]
胜负：未分
```

During the placement phase, an unsuffixed change entry may also represent
placement from a pool:

```text
变化：[红车四四]
胜负：未分
```

Two pieces swap points. Both entries refer to the pre-action coordinates:

```text
变化：[一二三四 三四一二]
胜负：未分
```

The capturing piece and target both leave the board:

```text
变化：[红雷失 黑车失]
胜负：未分
```

Pass has no board changes:

```text
变化：[]
胜负：未分
```

An action may also decide the game:

```text
变化：[红车平二]
胜负：红胜
```

Resignation and an already-confirmed draw likewise have no board changes:

```text
变化：[]
胜负：红胜
```

```text
变化：[]
胜负：和棋
```

### Result line

The result is exactly one of:

- `未分`: the game continues;
- `红胜`: Red wins;
- `黑胜`: Black wins;
- `和棋`: draw.

### Errors

```text
错误：{single-line message}
```

`错误：` is the stable protocol prefix. The remaining message is for people, may change between versions, and must not be parsed by machines. An error reaction occupies one line and leaves the entire game state unchanged.

For example:

```text
错误：path blocked, cannot reach destination
```

## Game records

A game record stores an optional starting snapshot followed by actions in play order, providing a common interchange format for complete games.

- When the game does not start from the standard initial state, write the complete snapshot followed by one blank line.
- Each round occupies one line: an Arabic round number, a period, one space, then Red's and Black's half-moves.
- If the starting snapshot has Black to move, write `……` for the missing Red half-move.
- The final round may contain only one half-move, and a record may stop after any half-move.

Example:

```text
1. 红将五十 黑将五一
2. 红盾四十 黑车五二
```

## Appendix: grammar

The following is a simplified EBNF for the complete format. Terminals are quoted; `{ x }` means zero or more repetitions and `[ x ]` means optional.

```ebnf
game-state   = player-line , pool-line , pool-line , white-line ,
               result-line , board ;
player-line  = "行棋方：" , side , newline ;
pool-line    = ( "红方：[" | "黑方：[" ) ,
               [ name , { " " , name } ] , "]" , newline ;
white-line   = "白方：" , integer , newline ;
result-line  = "胜负：" , result , newline ;
board        = "棋盘：" , newline , header-row , { board-row } ;
header-row   = "零[" , column-label , { " " , column-label } , "]" , newline ;
column-label = numeral , "路" ;
board-row    = numeral , "[" , cell , { " " , cell } , "]" , newline ;
cell         = "一一" | ( color , name ) ;

action       = pass | resign | piece-action ;
pass         = side , "按兵" ;
resign       = side , "认负" ;
piece-action = piece , position , [ suffix ] ;
piece        = ( color , name ) | coordinate ;
coordinate   = numeral , numeral ;
position     = coordinate | relative ;
relative     = ( "平" | "直" | "进" | "退" ) , numeral ;
suffix       = "捉" | "推" | "和" | "分" ;

reaction     = success | error ;
success      = "变化：[" , [ change , { " " , change } ] , "]" , newline ,
               "胜负：" , result ;
change       = ( piece , "失" )
             | ( piece , position , "占" )
             | ( piece , position ) ;
error        = "错误：" , text ;

record       = [ game-state , newline ] , { round , newline } ;
round        = integer , ". " , half-move , [ " " , half-move ] ;
half-move    = action | "……" ;

side         = "红" | "黑" ;
color        = "红" | "黑" | "白" ;
result       = "未分" | "红胜" | "黑胜" | "和棋" ;
name         = "将" | "军" | "间" | "谍" | "车" | "卒" | "士" | "马"
             | "风" | "山" | "火" | "林" | "矛" | "盾" | "弹" | "雷"
             | "子" ;
numeral      = "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八"
             | "九" | "十" | "甲" | "乙" | "丙" | "丁" | "戊" | "己" ;
integer      = (* non-negative Arabic integer *) ;
text         = (* free-form text without a line break *) ;
newline      = "\n" | "\r\n" ;
```

The grammar does not capture every semantic constraint:

- `进` / `退` require a color-prefixed piece name and cannot be used for White.
- Placement requires a color-prefixed piece name and an absolute destination.
- A coordinate-identified piece cannot use `进` / `退`.
- Change entries never carry action suffixes; `占` only means entering from off-board, and `失` only means leaving without a replacement at the origin.
- `分` requires the piece to have the Divide ability in the current position and consumes one White piece from the pool.
- The final line may omit a trailing newline. These format details do not change whether an action is legal in the game.
