# Formation Chess — Text Notation

[简体中文](notation.zh-Hans.md) · [Game rules](rules.md)

All game state and actions in Formation Chess can be expressed as plain
text. This text protocol is the canonical format for saving games,
replaying matches, and communicating moves over any text channel.

It defines four kinds of text, described in order below: a **game state**
snapshot, an **action**, an **action result**, and a **game record**. A
formal grammar for all of them is collected in the
[appendix](#appendix-grammar).

The notation is written in Chinese characters, with two exceptions: the
white-piece count and the round numbers of game records use Arabic
numerals, and error messages are free-form text.


## Text Conventions

- Text is encoded in **UTF-8**.
- The colon after every label is the full-width `：`, never the ASCII `:`.
- Spacing is exact: a single space ` ` separates list entries; there is
  no other whitespace — no leading or trailing spaces, no blank lines
  except where a format explicitly calls for one.
- Readers accept both `\n` and `\r\n` line endings; writers should emit
  `\n`.


## Numerals and Coordinates

All coordinate numbers, row labels, and step counts use the same Chinese
numeral system:

| Value | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|-------|---|---|---|---|---|---|---|---|---|----|----|----|----|----|----|----|
| Char  | 一 | 二 | 三 | 四 | 五 | 六 | 七 | 八 | 九 | 十 | 甲 | 乙 | 丙 | 丁 | 戊 | 己 |

Values beyond ten use the auxiliary numerals `甲` through `己`, matching
the protocol's maximum board size of 16 columns by 16 rows. The standard
game is played on 9×10.

A position is written as **column then row**: `三二` means column 3,
row 2. Columns are numbered from `一` left to right; rows are numbered
from `一` top to bottom. Black plays from the top of the board (low row
numbers), Red from the bottom (high row numbers), so "forward" means up
the board for Red and down the board for Black.


## Game State

A complete game snapshot consists of six lines followed by a board grid:

```
行棋方：黑
红方：[炮 马]
黑方：[将 犬 盾]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
```

### Player Line

The first line names the side whose turn it is. It starts with `行棋方：`
followed by either `红` (Red) or `黑` (Black).

### Pool Lines

The next two lines list the pieces each player has not yet placed on the
board. Red's pool begins with `红方：[` and Black's with `黑方：[`. Piece
names are listed as single characters without the color prefix since the
pool already identifies the side. Names are separated by a space ` `.
An empty pool is written as `[]`.

### White Count

The fourth line shows how many white pieces are available for placement.
It takes the form `白方：` followed by an Arabic numeral.

### Result Line

The fifth line records the game result. It takes the form `胜负：`
followed by one of `未分` (unfinished), `红胜` (Red wins), `黑胜` (Black
wins), or `和棋` (draw). When the result is anything other than `未分`,
the game is over and no further actions are accepted.

### The Board

The board begins with the line `棋盘：`, followed by the grid itself.

The first row of the grid is a header row for the columns. It starts with
`零` (zero) in the row-label position and then lists each column in
`x路` format — for instance `一路 二路 三路`. The `路` suffix ensures that
the two-character piece names align neatly in every row. The `零` is
only a marker for the label row; columns and rows themselves are
numbered from `一`.

Each subsequent row starts with the Chinese numeral for the row index,
followed by the cells in brackets. Every cell is exactly two characters
wide. An empty point is `一一`. An occupied point shows the piece's
color prefix and its name as a single character, for example `红车` (Red
Rook) or `黑将` (Black General). White pieces appear as `白子`.

### Custom Setups and Validity

A snapshot need not start from the standard setup: it may describe any
board size up to 16×16 and any mid-game position, including positions
the standard game could never reach. The [game rules](rules.md)
describe the standard game; this section describes what any snapshot must
satisfy to be accepted.

On a board of non-standard height, the placement halves generalize as
follows: Red may place only on rows numbered greater than ⌈height/2⌉,
Black only on rows numbered at most ⌊height/2⌋. For boards with an odd
number of rows, the center row belongs to neither side and cannot be used
during placement.

A snapshot is rejected as invalid when it breaks the rules it claims to
be a position of:

- a side has more than one vital piece;
- pool sizes do not satisfy alternation (the player to move must have
  the same number of pieces in pool as the opponent, or exactly one
  more);
- during the placement phase (some pool non-empty), a piece stands
  outside its side's half;
- the recorded result is `未分` but the position is already decided:
  a side no longer owns its vital piece (on the board or in its pool),
  or neither side owns a vital piece (both were destroyed, which is a
  draw).


## Actions

Every action — whether placing a piece or moving one — is written as
`piece` + `position` + optional `suffix`. Two additional actions, pass
and resign, are written as standalone words (see below).

### Piece Identification

A piece is identified by its color prefix (`红` / `黑` / `白`) followed
by its single-character name. For instance `红车` is the Red Rook.

When a piece is unique on the board — there is only one piece of that
color and name — it can be referred to simply by its name. When there
are multiple pieces with the same color and name (e.g. two White pieces
on the board),
each must be identified by the coordinates of its current point (see
Position below).

### Position

A position is expressed in one of two ways.

**Absolute position** — column then row, using Chinese numerals. `三二`
means column 3, row 2.

**Relative position** — describes movement from the piece's current
location:

- `平` + column: move to a different column on the same row ("horizontal").
- `进` + steps: move forward (up for Red, down for Black).
- `退` + steps: move backward (down for Red, up for Black).
- `直` + row: move to a different row in the same column ("vertical").

`进` / `退` depend on the piece's color for direction and are only
available when the piece is identified by a color-prefixed name
(e.g. `红车进二`). When a piece is identified by coordinates
(e.g. `一二`), use `直` for vertical moves and `平` for lateral moves.
White pieces have no forward direction, so `进` / `退` do not apply to
them either — use `直` / `平` or an absolute position instead.

Steps use Chinese numerals ranging from 1 to 16, matching the maximum
board dimension.

**A note for Xiangqi players:** `平` / `进` / `退` work as they do in
Xiangqi notation, but the coordinate system differs. Both players use
the same absolute coordinates — columns are not counted from each
player's own right, and a piece is never named by its column alone.
Where Xiangqi disambiguates identical pieces with 前 / 后, this notation
uses the piece's coordinates instead, and it adds `直` for vertical
moves to an absolute row.

### Suffix — Declaring Intent

A suffix after the position makes the intent of the action explicit.

The two combat suffixes are used when the action targets an occupied
point; they declare how the moving piece interacts with the piece
already there:

- `推` (push): shove the target one step further in the same direction.
- `捉` (capture): remove the target from the board. A captured piece
  becomes a white piece added to the pool.
- `和` (draw): move your vital piece onto the opponent's vital piece,
  removing it and declaring a draw.

The placement suffix targets an empty point instead:

- `占` (place): explicitly place a piece onto the board. Used when the
  piece might otherwise be misinterpreted as a move — for example, a second
  white piece placed while one is already on the board.

If the suffix is omitted, the action must target an empty point (a simple
move or placement). Moving onto an occupied point without a push or
capture suffix is rejected.

### Placement vs. Movement

When a piece is not yet on the board and the written position is
absolute without a combat suffix, the game interprets the action as
placing that piece onto the given point.

If the piece is already on the board, the action is treated as a move.
Use the `占` suffix to explicitly mark a placement even when an
identical piece already stands on the board.

### Pass and Resign

Two actions consist of a color followed by a keyword, with no piece
name, position, or suffix. The color must match the player to move:

- color + `按兵` (pass): the player skips the turn without moving a
  piece — `红按兵` or `黑按兵`.
- color + `认负` (resign): the player concedes; the opponent wins
  immediately — `红认负` or `黑认负`.

Pass is not allowed during the placement phase: every placement
turn must place a piece. Resign may be used at any time. (Placing a
white piece is likewise forbidden during the placement phase.) Both
actions produce an empty change list.

### Examples

A Red Dog on column 2, row 3 moves diagonally to column 3, row 4:

```
红犬三四
```

A Black General advances two steps:

```
黑将进二
```

Two Red Rooks are on the board. The one on column 1, row 2 moves
to column 3 on the same row:

```
一二三二
```

Here `一二` identifies the piece by its current point (column 1, row 2).
Because the piece is identified by coordinates, only `平`, `直`, or
absolute positions may follow — `进` / `退` require a color-prefixed
name to determine the forward direction.

A white piece on column 3 moves vertically to row 2:

```
白子直二
```

The Wizard (`巫`) places a white piece from the pool onto column 2, row 4:

```
白子二四占
```

The `占` suffix makes the placement explicit. Without it the action could
be misinterpreted as a move once a white piece is already on the board.

Red passes the turn:

```
红按兵
```

Black resigns, handing Red the win:

```
黑认负
```


## Action Results

Every action produces either a success with board changes, or an error.

### Success

A successful action is written as two lines:

```
变化：[{entry} {entry} ...]
胜负：{outcome}
```

The first line begins with `变化：[` and ends with `]`. Inside the brackets,
a series of entries describes how the pieces changed. Entries are
separated by spaces.

Each entry describes the change of a single piece and is written like an
action — piece identification followed by a position — but never carries
a combat suffix (`捉` / `推`):

- **Piece + position** — the piece now stands on that point, for
  example `红车进一` or `黑马三四`. The position may be absolute or
  relative, exactly as in actions. If the piece was not on the board
  before the action, this is a placement onto the (absolute) point,
  marked with the `占` suffix.
- **Piece + `提`** — the piece is removed from the board and no new
  piece arrives on its former point, for example `红雷提`. A normal
  capture therefore needs no `提` entry for the captured piece: the
  attacker's own entry already shows a new piece arriving on that point.

Piece identification follows the same rules as actions: a unique piece
is written with its color prefix and name (`红车`); when several pieces
share the same color and name, the piece is identified by the
coordinates of the point it occupied before the action (`一二`).

Every entry is expressed relative to the board as it stood when the
action was executed. Because all entries reference that same snapshot,
the list is order-independent — entries may appear in any order.

To apply a change list, the receiver first converts the entries into
changes per position against that same board snapshot:

1. Every point a piece moved away from or was removed from becomes empty.
2. Every point a piece arrived on now holds that piece.
3. When both apply to the same point, the arrival wins.

The resulting position changes are then written onto the board. Because
arrivals take priority over departures, cyclic changes — such as two
pieces swapping points — are expressed naturally.

The second line begins with `胜负：` followed by one of:

- `未分` — the game continues.
- `红胜` — Red has won.
- `黑胜` — Black has won.
- `和棋` — the game is a draw.

### Error

```
错误：{message}
```

The line begins with `错误：` and the remainder is the error description.
The message must be a single line: an error reaction always occupies
exactly one line of text.

Only the `错误：` prefix is part of the protocol. The message text is
meant for humans during development and debugging, is not stable across
versions, and must not be parsed by machines.

### Examples

A simple move: the Red Rook shifts to column 5 on its row. A capture is
written the same way — the attacker simply arrives on the target's
point, so the captured piece needs no entry:

```
变化：[红车平五]
胜负：未分
```

Both attacker and target are destroyed (e.g. mine effect); neither
point receives a new piece:

```
变化：[红雷提 黑车提]
胜负：未分
```

A push: the pusher moves onto the target's point, the pushed piece
lands one step further:

```
变化：[红河平二 红车平三]
胜负：未分
```

Placement from the pool (the piece was not on the board before):

```
变化：[红车四四占]
胜负：未分
```

Two pawns swap points — a cyclic change. Both entries identify the
pieces by the points they occupied before the action:

```
变化：[一二三四 三四一二]
胜负：未分
```

An action that wins the game:

```
变化：[红车平二]
胜负：红胜
```

A pass — the board does not change:

```
变化：[]
胜负：未分
```

Black resigns (`黑认负`) — the board does not change, Red wins:

```
变化：[]
胜负：红胜
```

No action taken, but the board is already in a draw:

```
变化：[]
胜负：和棋
```

An illegal action:

```
错误：path blocked, cannot reach empty destination
```


## Game Records

A game record stores a whole game as its starting point plus the
sequence of actions played. Replaying the actions reproduces every
intermediate position and the final result, so nothing else needs to be
stored.

A record consists of:

- an optional game state block, followed by one blank line — present
  only when the game does not start from the standard setup (empty 9×10
  board, both armies in their pools, Red to move);
- one line per round, in play order.

Following the convention of chess and Xiangqi score sheets, each round
line contains the round number in Arabic numerals, a period and a space,
then the two half-moves of the round separated by a single space — Red's
action first, then Black's. Each half-move is an action exactly as
defined above. When the record starts from a snapshot where Black is to
move, the missing first half-move is written as `……`; the final round
may likewise contain only one half-move. A record may stop after any
half-move — an unfinished record is simply a valid prefix of a game.

The first rounds of a standard game (a record that starts with
placements):

```
1. 红将五十 黑将五一
2. 红盾四十 黑车五二
```

A record starting from the snapshot in [Game State](#game-state), where
Black is to move:

```
1. …… 黑将一一
2. 红炮二四 黑犬四一
```

(The engine currently parses states, actions, and results; reading and
writing game records is not implemented yet.)


## Appendix: Grammar

The complete notation in EBNF. Terminals are literal strings; `{ x }`
means zero or more repetitions, `[ x ]` means optional.

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
coordinate   = numeral , numeral ;              (* column, then row *)
position     = coordinate | relative ;
relative     = ( "平" | "直" | "进" | "退" ) , numeral ;
suffix       = "捉" | "推" | "和" | "占" ;

reaction     = success | error ;
success      = "变化：[" , [ change , { " " , change } ] , "]" , newline ,
               "胜负：" , result ;
change       = ( piece , "提" )
             | ( piece , position , [ "占" ] ) ;
error        = "错误：" , text ;

record       = [ game-state , newline ] , { round , newline } ;
round        = integer , ". " , half-move , [ " " , half-move ] ;
half-move    = action | "……" ;

side         = "红" | "黑" ;
color        = "红" | "黑" | "白" ;
result       = "未分" | "红胜" | "黑胜" | "和棋" ;
name         = "将" | "巫" | "间" | "谍" | "车" | "卒" | "犬" | "马"
             | "河" | "山" | "风" | "林" | "矛" | "盾" | "炮" | "雷"
             | "子" ;
numeral      = "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八"
             | "九" | "十" | "甲" | "乙" | "丙" | "丁" | "戊" | "己" ;
integer      = (* a non-negative number in Arabic digits *) ;
text         = (* free-form text, no line breaks *) ;
newline      = "\n" | "\r\n" ;
```

Constraints the grammar does not capture:

- `进` / `退` require the piece to be identified by a color-prefixed
  name, and never apply to white pieces.
- After a coordinate-identified piece, only `平`, `直`, or an absolute
  position may follow.
- A change entry never carries `捉`, `推`, or `和`; the `占` suffix appears
  only on placements, whose position is absolute.
- The final line of a text may omit the trailing newline.
