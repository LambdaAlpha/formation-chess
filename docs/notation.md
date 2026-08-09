# Formation Chess — Text Notation

[简体中文](notation.zh-Hans.md) · [Game rules](rules.md) · [Project overview](../README.md)

This specification defines the Chinese-character text protocol for complete
game snapshots, actions, action results, and line-oriented complete game
records.

## Format conventions

- Text is UTF-8.
- Fixed labels use the full-width colon `：`.
- List entries are separated by exactly one space, with no leading or trailing
  spaces.
- Line endings may be `\n` or `\r\n`; canonical text uses `\n`.
- Coordinates, row labels, and step counts use Chinese numerals.
- All snapshot and action coordinates are 1-based.

Structural validity does not imply legality in the stated position. A legal
action must also satisfy the phase, player, control, effective-ability, path,
target, and current-result requirements.

## Numerals and coordinates

| Value | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Character | 一 | 二 | 三 | 四 | 五 | 六 | 七 | 八 | 九 | 十 | 甲 | 乙 | 丙 | 丁 | 戊 | 己 |

A coordinate is always column first, then row. `三二` means column 3, row 2.
Columns increase left to right and rows increase top to bottom. Red advances
toward smaller row numbers; Black advances toward larger row numbers.

## Game snapshots

A complete snapshot contains four state lines, a `棋盘：` marker, and the board
grid:

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

### State lines

- `行棋方：红` or `行棋方：黑` identifies the player who acts next.
- `红方：[...]` and `黑方：[...]` contain the pieces not yet placed by that
  owner. Entries use one-character piece names separated by one space; an empty
  pool is `[]`.
- `胜负：未分`, `红胜`, `黑胜`, or `和棋` states the current game result.
- The snapshot has exactly the Red and Black placement-pool lines shown above.

### Board grid

The header starts with `零[` and contains one `numeral + 路` label per column.
Each board row starts with its numeral and contains one cell per column.

- Empty point: `一一`.
- Occupied point: owner prefix plus piece name, such as `红车` or `黑将`.

Every cell is two characters wide. Header width and row widths must agree, and
neither board dimension may exceed 16.

### Snapshot validation

A custom snapshot need not be reachable from the standard opening, but an
acceptable snapshot must satisfy all of these consistency rules:

- every Red pool entry is a Red piece and every Black pool entry is a Black
  piece;
- each owner has at most one Vital piece across its pool and board;
- when either pool is non-empty, every on-board piece lies in its owner's
  placement half;
- placement pool sizes and the declared next player/result are compatible with
  strict Red-then-Black alternation;
- an unfinished snapshot retains a Vital piece for both owners;
- a declared Red or Black win retains a Vital piece for the winner; and
- a draw cannot be declared while the game is still in placement.

On an odd-height board, the center row belongs to neither placement half.

## Actions

Most actions use:

```text
piece reference + destination + optional suffix
```

Resign uses a dedicated phrase.

### Piece references

A board piece may be named with its owner prefix, such as `红车` or `黑将`, when
that identity occurs exactly once on the board. If duplicate identities exist,
use the pre-action coordinate of the acting piece, such as `一二`.

Placement must name a pool piece with an owner-prefixed identity. A coordinate
cannot refer to a piece that is still outside the board.

### Destinations

An absolute destination is a column numeral followed by a row numeral, such as
`三二`.

A relative destination uses one of four operators:

- `平` + column: move to that column on the same row;
- `直` + row: move to that row in the same column;
- `进` + steps: move forward for the named piece's owner;
- `退` + steps: move backward for the named piece's owner.

`进` and `退` require an owner-prefixed piece name because direction depends on
the piece owner. A coordinate-identified piece may use only an absolute
destination, `平`, or `直`.

### Suffixes

| Suffix | Action | Destination |
|---|---|---|
| none | placement in the placement phase; ordinary move in the movement phase | empty |
| `捉` | capture | occupied |
| `推` | push | occupied |
| `拉` | pull the piece behind the origin | empty |
| `和` | exchange with an opposing Vital piece and draw | occupied |

There is no `分` suffix. A suffix declares intent but does not prove legality.

### Placement and movement examples

```text
红将五十
红士三四
黑车进二
一二三二
红马四五捉
红火五四推
红风平四拉
红将五一和
```

The same unsuffixed `红将五十` form means placement when the named piece is in
the current pool, and an ordinary move when the game is already in the movement
phase and the piece is uniquely present on the board.

### Resign

```text
黑将认负
```

- `认负` must name a General. During movement, it must uniquely identify an
  on-board Vital piece controlled by the current player; the named piece's owner
  loses. During placement, the phrase must name the current player's General,
  and the current player loses directly.

## Action results

An action result is either a successful two-line block or a one-line error.

```text
变化：[{change entry} {change entry} ...]
胜负：{result}
```

### Change entries

Every change entry is interpreted against the same complete **pre-action
position**. Entries never carry the action suffixes `捉`, `推`, `拉`, or
`和`.

- `piece + destination`: move an on-board piece. During placement only, a named
  piece plus an absolute destination may also mean placement from the pool.
