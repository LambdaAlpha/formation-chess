# Formation Chess — Game Rules

[简体中文](rules.zh-Hans.md) · [Project overview](../README.md) · [Text notation](notation.md)

This is the complete rulebook for the standard game of Formation Chess.

- [The Idea](#the-idea)
- [If You Know Xiangqi](#if-you-know-xiangqi)
- [The Basics](#the-basics)
- [Abilities](#abilities)
- [Formations](#formations)
- [Moving a Piece](#moving-a-piece)
- [Push and Capture](#push-and-capture)
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
horse" — it merely carries the L-shaped movement ability by default. Any
piece standing in a horse's formation can be granted that same L-shaped
movement. The rules only care about abilities, not names.

## If You Know Xiangqi

Formation Chess borrows the 9×10 board of Xiangqi and many familiar piece
names — the rook, horse, pawn, general, and scholar — along with one
signature mechanic: the horse's leg can be blocked.

Everything else is different. There is no palace, no river, and no fixed
starting position — you place your army yourself. Pieces have no fixed
powers: what a piece can do is decided by the formations it stands in.
There is no check or checkmate — the general is captured like any other
piece, and losing it loses the game. Captured pieces are not gone for
good: they turn white and can be brought back to the board by either
side's army.

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
| **Push to Capture** | When this piece pushes a target and the target cannot be shoved to its landing point (the point is occupied, off the board, or the path from the target's current point to the landing point is blocked), the push becomes a capture instead — the target is destroyed regardless of its abilities or color. |
| **Pushed to Captured** | When this piece is pushed and the push is blocked (the pushed landing point is invalid), the pusher captures this piece. |
| **Capture to Push** | When this piece captures a target and the path from the mover to the target is clear of intervening pieces, the capture becomes a push instead — the target is shoved one step farther along the movement direction rather than removed. |
| **Captured to Pushed** | When this piece is captured and the path from the attacker to this piece is clear, this piece is shoved instead of captured. |
| **Capture** | Can capture (remove) an enemy piece by moving onto its point. |
| **Captured** | Can be captured by an enemy piece. |
| **Capture on Captured** | When this piece is captured, the capturer is removed as well. Also makes this piece capturable even by pieces without CAPTURE. |
| **Captured on Capture** | When this piece captures another, it is removed as well. Also allows capturing targets even without CAPTURED. |
| **Any Distance** | Can slide any number of steps along an allowed direction instead of just one. |
| **Direction: Cross** | Can move horizontally or vertically. |
| **Direction: Diagonal** | Can move diagonally. |
| **Direction: L-shaped** | Can move in an L-shaped pattern (one step orthogonally plus one step diagonally, the knight's move). |
| **Divide** | Move to an empty point and leave a white piece behind at the original position (divide forces). Commanding the left-behind white pieces is granted separately, by the army's, agent's, or spy's formation effects. |
| **Vital** | Losing this piece means losing the game. The general carries it. |
| **Draw** | Can move onto an opponent's vital piece, removing it and ending the game in a draw. Granted to allies by the General's formation. |

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

- **Corners** — Control group: General, Army, Agent, Spy
- **Edges** — Movement group: Rook, Pawn, Scholar, Horse
- **Upper Triangle** — Push group: Wind, Mountain, Fire, Forest
- **Lower Triangle** — Capture group: Spear, Shield, Shell, Mine

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

**White pieces are neutral.** For formation effects, a white piece is
neither an ally nor an enemy. No formation modifies a white piece's
abilities, except for the control explicitly granted by the Army, Agent,
and Spy. Movement, pushing, and capture involving white pieces still use
the white piece's own abilities.

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

## Push and Capture

These interactions are governed by matched pairs of abilities.

**Push** means shoving the target piece one step farther along the
direction of your movement. The shove is external force: the pushed piece
is treated as if it had the required direction ability, so its own
movement abilities are irrelevant.

- To push a **friendly** piece: you need *Push Ally* **or** the target
  needs *Pushed by Ally*. Either side's consent is enough.
- To push an **enemy** piece: you need *Push Enemy* **and** the target
  needs *Pushed by Enemy*. Both must agree.

**When a push is blocked.** The push is blocked when the pushed piece
cannot land — its landing point is occupied, off the board, or the path
from the target's current point to the landing point contains a blocking
piece. Normally a blocked push fails with an error. However, if the pusher
has *Push to Capture* **or** the target has *Pushed to Captured*,
the push becomes a capture: the pusher takes the target's point and
destroys the target, regardless of the target's abilities or color.
This is the Fire formation's domain.

**Normal capture** means moving onto an enemy-occupied point and removing
that piece. You need *Capture*, the target needs *Captured*, and your
colors must differ. The path from the mover to the target must be clear of
intervening pieces.

**When a capture becomes a push.** If the path from the mover to the
target is clear (no intervening pieces) and the attacker has *Capture to
Push* **or** the target has *Captured to Pushed*,
the capture becomes a push instead: the attacker shoves the target one
step farther rather than removing it. If the shove itself is blocked (the
pushed landing point is invalid), the action falls back to a normal
capture — the push attempt does not cause an error. This is the Forest
formation's domain.

**Mutual destruction:** after a capture, two effects may trigger:

- If the target had *Capture on Captured*, the capturer is removed as well.
  This effect also makes the target capturable even by pieces without
  CAPTURE.
- If the capturer had *Captured on Capture*, the capturer is removed as
  well. This effect also allows capturing targets even without CAPTURED.

Both effects apply to any piece standing inside a formation that grants
them, not just the piece that carries them by default.

### L-Shaped Movement: Special Cases

Because an L-shaped move jumps through an intermediate point rather than
sliding across the board, two mechanics work slightly differently:

**Blocking (hobbling the horse's leg).** An L-shaped move passes through
an intermediate point on its way to the destination. For instance, moving
from (1,1) to (2,3) passes through (1,2). If that point is occupied, the
move is blocked — the horse's leg is hobbled. Any piece on the leg blocks
the move.

**Pushing with an L-shaped move.** When you shove a target via an
L-shaped move, the target is pushed one knight step further in the same
direction — from (1,1) via (2,3) to (3,5). The pushed piece makes this
step as though moving on its own: no L-shaped ability is required, but
its own leg-blocking rules apply. If the leg is blocked or the landing
point is occupied, the push is blocked and may become a capture if either
piece has the relevant push-blocked ability (see above).

## The Pieces

Each side has 16 distinct pieces organized into four groups by formation
pattern.

By default every Red and Black piece has *Pushed by Enemy* and *Captured*;
the exceptions are the **Mountain** (not pushable by enemies), and the
**Shield** (not capturable). Unless a formation
changes these settings, most pieces can be pushed and captured by the
enemy.

### Group Commonalities

Each group shares a set of core abilities. The table below lists the
common defaults for each group; individual pieces may add to or override
them.

| Group | Formation | Common abilities |
|---|---|---|
| Control | Corners | Cross, Any Distance, Push Ally, Push Enemy |
| Movement | Edges | Capture |
| Push | Upper Triangle | Diagonal, Any Distance |
| Capture | Lower Triangle | L-shaped, Any Distance |

In the sections below, each piece's individual table lists only its
**distinguishing** traits — what differs from the group commonality above
— plus its formation effect.

### Control Group — Corners Pattern

These pieces influence **who controls what**.

| Piece | Symbol | Distinguishing traits | Formation Effect |
|---|---|---|---|
| **General** | 将 | *Vital*, *Draw* | Allies gain *Draw*; enemies lose it |
| **Army** | 军 | *Divide* | White pieces inside become controlled by the army's player; same-color allies gain *Divide*; different-color enemies lose *Divide* |
| **Agent** | 间 | — | Enemies inside become **also** controlled by the agent's player (converts them); allies inside have the opponent's control disabled (purges foreign control); white pieces inside become controlled by the agent's player |
| **Spy** | 谍 | Controlled by *both* players | Allies inside become **also** controlled by the opponent (double agent effect); enemies inside have the spy player's control disabled (strips own control from enemies); white pieces inside become controlled by the spy's opponent |

The **General** is your vital piece — lose your general and you lose. It
moves freely but cannot capture; protect it with formations. Its formation
grants *Draw* to allies and strips it from enemies, allowing allies to move
onto the opponent's general to declare a draw. The general itself also
carries this ability (see [How the Game Ends](#how-the-game-ends)).

The **Army** holds the *Divide* ability: it can move and leave a white
piece behind at its original position (dividing forces). Its formation
grants the same *Divide* ability to same-color allies standing inside it,
and makes white pieces inside it controllable by the army's player. See
[White Pieces](#white-pieces).

### Movement Group — Edges Pattern

These pieces carry their namesake movement direction and **grant it to
allies while stripping it from enemies**.

| Piece | Symbol | Distinguishing traits | Formation Effect |
|---|---|---|---|
| **Rook** | 车 | Cross, Any Distance | Allies gain any distance; enemies lose it |
| **Pawn** | 卒 | Cross | Allies gain cross movement; enemies lose it |
| **Scholar** | 士 | Diagonal | Allies gain diagonal movement; enemies lose it |
| **Horse** | 马 | L-shaped | Allies gain L-shaped movement; enemies lose it |

These pieces form the backbone of your army's mobility, and they support
each other: except for the Rook they move one step at a time, but standing
next to a friendly Rook they gain unlimited range. The Rook's formation
also pins: any enemy piece inside it loses the ability to slide.

### Push Group — Upper Triangle Pattern

These pieces specialize in **push mechanics**. They all move diagonally at
any distance.

| Piece | Symbol | Distinguishing traits | Formation Effect |
|---|---|---|---|
| **Wind** | 风 | *Push Ally*, *Push Enemy* | Allies gain both push abilities; enemies lose both |
| **Mountain** | 山 | *Pushed by Ally*, **no** *Pushed by Enemy* | Allies become pushable by allies only; enemies become pushable by the mountain's side only |
| **Fire** | 火 | *Push Ally*, *Push Enemy*, *Push to Capture* | Allies gain *Push to Capture*, lose *Pushed to Captured*; enemies gain *Pushed to Captured*, lose *Push to Capture* |
| **Forest** | 林 | *Captured to Pushed* | Allies gain *Captured to Pushed*, lose *Capture to Push*; enemies gain *Capture to Push*, lose *Captured to Pushed* |

Wind empowers the pieces around it to shove; Mountain takes full control
of how the pieces around it can be acted upon, rewriting both "pushed by"
abilities at once, for allies and enemies alike. Fire and Forest control
the push-blocked and capture-clear behaviors — Fire makes pushes
become captures when blocked, while Forest makes captures become pushes
when the path is clear.

### Capture Group — Lower Triangle Pattern

These pieces specialize in **capture and combat**. They all move in
L-shaped directions at any distance. Only the Spear can capture on its
own; the Shield, Shell, and Mine lack *Capture* and rely on formations or
mutual destruction to interact. The Shield lacks *Captured* — it is the
only piece immune to normal capture across the whole board and can only be
removed through mutual destruction or push-blocked effects.

| Piece | Symbol | Distinguishing traits | Formation Effect |
|---|---|---|---|
| **Spear** | 矛 | *Capture*, *Captured* | Allies gain capture; enemies lose it |
| **Shield** | 盾 | No *Captured* | Allies lose *Captured* (become immune); enemies gain *Captured* |
| **Shell** | 弹 | *Captured on Capture* | Allies gain *Captured on Capture*; enemies lose it |
| **Mine** | 雷 | *Capture on Captured* | Allies gain *Capture on Captured*; enemies lose it |

The **Spear** is the only piece in this group with *Capture*. Its
formation grants capture to allies while stripping it from enemies. The
**Shield** is itself uncapturable and makes everyone around it immune —
stack protection on your general by keeping a Shield nearby. The **Shell**
carries *Captured on Capture*: if it gains *Capture* (e.g., through a
friendly Spear's formation) and captures a piece, it self-destructs in
the process. This effect bypasses the target's *Captured* requirement —
it can take the Shield. The **Mine** carries *Capture on Captured*: anyone
that captures it is destroyed with it. Even pieces without *Capture* can
trigger this by moving onto the Mine — turning it into a trap.

## White Pieces

When any piece is taken off the board by combat — captured, or destroyed
by mutual-destruction — it becomes a **white piece** in a shared pool.
White pieces have cross, diagonal, and L-shaped movement at any distance
and can be captured by anyone. They cannot capture on their own.

A piece with the **Divide** ability can move to an empty point and
leave a white piece behind at its original position (divide forces),
consuming one white piece from the pool. The Army starts with this
ability, and its formation grants it to same-color allies standing
within it.

White pieces carry no control of their own. They are commanded only by
three control-group formations: an Army or Agent lets its owner command
white pieces inside, while a Spy lets the spy's opponent command them.
Several control effects may combine, so a white piece can be commanded by
both players at once. Outside these Army, Agent, and Spy control effects,
a white piece cannot be moved. Capturing one returns it to the pool —
white pieces are recycled, not destroyed.

This adds a third dimension to the game: captured enemies become white
pieces that any piece with *Divide* can bring back onto the board through
dividing forces. Apart from the control effects above, formations never
rewrite a white piece's abilities. White pieces cannot capture and project
no formation, so they are primarily useful for occupying space and
blocking paths.

## How a Game Unfolds

**Placement phase.** The game begins with an empty board. Starting with
Red, the players alternate placing one of their own pieces per turn onto
any empty point in their half of the board: Red places on rows 6–10,
Black on rows 1–5. Passing is not allowed, and white pieces cannot be
placed — every turn must place one of the player's 16 pieces. Resigning
is permitted at any time. No piece may move until both players have
placed all their pieces. This is your one chance to design the initial
formation web — position your pieces so they support each other's
abilities.

**Movement phase.** Once all 32 pieces stand on the board, the players
keep alternating turns (Red first again, since Black placed last). On each
turn a player takes exactly one of these actions:

- **Move** a piece they control to an empty point.
- **Capture:** move onto an occupied point, declaring the intent to take
  the piece there.
- **Push:** move onto an occupied point, declaring the intent to shove it.
  If the push is blocked and either piece has *Push to Capture* or
  *Pushed to Captured*, it becomes a capture (the target is
  destroyed even if it is a friendly piece or lacks *Captured*). Without
  these abilities, a blocked push fails.
- **Draw:** move your own piece with *Draw* ability onto the opponent's
  *Vital* piece, removing the opponent's piece and ending the game in a
  draw.
- **Divide:** move a piece with *Divide* ability to an empty point
  and leave a white piece behind at its original position (dividing
  forces).
- **Pass:** skip the turn without moving. Because passing is always
  available, having no legal move never forces a loss — there is no
  stalemate defeat. The core game assigns no automatic result for repeated
  positions, consecutive passes, or elapsed time; tournament organizers may
  specify repetition, clock, and adjudication rules separately.
- **Resign:** concede the game, immediately ending it with a win for the
  opponent.

Note that when moving onto an occupied point, the intent — capture, push,
or draw — must always be declared as part of the action.

## How the Game Ends

The game carries a persistent **result**: unfinished, Red wins, Black
wins, or draw. Once the result is anything other than unfinished, no
further actions are accepted.

After every movement action (move, capture, push) the board is checked:

- If **neither** side has any piece with the *Vital* ability (both vital
  pieces perished in the same action, e.g. through mutual destruction)
  → **draw**.
- If Red has **no** pieces with the *Vital* ability → Black wins.
- If Black has **no** pieces with the *Vital* ability → Red wins.
- Otherwise → play continues.

A vital piece still waiting to be placed counts as alive. The placement
phase does not change the result — it stays unfinished until movement
begins.

The **draw** action lets a piece with the *Draw* ability move onto the
opponent's vital piece, removing it and setting the result directly to a
draw. The general carries this ability by default and grants it to
allies via its formation. **Resigning** sets the result directly to the
opponent's win. Both bypass the board check.

## Strategic Notes

1. **Formations move with pieces — root them together.** Influence on the
   board is never static: every move reshapes the formation web around it.
   Build a mobile network — one where allies cover each other but can still
   advance together, and critical positions have rotating backup coverage.
   Positioning determines who covers whom, and where the weak spots are.

2. **Blocking — control the battlefield.** Position pieces to block key
   routes and divide the enemy. Every piece on the path blocks movement —
   use this to your advantage. Occupying space is a form of control.

3. **Pushing breaks formations.** Pushing and capturing serve different
   purposes: capturing removes the enemy; pushing displaces them. A
   well-placed shove can throw a key piece out of friendly formation
   coverage, force it into bad terrain, or sever the opponent's formation
   connections. Pushing does not eliminate the piece, but it can dismantle
   the network that piece depends on.

4. **Read the tide — play the position, not the score.** When ahead, stay
   disciplined: one reckless charge beyond friendly formation coverage can
   turn your strongest piece into dead weight. When behind, don't give up:
   your general can move onto the opponent's general to force a draw. Use
   positioning and pushing to stall the opponent and create complications.
   A draw is closer than it looks in this game.

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
- **Ally / Enemy** — for Red and Black pieces, the same color is allied and
  the opposing player's color is enemy. White pieces are neutral: formation
  effects treat them as neither allies nor enemies.
- **Pool** — where pieces wait off the board: each player's own pieces
  before placement, and the shared pool of white pieces.
- **White piece** — a captured piece, recycled into the shared pool;
  any piece with *Divide* ability can bring one back to the board by
  dividing forces.
- **Vital piece** — a piece whose ability *Vital* makes it a win
  condition: a player with no vital pieces left has lost. Normally the
  general.
- **Placement phase / Movement phase** — the two stages of a game: first
  all pieces are placed, then they move.
