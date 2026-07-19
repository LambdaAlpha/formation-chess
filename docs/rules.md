# Formation Chess — Game Rules

[简体中文](rules.zh-Hans.md) · [Project overview](../README.md) · [Text notation](notation.md)

This is the complete rulebook for the standard game of Formation Chess.

- [The Idea](#the-idea)
- [If You Know Xiangqi](#if-you-know-xiangqi)
- [The Basics](#the-basics)
- [Abilities](#abilities)
- [Formations](#formations)
- [Moving a Piece](#moving-a-piece)
- [Push, Pass-Through, and Capture](#push-pass-through-and-capture)
- [The Pieces](#the-pieces)
- [White Pieces](#white-pieces)
- [How a Game Unfolds](#how-a-game-unfolds)
- [How the Game Ends](#how-the-game-ends)
- [Strategic Notes](#strategic-notes)
- [Glossary](#glossary)

## The Idea

In most chess games, a piece is what its name says it is. A bishop moves
diagonally; a knight leaps; these are fixed identities, and the game
unfolds through their interplay. Formation Chess takes a different
approach. Every piece projects a **formation** — a personal zone of
influence covering the area around it. Pieces that stand inside another
piece's formation have their capabilities temporarily reshaped: they may
gain new powers or lose ones they normally possess.

This means no piece is ever just itself. A pawn standing near a rook
suddenly moves at unlimited range. A general surrounded by enemies who
have lost the ability to capture becomes unassailable. The board is alive
with shifting influence, and every move reconfigures the local web of
power.

There are no rules tied to piece names. Instead, every mechanic is an
**ability**. The horse does not "move in an L-shape because it is a
horse" — it merely carries the L-shaped movement ability by default, and
the cannon carries jump capture. Any piece standing in a horse's formation
can be granted that same L-shaped movement, and a cannon that strays into
an enemy cannon's formation loses its jump capture. The rules only care
about abilities, not names.

## If You Know Xiangqi

Formation Chess borrows the 9×10 board of Xiangqi and many familiar piece
names — the rook, horse, cannon, pawn, and general — along with two
signature mechanics: the horse's leg can be blocked, and the cannon
captures by jumping over a screen piece.

Everything else is different. There is no palace, no river, and no fixed
starting position — you place your army yourself. Pieces have no fixed
powers: what a piece can do is decided by the formations it stands in.
There is no check or checkmate — the general is captured like any other
piece, and losing it loses the game. Captured pieces are not gone for
good: they turn white and can be brought back to the board by either
side's wizard.

## The Basics

The game is played on a **9 by 10 board** between two players, **Red** and
**Black**. Each player commands **16 unique pieces**. There is also a
third color, **White**, for captured pieces that can be redeployed (see
[White Pieces](#white-pieces)).

A position on the board is written as a (column, row) pair: columns are
numbered 1–9 from left to right, rows 1–10 from top to bottom. Black plays
from the top of the board (rows 1–5), Red from the bottom (rows 6–10).

A game has two phases: first both players **place** all their pieces on
the board, then they **move** them. **Red acts first**, and the players
strictly alternate, one action per turn, from the first placement to the
end of the game.

## Abilities

Every piece has a set of **abilities** that determine what it can do.
These are not static — they can be altered by formations. The full list:

| Ability | Effect |
|---|---|
| **Control (Red / Black)** | Determines which player may issue commands for this piece. A piece must be controlled by you for you to move it. |
| **Push Ally** | Can shove a friendly piece one step farther along the movement direction. |
| **Push Enemy** | Can shove an enemy piece one step farther along the movement direction. |
| **Pushed by Ally** | Can be shoved by friendly pieces. |
| **Pushed by Enemy** | Can be shoved by enemy pieces. |
| **Pass Ally** | Can move through points occupied by friendly pieces. |
| **Pass Enemy** | Can move through points occupied by enemy pieces. |
| **Passed by Ally** | Can be moved through by friendly pieces. |
| **Passed by Enemy** | Can be moved through by enemy pieces. |
| **Capture** | Can capture (remove) an enemy piece by moving onto its point. |
| **Captured** | Can be captured by an enemy piece. |
| **Capture on Captured** | When this piece is captured, the capturer also dies. A mutual destruction effect (retaliation). |
| **Captured on Capture** | When this piece captures another, it dies as well. A mutual destruction effect (sacrifice). |
| **Jump Capture** | Can capture an enemy piece by jumping over exactly one intervening piece (the cannon mechanic). |
| **Any Distance** | Can slide any number of steps along an allowed direction instead of just one. |
| **Direction: Cross** | Can move horizontally or vertically. |
| **Direction: Diagonal** | Can move diagonally. |
| **Direction: L-shaped** | Can move in an L-shaped pattern (one step orthogonally plus one step diagonally, the knight's move). |
| **Control White** | Can place white pieces from the pool onto empty points inside its own formation. Commanding the placed pieces is granted separately, by the wizard's formation effect. |
| **Vital** | Losing this piece means losing the game. The general carries it. |

## Formations

Every piece projects a **formation** over the 3×3 area centered on itself.
Pieces that stand inside this area may have their abilities altered. But
the formation is not a uniform field: only certain positions within it are
active. Which positions depends on the piece's **formation pattern**.

There are four patterns:

```
   Corners                     Edges
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+
|   | O |   |              | X | O | X |
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+

Upper Triangle             Lower Triangle
+---+---+---+              +---+---+---+
|   | X |   |              | X |   | X |
+---+---+---+              +---+---+---+
|   | O |   |              |   | O |   |
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+
```

`O` marks the formation's owner standing at the center; `X` marks the
active positions. The diagrams show the canonical orientation for Red,
who sits at the bottom of the board with its advance direction toward
the top. Black sits at the top and advances toward the bottom, so its
formations are vertically mirrored: Black's upper-triangle pattern points
downward, its lower-triangle pattern points upward.

Each pattern belongs to one group of four pieces:

- **Corners** — Control group: General, Wizard, Traitor, Spy
- **Edges** — Movement group: Rook, Pawn, Dog, Horse
- **Upper Triangle** — Push & pass group: River, Mountain, Wind, Forest
- **Lower Triangle** — Capture group: Spear, Shield, Cannon, Mine

Think of it this way: a piece's influence reaches out in a specific
**shape**. A corners piece influences diagonally — like a general
surveying the battlefield from the flanks. An edges piece influences
orthogonally, like infantry covering the cardinal approaches. The triangle
patterns are asymmetric: upper triangle covers the front-center and rear
wings; lower triangle covers the front wings and rear-center. "Front" is
always the owner's advance direction — toward the opponent.

**How overlapping formations combine:** when multiple formations try to
modify the same ability on the same piece, the results are combined with
the rule that **any formation that disables an ability wins**. If even one
formation strips a power away, no other formation can restore it.
Disabling is absolute.

## Moving a Piece

A piece can only move along directions enabled by its **Direction**
abilities. There are three direction types: cross (horizontal / vertical),
diagonal, and L-shaped (the knight's move).

By default, movement is limited to a **single step** per direction:

- Cross: one point left, right, up, or down.
- Diagonal: one point diagonally.
- L-shaped: one knight move.

With the **Any Distance** ability, a piece may slide any number of points
along an allowed direction. For L-shaped moves, this means chaining
consecutive knight moves along the same diagonal line.

A piece must have at least one direction ability to move at all. Enemy
formations can strip direction abilities — stray into hostile territory
without friendly support and you may find yourself immobilized. **Keep
your formation network intact** so allies cover each other.

## Push, Pass-Through, and Capture

These three interactions are governed by matched pairs of abilities.

**Push** means shoving the target piece one step farther along the
direction of your movement. The shove is external force: the pushed piece
is treated as if it had the required direction ability, so its own
movement abilities are irrelevant. The push fails only when the pushed
piece cannot physically make that step — the landing point is occupied or
off the board, or its path there is blocked by a piece it cannot pass
through — in which case the push escalates into a capture attempt (the
target must have *Captured* for the capture to succeed; otherwise the push
is blocked).

- To push a **friendly** piece: you need *Push Ally* **or** the target
  needs *Pushed by Ally*. Either side's consent is enough.
- To push an **enemy** piece: you need *Push Enemy* **and** the target
  needs *Pushed by Enemy*. Both must agree.

**Pass-through** means treating an intervening piece as if it weren't
there while moving past it. You cannot stop on an occupied point, however.

- To pass through a **friendly** piece: you need *Pass Ally* **or** the
  blocker needs *Passed by Ally*.
- To pass through an **enemy** piece: you need *Pass Enemy* **and** the
  blocker needs *Passed by Enemy*.

**Normal capture** means moving onto an enemy-occupied point and removing
that piece. You need *Capture*, the target needs *Captured*, and your
colors must differ. The path must be clear (all intervening pieces must be
passable).

**Jump capture** (the cannon) allows capturing by jumping over exactly one
intervening piece. You need *Jump Capture*, the target needs *Captured*,
and there must be exactly one piece on the path. The intervening piece's
pass-through status is irrelevant — it acts as a screen, not something you
pass.

**Mutual destruction:** after any capture, two effects may trigger:

- If the target had *Capture on Captured*, the capturer is removed as well.
- If the capturer had *Captured on Capture*, the capturer is removed as well.

Both effects apply to any piece standing inside a formation that grants
them, not just the piece that carries them by default.

### L-Shaped Movement: Special Cases

Because an L-shaped move jumps through an intermediate point rather than
sliding across the board, three mechanics work slightly differently:

**Blocking (hobbling the horse's leg).** An L-shaped move passes through
an intermediate point on its way to the destination. For instance, moving
from (1,1) to (2,3) passes through (1,2). If that point is occupied by a
piece you cannot pass through, the move is blocked — the horse's leg is
hobbled. The pass-through rules described above determine whether you can
clear the blocking piece.

**Pushing with an L-shaped move.** When you shove a target via an
L-shaped move, the target is pushed one knight step further in the same
direction — from (1,1) via (2,3) to (3,5). The pushed piece makes this
step as though moving on its own: no L-shaped ability is required, but
its own leg-blocking rules apply. If the leg is blocked by a piece the
pushed piece cannot pass through, or the destination is occupied, the
push escalates to a capture as usual.

**Jump capture with an L-shaped move.** A cannon can use its jump capture
through an L-shaped path: it jumps over the leg-blocking piece to reach
the target beyond. The blocking piece serves as the screen, just as it
would for a straight-line jump capture.

## The Pieces

Each side has 16 distinct pieces organized into four groups by formation
pattern.

By default every Red and Black piece has *Pushed by Enemy*, *Passed by
Enemy*, and *Captured*; the only exceptions are the **Mountain** (not
pushable by enemies) and the **Forest** (not passable by enemies). In
other words, most pieces can be pushed, passed through, and captured by
the enemy unless a formation changes those settings.

### Control Group — Corners Pattern

These pieces move in cross directions at any distance. Their formations
deal with **who controls what**.

| Piece | Symbol | Default Abilities | Formation Effect |
|---|---|---|---|
| **General** | 将 | Cross, any distance, *Vital* | Modifies no abilities — but see below |
| **Wizard** | 巫 | Cross, any distance, *Control White* | White pieces inside become controlled by the wizard's player |
| **Traitor** | 叛 | Cross, any distance | Enemies inside become **also** controlled by the traitor's player (converts them) |
| **Spy** | 谍 | Cross, any distance, controlled by *both* players | Friends inside become **also** controlled by the opponent (double agent effect) |

The **General** is your vital piece — lose your general and you lose. It
moves freely but cannot capture, push, or pass through other pieces, so
protect it with formations. Its own formation modifies no abilities, but
its pattern still matters: if the two generals ever stand inside each
other's formation, the game is drawn (see
[How the Game Ends](#how-the-game-ends)).

The **Wizard** brings captured pieces back into play: it places white
pieces onto empty points inside its formation, and white pieces standing
inside its formation obey the wizard's player. See
[White Pieces](#white-pieces).

### Movement Group — Edges Pattern

These pieces have their named movement direction plus the ability to
capture. Their formations **grant movement abilities to allies and strip
them from enemies**.

| Piece | Symbol | Default Abilities | Formation Effect |
|---|---|---|---|
| **Rook** | 车 | Cross, any distance, capture | Allies gain any distance; enemies lose it (stuck at one step) |
| **Pawn** | 卒 | Cross, capture | Allies gain cross movement; enemies lose it |
| **Dog** | 犬 | Diagonal, capture | Allies gain diagonal movement; enemies lose it |
| **Horse** | 马 | L-shaped, capture | Allies gain L-shaped movement; enemies lose it |

These pieces form the backbone of your army's mobility, and they support
each other: except for the Rook they move one step at a time, but standing
next to a friendly Rook they gain unlimited range. The Rook's formation
also pins: any enemy piece inside it loses the ability to slide.

### Push & Pass Group — Upper Triangle Pattern

These pieces move in cross directions at any distance and specialize in
**push and pass-through mechanics**.

| Piece | Symbol | Default Abilities | Formation Effect |
|---|---|---|---|
| **River** | 河 | Cross, any distance, *Push Ally*, *Push Enemy* | Allies gain both push abilities; enemies lose both |
| **Mountain** | 山 | Cross, any distance, *Pushed by Ally*, **not** *Pushed by Enemy* | Allies become pushable by allies only; enemies become pushable by the mountain's side only |
| **Wind** | 风 | Cross, any distance, *Pass Ally*, *Pass Enemy* | Allies gain both pass abilities; enemies lose both |
| **Forest** | 林 | Cross, any distance, *Passed by Ally*, **not** *Passed by Enemy* | Allies become passable by allies only; enemies become passable by the forest's side only |

River and Wind empower the pieces around them to act — to shove and to
move through; Mountain and Forest instead take full control of how the
pieces around them can be acted upon, rewriting both "pushed by" (or
"passed by") abilities at once, for allies and enemies alike.

### Capture Group — Lower Triangle Pattern

These pieces move in cross directions at any distance and specialize in
**capture and combat**.

| Piece | Symbol | Default Abilities | Formation Effect |
|---|---|---|---|
| **Spear** | 矛 | Cross, any distance (no capture!) | Allies gain capture; enemies lose it |
| **Shield** | 盾 | Cross, any distance | Allies become uncapturable (immune); enemies become capturable |
| **Cannon** | 炮 | Cross, any distance, jump capture | Allies gain jump capture; enemies lose it |
| **Mine** | 雷 | Cross, any distance, *Capture on Captured*, *Captured on Capture* | Everyone inside gains both effects — any capture becomes mutual destruction |

The **Spear** cannot capture by default: its formation grants capture to
allies and strips it from enemies. The **Shield** protects everyone around it —
stack immunity on your general by keeping a Shield nearby. The **Cannon**
can only take pieces by jump capture, so it always needs a screen piece
between itself and the target. The **Mine** turns its surroundings into a
no-go zone where nobody wants to initiate an attack.

## White Pieces

When any piece is taken off the board by combat — captured normally, or
destroyed by the mutual-destruction effects — it becomes a **white piece**
in a shared pool of available dead. White pieces have only cross movement
(one step) and can be captured by anyone. They cannot capture on their
own.

A piece with the **Control White** ability (the Wizard) can place white
pieces from the pool onto any empty point within its formation. White
pieces carry no control of their own: they are commanded through the
wizard's formation — a white piece standing inside a wizard's formation
can be moved by that wizard's player (by both players, if wizards of both
sides cover it); outside every wizard's formation a white piece cannot be
moved. Capturing a white piece returns it to the pool — white pieces are
recycled, not destroyed.

This adds a third dimension to the game: captured enemies become white
pieces that your Wizard can bring back onto the board. White pieces do
not benefit from formation effects, so their combat strength is far
weaker than in their original form — they are primarily useful for
occupying space and blocking paths.

## How a Game Unfolds

**Placement phase.** The game begins with an empty board. Starting with
Red, the players alternate placing one of their own pieces per turn onto
any empty point in their half of the board: Red places on rows 6–10,
Black on rows 1–5. Passing is not allowed, and white
pieces cannot be placed — every turn must place one of the player's 16
pieces. Resigning is permitted at any time. No piece may move until both
players have placed all their pieces. This is your one chance to design
the initial formation web —
position your pieces so they support each other's abilities.

**Movement phase.** Once all 32 pieces stand on the board, the players
keep alternating turns (Red first again, since Black placed last). On each
turn a player takes exactly one of these actions:

- **Move** a piece they control to an empty point.
- **Capture:** move onto an occupied point, declaring the intent to take
  the piece there.
- **Push:** move onto an occupied point, declaring the intent to shove it.
  If the shove is blocked, it becomes a capture attempt automatically.
- **Place** a white piece through a wizard (this consumes the turn).
- **Pass:** skip the turn without moving. Because passing is always
  available, having no legal move never forces a loss — there is no
  stalemate defeat. Repeated mutual passing has no automatic outcome;
  settling such a standoff is up to the players.
- **Resign:** concede the game, immediately ending it with a win for the
  opponent.

Note that when moving onto an occupied point, the intent — capture or
push — must always be declared as part of the action.

## How the Game Ends

The game carries a persistent **result**: unfinished, Red wins, Black
wins, or draw. Once the result is anything other than unfinished, no
further actions are accepted.

After every successful action — placements included — the board is
checked:

- If **neither** side has any piece with the *Vital* ability (both vital
  pieces perished in the same action, e.g. through mutual destruction)
  → **draw**.
- If Red has **no** pieces with the *Vital* ability → Black wins.
- If Black has **no** pieces with the *Vital* ability → Red wins.
- If both sides still have their vital piece, and the two vital pieces
  are standing inside each other's formation → **draw** (the two generals
  acknowledge each other's position and agree to a truce).
- Otherwise → play continues.

A vital piece still waiting to be placed counts as alive. Because the
check also runs during the placement phase, a draw can already arise from
placements — for example, both generals placed inside each other's
formation across the border.

Resigning sets the result directly to the opponent's win, bypassing the
board check.

## Strategic Notes

1. **Formations move with pieces — root them together.** Influence on the
   board is never static: every move reshapes the formation web around it.
   Build a mobile network — one where allies cover each other but can still
   advance together, and critical positions have rotating backup coverage.
   Positioning determines who covers whom, and where the weak spots are.

2. **Position and pass through — control the battlefield.** Position
   pieces to block key routes and divide the enemy; use pass-through to
   bridge gaps and connect allies. The two are complementary: who can
   pass through a chokepoint depends on positioning; whether a position
   matters depends on what can pass through.

3. **Pushing breaks formations.** Pushing and capturing serve different
   purposes: capturing removes the enemy; pushing displaces them. A
   well-placed shove can throw a key piece out of friendly formation
   coverage, force it into bad terrain, or sever the opponent's formation
   connections. Pushing does not eliminate the piece, but it can dismantle
   the network that piece depends on.

4. **Read the tide — play the position, not the score.** When ahead, stay
   disciplined: one reckless charge beyond friendly formation coverage can
   turn your strongest piece into dead weight. When behind, don't give up:
   the double-general draw is always within reach. Use positioning and
   pushing to stall the opponent and create complications. A draw is closer
   than it looks in this game.

## Glossary

- **Formation** — the zone of influence a piece projects over the 3×3
  area centered on itself; it alters the abilities of pieces standing on
  its active positions.
- **Formation pattern** — the shape that decides which of the eight
  surrounding positions are active: corners, edges, upper triangle, or
  lower triangle.
- **Ability** — a single capability a piece may have or lack, such as a
  movement direction, capture, or being capturable. All rules operate on
  abilities.
- **Ally / Enemy** — pieces of the same color as, or the opposing player's
  color to, the piece in question. White pieces are nobody's ally.
- **Pool** — where pieces wait off the board: each player's own pieces
  before placement, and the shared pool of white pieces.
- **White piece** — a captured piece, recycled into the shared pool;
  either side's wizard can bring it back to the board.
- **Vital piece** — a piece whose ability *Vital* makes it a win
  condition: a player with no vital pieces left has lost. Normally the
  general.
- **Screen** — the single intervening piece a jump capture leaps over.
- **Placement phase / Movement phase** — the two stages of a game: first
  all pieces are placed, then they move.