- `piece + destination + 占`: explicitly place a piece arriving from off-board.
- `piece + 失`: remove a piece without a replacement at its origin.

Departures and arrivals are interpreted together and combined by board point.
If a point is both vacated and occupied, occupancy takes precedence. Swaps and
cycles therefore do not depend on entry order.

All board and pool references use the pre-action position. During placement, a
named off-board arrival must match a piece in the corresponding pre-action pool.

### Examples

Ordinary movement or capture:

```text
变化：[红车平五]
胜负：未分
```

A push moves the active piece and the target:

```text
变化：[红火平二 红车平三]
胜负：未分
```

A pull can change the active destination, the vacated origin, and the pulled
source:

```text
变化：[红卒平三 红风平四]
胜负：未分
```

An explicit off-board arrival uses `占`:

```text
变化：[红车四四占]
胜负：未分
```

During placement, the unsuffixed form is also accepted for a pool arrival:

```text
变化：[红车四四]
胜负：未分
```

A Draw action swaps the acting piece with an opposing Vital piece and ends the
game as a draw. For example, in the position below 红势 at 二二 has gained the
draw ability from 红将's formation and swaps with 黑将 at 四四:

```text
棋盘：
零[一路 二路 三路 四路 五路]
一[红将 一一 一一 一一 一一]
二[一一 红势 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 一一 一一 黑将 一一]
五[一一 一一 一一 一一 一一]
```

After `红势四四和`, the two pieces exchange positions; every entry is
interpreted against the pre-action position:

```text
变化：[黑将二二 红势四四]
胜负：和棋
```

Mutual destruction removes both pieces:

```text
变化：[红雷失 黑计失]
胜负：未分
```

A winning action or resignation declares the resulting win:

```text
变化：[红车平二]
胜负：红胜
```

```text
变化：[]
胜负：红胜
```

A draw result with no position changes is also structurally valid:

```text
变化：[]
胜负：和棋
```

### Result values

- `未分`: unfinished;
- `红胜`: Red wins;
- `黑胜`: Black wins;
- `和棋`: draw.

Once a game result is not `未分`, no further action is legal.

### Errors

```text
错误：{single-line message}
```

Only the `错误：` prefix is a stable protocol marker. The remaining
human-readable text has no stable protocol semantics. An error leaves the board,
pools, player, and result unchanged.

```text
错误：path blocked, cannot reach destination
```

## Game-record convention

A complete record may contain an optional starting snapshot followed by actions
in play order.

- For a non-standard start, write the complete snapshot followed by one blank
  line.
- Each round uses an Arabic number, `. `, Red's half-move, and optionally Black's
  half-move.
- If the starting snapshot has Black to act, write `……` for the absent Red
  half-move.
- The final round may contain only one half-move, and a record may stop after any
  half-move.

```text
1. 红将五十 黑将五一
2. 红盾四十 黑车五二
```

## Simplified grammar

```ebnf
game-state   = player-line , pool-line , pool-line , result-line , board ;
player-line  = "行棋方：" , side , newline ;
pool-line    = ( "红方：[" | "黑方：[" ) ,
               [ name , { " " , name } ] , "]" , newline ;
result-line  = "胜负：" , result , newline ;
board        = "棋盘：" , newline , header-row , { board-row } ;
header-row   = "零[" , column-label , { " " , column-label } , "]" , newline ;
column-label = numeral , "路" ;
board-row    = numeral , "[" , cell , { " " , cell } , "]" , newline ;
cell         = "一一" | named-piece ;

action       = resign | piece-action ;
resign       = named-piece , "认负" ;
piece-action = piece , position , [ suffix ] ;
piece        = named-piece | coordinate ;
named-piece  = side , name ;
coordinate   = numeral , numeral ;
position     = coordinate | relative ;
relative     = ( "平" | "直" | "进" | "退" ) , numeral ;
suffix       = "捉" | "推" | "拉" | "和" ;

action-result = success | error ;
success      = "变化：[" , [ change , { " " , change } ] , "]" , newline ,
               "胜负：" , result ;
change       = ( piece , "失" )
             | ( named-piece , coordinate , "占" )
             | ( piece , position ) ;
error        = "错误：" , text ;

record       = [ game-state , newline ] , { round , newline } ;
round        = integer , ". " , half-move , [ " " , half-move ] ;
half-move    = action | "……" ;

side         = "红" | "黑" ;
result       = "未分" | "红胜" | "黑胜" | "和棋" ;
name         = "将" | "计" | "势" | "变" | "风" | "林" | "火" | "山"
             | "矛" | "盾" | "弹" | "雷" | "士" | "卒" | "马" | "车" ;
numeral      = "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八"
             | "九" | "十" | "甲" | "乙" | "丙" | "丁" | "戊" | "己" ;
integer      = (* non-negative Arabic integer *) ;
text         = (* free-form text without a line break *) ;
newline      = "\n" | "\r\n" ;
```

The grammar omits semantic constraints such as board bounds, uniqueness,
control, phase, legal movement, and effective abilities. The final line may omit
a trailing newline.
