# Formation Chess — Game Rules

[简体中文](rules.zh-Hans.md) · [Project overview](../README.md) · [Text notation](notation.md)

This is the rulebook for a standard game of Formation Chess: a 9×10 board,
16 pieces per side, an empty-board setup, and Red moving first. Text formats
for custom boards and snapshots are described in the
[text-notation specification](notation.md).

- [Goal and overview](#goal-and-overview)
- [Board and setup](#board-and-setup)
- [Abilities and effective abilities](#abilities-and-effective-abilities)
- [Formations](#formations)
- [Movement](#movement)
- [Push and capture](#push-and-capture)
- [Piece reference](#piece-reference)
- [White pieces and divide](#white-pieces-and-divide)
- [Turn flow](#turn-flow)
- [Game end](#game-end)
- [Strategic notes](#strategic-notes)
- [Glossary](#glossary)

## Goal and overview

The goal is to remove every piece on the opponent's side that has the
**Vital** ability. In the standard set, the General is the only vital piece,
so the usual result is that the side whose General leaves the board loses.
Custom configurations may use another vital piece, but each side may have at
most one.

The game loop is:

1. Both players place all 16 pieces in their own halves.
2. The players alternate one action at a time.
3. Each action is judged from the **effective abilities** in the position
   before the action, then its changes are applied.
4. Formations are recalculated around the new positions; the vital-piece
   condition is then checked.

Formation Chess has no palace, river, check, or checkmate. The General moves
like any other piece and may be captured, pushed, or removed in a mutual-
destruction effect.

## Board and setup

### Board

The standard board has 9 columns and 10 rows. Columns run from left to right
as 1–9; rows run from top to bottom as 1–10. A point is always written as
`(column, row)`, so `三二` means column 3, row 2.

- Black occupies the top half, rows 1–5, and advances downward.
- Red occupies the bottom half, rows 6–10, and advances upward.
- Every point uses the same geometry; there is no palace, river, or special
  region.

### Placement phase

The game starts with an empty board and Red places first. The players strictly
alternate. On each turn, a player takes one of their unplaced pieces and puts
it on any empty point in their own half:

- Red may place only on rows 6–10; Black only on rows 1–5.
- Each standard piece is placed once; the 16 pieces on a side are unique.
- Passing and placing White pieces are not allowed during placement. Every
  turn must place one of the player's pieces.
- Resignation is allowed during placement.

Black places the final piece. Once both pools are empty, the movement phase
begins and Red moves first again.

## Abilities and effective abilities

Piece names are labels, not special-case rules. What a piece can do is
determined by the abilities it currently has.

Every piece has a set of **base abilities**. Neighboring formations temporarily
rewrite those abilities, producing the **effective abilities** used to judge an
action. They change with the position and are not permanently written back to
the piece.

All four Control-group pieces (General, Army, Agent, and Spy) also start with
**Push Ally** and **Push Enemy**. The table's “Distinctive base abilities or
exceptions” column lists only what distinguishes each row from the other pieces.

| Ability | Meaning |
|---|---|
| **Control (Red / Black)** | Which side may command the piece. A piece may be controlled by both sides or by neither. |
| **Push Ally** | May push a same-color piece. |
| **Push Enemy** | May push a different-color piece. |
| **Pushed by Ally** | May be pushed by a same-color piece. |
| **Pushed by Enemy** | May be pushed by a different-color piece. |
| **Blocked-push capture (active)** | When this piece is the pusher and the target cannot land, the blocked push becomes a capture. The target's capture status and color are ignored. |
| **Blocked-push capture (passive)** | When this piece is the pushed target and the push cannot land, the pusher captures it. |
| **Clear-capture push (active)** | When this piece captures along a clear path, the capture becomes a push if the target can land one step farther. |
| **Clear-capture push (passive)** | When this piece would be captured along a clear path, it is pushed instead if that landing is valid. |
| **Capture** | Has the normal permission to capture. |
| **Captured** | May be removed by a normal capture. |
| **Retaliation** | If this piece is captured, the capturer is removed too; its presence bypasses both normal Capture/Captured ability checks. |
| **Sacrifice** | If this piece captures, it is removed too; its presence bypasses both normal Capture/Captured ability checks. |
| **Any Distance** | May travel any number of steps along an enabled direction instead of one step. |
| **Cross direction** | May move horizontally or vertically. |
| **Diagonal direction** | May move diagonally. |
| **L-shaped direction** | May move by a knight's move. |
| **Divide** | May spend one White piece from the pool, move to an empty point, and leave that White piece at the origin. |
| **Vital** | Losing this piece satisfies that side's loss condition. |
| **Draw** | May move onto the opponent's vital piece, remove it, and end the game as a draw. |

“Active” abilities belong to the piece initiating the interaction; “passive”
abilities belong to the target. An ability matters only when it is present in
the current effective set.

### Which abilities an action uses

An action is judged from the board before it happens:

1. Read the mover's effective abilities and check control, direction, range,
   and path.
2. If the destination is occupied, read the target's effective abilities and
   check the paired push or capture conditions.
3. Resolve landing, blocking, and special conversions.
4. Apply the changes, recycle removed pieces, and switch the player.

Formations are recalculated only after the action. An ability gained at the
destination cannot retroactively make that same action legal.

## Formations

### Range and patterns

Every piece has a 3×3 neighborhood centered on itself, but only some of the
eight neighboring points are active. `O` is the owner; `X` marks an active
point:

```text
    Corners                    Edges
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

The diagrams use Red's viewpoint: Red is at the bottom and advances upward.
Black's formations are mirrored vertically, so Black's Upper Triangle points
downward and its Lower Triangle points upward. A formation affects only active
neighboring points, never its owner.

The four patterns and their groups are:

| Pattern | Group | Pieces |
|---|---|---|
| Corners | Control group | General, Army, Agent, Spy |
| Edges | Movement group | Rook, Pawn, Scholar, Horse |
| Upper Triangle | Push group | Wind, Mountain, Fire, Forest |
| Lower Triangle | Combat group | Spear, Shield, Shell, Mine |

### Overlapping formations

Several formations may affect the same piece at once. Their effects are
combined simultaneously rather than applied in an arbitrary order:

- A formation can grant an ability the piece did not start with, or remove one
  it did have.
- If any active formation removes a particular ability, removal wins and no
  other formation can restore that ability.
- Different abilities can still be modified independently. A piece may, for
  example, gain a movement direction while losing Capture.
- Control bits follow the same rule, so a piece can be controlled by both
  players.

### White is neutral to formation effects

White is a neutral color. For the ally/enemy tests used by formations, a White
piece is neither Red nor Black. Other than the explicit control granted by a control-group piece, formations do not rewrite a White piece's abilities.
White still participates in movement, push, and capture checks using its own
abilities.

## Movement

A piece may move only when all of these conditions hold:

- the player to move has effective control of the piece;
- the piece has a direction ability matching the destination;
- the destination is on the board;
- the path is not blocked;
- an ordinary move or Divide targets an empty point, while a combat action
  targets an occupied point.

### Three directions

- **Cross:** horizontal or vertical.
- **Diagonal:** along a diagonal.
- **L-shaped:** one knight move.

Without **Any Distance**, each direction moves one step. With it, a piece may
repeat the same enabled direction for any number of steps. For L-shaped
movement, this means chaining knight moves along the same 1:2-slope line.

Long cross and diagonal moves require every intermediate point to be empty. An
L-shaped move checks the “leg” of each knight step; any piece on that leg
blocks the step. Every segment of a chained L-shaped move must pass this check.

When the destination is occupied, an ordinary move is not enough: the action
must explicitly declare **capture**, **push**, or **draw**. The text forms are
documented in [Text notation](notation.md).

## Push and capture

Push and capture are different declared intents. Both first require legal
movement geometry and a clear path to the target, then check their interaction
abilities.

### Push

The pusher moves onto the target's point, and the target moves one step farther
along the same movement direction. This is external force: the pushed piece
does not need the corresponding direction ability. Its landing, occupancy,
board edge, and L-shaped leg are still checked.

- To push an ally, the pusher needs **Push Ally** **or** the target needs
  **Pushed by Ally**.
- To push an enemy, the pusher needs **Push Enemy** **and** the target needs
  **Pushed by Enemy**.

If the target cannot land on the next point, the push is blocked. Normally the
action fails. If the pusher has **Blocked-push capture (active)** or the target
has **Blocked-push capture (passive)**, the push escalates to a capture: the
pusher occupies the target's point and the target is removed. Normal Capture,
Captured, and color requirements are not checked for this escalation, so even
a same-color piece or a Shield can be removed this way.

When an L-shaped move pushes, the target takes one further knight step with the
same vector. For example, a push from `(1,1)` toward `(2,3)` sends the target
to `(3,5)`.

### Normal capture

A normal capture moves the attacker onto an enemy-occupied point and removes
the target. It requires:

- different colors;
- Capture on the attacker and Captured on the target;
- no intervening piece between the attacker and the target.

Two special abilities can bypass the normal Capture/Captured ability check:

- **Sacrifice** on the attacker allows it to capture an enemy even when either
  side lacks the ordinary Capture/Captured ability; both pieces are removed.
- **Retaliation** on the target allows the capture under the same bypass; the
  attacker is removed with it.

### Capture becoming a push

If the capture path is clear and either the attacker has **Clear-capture push
(active)** or the target has **Clear-capture push (passive)**, the action first
tries to become a push. If the target can land one step farther, the target is
pushed rather than removed; this conversion does not re-check the normal push
permissions. If that landing is invalid, the action falls back to a normal
capture; the conversion attempt does not make the action fail.

### Mutual destruction

Once a capture is determined, either of these effects can remove both pieces:

- the target's **Retaliation** removes the attacker too;
- the attacker's **Sacrifice** removes the attacker too.

These abilities may be carried by the piece itself or granted by a formation.
If both trigger, the result is still that both pieces leave the board.

## Piece reference

The table lists the base differences between the 16 Red and Black pieces.
Unless a row says otherwise, a colored piece is controlled by its own side,
can be pushed by an enemy, and can be captured. A Spy is also controlled by
the opposing side by default. A formation affects neighbors in its active
pattern, not the piece that projects it.

| Piece | Base movement | Distinctive base abilities or exceptions | Formation effect on neighbors |
|---|---|---|---|
| **General** | Cross, Any Distance | Vital, Draw | White is controlled by the General's side; Allies gain Draw; enemies lose Draw |
| **Army** | Cross, Any Distance | Divide | White is controlled by the Army's side; allies gain Divide; enemies lose Divide |
| **Agent** | Cross, Any Distance | — | White is controlled by the Agent's side; Enemy pieces also become controlled by the Agent's side; allies lose opposing control |
| **Spy** | Cross, Any Distance | Controlled by both sides | White is controlled by the opponent; Allies also become controlled by the opponent; enemy pieces lose the Spy side's control |
| **Rook** | Cross, Any Distance | Capture | Allies gain Any Distance; enemies lose Any Distance |
| **Pawn** | Cross | Capture | Allies gain Cross; enemies lose Cross |
| **Scholar** | Diagonal | Capture | Allies gain Diagonal; enemies lose Diagonal |
| **Horse** | L-shaped | Does not start with Capture | Allies gain L-shaped; enemies lose L-shaped |
| **Wind** | Diagonal, Any Distance | Push Ally, Push Enemy | Allies gain both push abilities; enemies lose both |
| **Mountain** | Diagonal, Any Distance | Pushed by Ally; not Pushed by Enemy | Allies can be pushed only by allies; enemies can be pushed only by the Mountain side |
| **Fire** | Diagonal, Any Distance | Push Ally, Push Enemy; active blocked-push capture | Allies gain active blocked-push capture and lose the passive form; enemies get the reverse |
| **Forest** | Diagonal, Any Distance | Passive clear-capture push | Allies gain the passive form and lose the active form; enemies get the reverse |
| **Spear** | L-shaped, Any Distance | Capture | Allies gain Capture; enemies lose Capture |
| **Shield** | L-shaped, Any Distance | Not Captured by a normal capture | Allies lose Captured; enemies gain Captured |
| **Shell** | L-shaped, Any Distance | Sacrifice | Allies gain Sacrifice; enemies lose Sacrifice |
| **Mine** | L-shaped, Any Distance | Retaliation | Allies gain Retaliation; enemies lose Retaliation |

Mountain rewrites both “pushed by” abilities of its neighbors. Fire and Forest
control the two direction-of-resolution conversions. Shield, Shell, and Mine
change capture permissions or consequences rather than movement directions.

## White pieces and divide

### Where White pieces come from

The shared White pool is replenished only by recycling pieces removed through
combat. This includes normal captures, blocked-push captures, mutual
destruction, and the target removed by a Draw action. Resignation and Divide
do not create White pieces.

The standard White pool starts at zero. Removing a piece increases the pool;
removing a White piece returns it to the same pool, so White pieces are
reusable.

### White's abilities

A White piece always has Cross, Diagonal, L-shaped, and Any Distance movement,
as well as Captured and Pushed by Enemy. It has no Capture, Divide, Vital,
Draw, or default control, and it projects no active formation.

Whether a White piece is controlled depends only on the four control-group
formations:

- a General controls White pieces in its active points for the General's side;
- an Army controls White pieces in its active points for the Army's side;
- an Agent controls them for the Agent's side;
- a Spy controls them for the Spy's opponent.

Several control effects can overlap, so a White piece may be controlled by
both players or by neither. Its neutral color affects formation ally/enemy
tests, but it still participates as White in push and capture checks.

### Divide

A controlled piece with **Divide** may move to an empty point and take one
White piece from the shared pool, leaving it at the original point. With no
White piece available, Divide is illegal.

## Turn flow

### Actions in the movement phase

Players strictly alternate, taking exactly one action per turn:

- **Move:** move a controlled piece to an empty point.
- **Capture:** declare that the occupied destination should be removed; the
  capture rules determine the result.
- **Push:** declare that the occupant should be displaced; a blocked push may
  escalate to capture.
- **Draw:** move a piece with Draw onto the opponent's vital piece and end the
  game immediately as a draw.
- **Divide:** spend one White piece from the pool, move to an empty point, and
  leave that White piece at the origin.
- **Pass:** skip the turn without moving. Pass is always available in the
  movement phase.
- **Resign:** concede and give the opponent the win.

An occupied destination must use an explicit capture, push, or draw intent. An
empty destination must distinguish an ordinary move from Divide.

A successful action switches the player. An invalid action leaves the board,
pools, player, and result unchanged.

### No automatic stalemate loss

Because Pass is always available during movement, having no other legal move
does not lose the game. The core rules also do not produce a result from
repetition, consecutive passes, or a clock; a competition or application may
add separate policies for those matters.

## Game end

The game stores one persistent result: `Unfinished`, `RedWin`, `BlackWin`, or
`Draw`. Once the result is not `Unfinished`, no further action is accepted.

When an action removes pieces without directly declaring a draw, determine the
result from the vital pieces remaining for each side:

- neither side has a vital piece → Draw;
- Red has none and Black still has one → Black wins;
- Black has none and Red still has one → Red wins;
- both still have one → play continues.

A vital piece still in a pool and waiting to be placed counts as alive. The
placement phase does not end the game just because pieces leave their pools.

The **Draw** action does not use the normal capture path. It requires legal
movement, a target that is the opponent's vital piece, and Draw on the mover;
the target is removed and returned to the White pool, and the result is set
directly to Draw. It does not require Capture on the mover or Captured on the
target.

**Resign** sets the result directly to the opponent's win. **Pass** changes
neither the board nor the result.

## Strategic notes

These are ways to inspect a position, not guaranteed openings or tactical
recipes. Effective abilities change after every move, so every conclusion must
be checked against the actual position.

1. **Read effective abilities before piece names.** Identify which pieces the
   player to move controls, then check direction, range, capture, push, and
   conversion abilities. “It is a Rook/Horse/Spear” is not a substitute for
   the current ability set.
2. **Verify path and landing first.** Check whether the destination is empty or
   occupied, whether the path and any Horse leg are clear, and whether a pushed
   target has a valid landing. Reaching a point does not imply that every
   interaction with that point is legal.
3. **Treat formation support as interruptible.** If a critical ability comes
   from one neighboring piece, an ordinary move, push, or capture may remove
   that support. Before committing, look for replacement coverage or a safe
   retreat.
4. **Evaluate push and capture by their final resolution.** A blocked push may
   become a capture; a clear capture may become a push; Retaliation and
   Sacrifice may remove both pieces. Material counts alone do not describe the
   resulting position.
5. **Count the White pool.** Captures add White pieces and Divide consumes
   them. White pieces can also block routes or become controlled by either
   side, so the pool and its reachable deployment points are part of the
   position.
6. **Check the vital piece and the draw exits.** Protecting a vital piece means
   more than avoiding a normal capture: check whether either side has a legal
   Draw route. When several moves are available, first eliminate lines that
   immediately remove your own vital piece.

## Glossary

- **Base ability** — an ability carried by a piece before neighboring
  formations are considered.
- **Effective ability** — the result of applying the current neighboring
  formations simultaneously; actions use this set.
- **Formation** — an ability effect projected onto selected points in the 3×3
  neighborhood around a piece.
- **Formation pattern** — the coverage shape: Corners, Edges, Upper Triangle,
  or Lower Triangle.
- **Control** — which player may command a piece; it is separate from piece
  color.
- **Pool** — unplaced Red and Black pieces, plus the reusable shared White pool.
- **White piece** — a neutral piece. Combat recycling replenishes the shared
  White pool; Divide only spends pieces from that pool.
- **Vital** — the ability that makes a piece part of the win condition.
- **Clear path** — no intervening piece between origin and destination; an
  L-shaped move must also pass its leg checks.
- **Action intent** — the declared treatment of an occupied destination:
  capture, push, or draw.
- **Placement phase / movement phase** — the two stages in which pieces are
  placed and then moved or interacted.
