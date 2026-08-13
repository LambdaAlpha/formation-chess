# Formation Chess — Game Rules

[简体中文](rules.zh-Hans.md) · [Project overview](../README.md) · [Text notation](notation.md)

This rulebook describes the standard game: a 9×10 board, Red and Black each
owning 16 unique pieces, an empty-board placement phase, and Red acting first.
The text format for custom boards and snapshots is documented in
[Text notation](notation.md).

## Goal and overview

Formation Chess is a two-player abstract strategy game about local formation
effects. The objective is to leave the opposing side with no piece that has the
**Leader** ability. In the standard set, each side's General (`将`) is its only
piece with **Leader**.

The game has no palace, river, check, or checkmate. A General moves and
interacts like any other piece. It can be captured, pushed off through a blocked
push conversion, or removed together with another piece.

The standard game has two phases:

1. **Placement:** Red and Black alternately place all pieces in their own halves.
2. **Movement:** the players alternately perform one movement-phase action.

Every action is judged from the complete pre-action position. The involved
pieces' **effective abilities** are fixed from that position for the action.
After its position changes take effect and the player to act changes, the
resulting state of pieces with **Leader** determines whether the game ends.

## Board and placement

### Board coordinates

The standard board has 9 columns and 10 rows. Columns increase from left to
right and rows increase from top to bottom. A coordinate is always written as
`(column, row)`; in text notation, `三二` means column 3, row 2.

- Black's half is rows 1–5, and Black advances toward larger row numbers.
- Red's half is rows 6–10, and Red advances toward smaller row numbers.
- Every point has the same geometry. There are no special regions.

Custom boards may be rectangular from 1×1 through 16×16. On an odd-height
board, the center row belongs to neither placement half.

### Placement phase

The standard game starts with an empty board and these pools for each side:

```text
将 计 势 变 风 林 火 山 矛 盾 弹 雷 士 卒 马 车
```

Red places first, then the players alternate strictly. A placement action takes
one piece from the current player's pool and puts it on an empty point in that
player's half.

- A piece cannot be placed twice.
- Red cannot place a Black piece, and Black cannot place a Red piece.
- The current player may resign during placement.

Black places the final standard piece. Both pools are then empty, so the game
enters the movement phase with Red to act.

## Abilities and effective abilities

A piece name identifies the piece but does not by itself determine its movement.
Each piece has a set of **base abilities**. Formations projected by neighboring
pieces temporarily modify selected abilities, producing the **effective
abilities** used to judge legal actions.

“Ally” and “enemy” compare piece ownership (`Red` or `Black`), not who currently
controls the piece. A Red piece remains an ally of Red pieces even when Black
can command it.

| Ability | Meaning |
|---|---|
| **Initiative**（主动） | The piece's owner may command it. |
| **Passivity**（被动） | The opposing side may command it. |
| **Push Ally**（推友） | Active permission to push an allied piece. |
| **Push Enemy**（推敌） | Active permission to push an enemy piece. |
| **Ally Push**（友推） | Permission to be pushed by an allied piece. |
| **Enemy Push**（敌推） | Permission to be pushed by an enemy piece. |
| **Pull Ally**（拉友） | Active permission to pull an allied piece. |
| **Pull Enemy**（拉敌） | Active permission to pull an enemy piece. |
| **Ally Pull**（友拉） | Permission to be pulled by an allied piece. |
| **Enemy Pull**（敌拉） | Permission to be pulled by an enemy piece. |
| **Hidden Capture**（暗捉） | A blocked push becomes a capture. |
| **Easy Capture**（易捉） | When this piece cannot be pushed farther, the push becomes a capture. |
| **Overt Capture**（明捉） | A capture becomes a push when the target has a valid landing. |
| **Hard Capture**（难捉） | When this piece would be captured but has a valid landing, it is pushed instead. |
| **Capture**（捕捉） | Normal active permission to capture. |
| **Capturable**（被捉） | Normal passive permission to be captured. |
| **Force Capture**（强捉） | Capture regardless of the target's **Capturable** ability; the attacker is removed too. |
| **Counter Capture**（反捉） | When this piece is captured, remove the capturer too; it also bypasses the attacker's **Capture** requirement. |
| **Swift Move**（疾行） | Repeat one enabled movement direction for multiple steps. |
| **Orthogonal Move**（纵横） | Horizontal or vertical movement. |
| **Diagonal Move**（交错） | Diagonal movement. |
| **Broad Step**（阔步） | Knight-step (`日`) movement. |
| **Leader**（首领） | The piece contributes to its owner's survival condition. |
| **Peace Talk**（议和） | Exchange with an opposing **Leader** and end the game in a draw. |

Formation effects depend only on the current position and do not persist after
their conditions cease to hold. After an action, the new position produces a
new set of effective abilities. An ability gained only at the destination
cannot retroactively make the action that reached it legal.

## Formations

### Local patterns

A formation affects selected points in the eight-neighbor area around its
owner. `O` is the owner and `X` is an affected point. The diagrams show Red's
orientation.

```text
      Corners                  Four edges
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+
|   | O |   |              | X | O | X |
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+

   Upper triangle          Down-pointing pentagon
+---+---+---+              +---+---+---+
|   | X |   |              | X |   | X |
+---+---+---+              +---+---+---+
|   | O |   |              | X | O | X |
+---+---+---+              +---+---+---+
| X |   | X |              |   | X |   |
+---+---+---+              +---+---+---+
```

Black formations are mirrored vertically. Corners and four edges are symmetric.
Black's triangle points downward, while Black's Mobility pentagon points upward.
The normal and reversed pentagons are the two player orientations of the same
Mobility pattern. A formation never affects its own center point.

| Group | Chinese name | Pieces | Pattern |
|---|---|---|---|
| Strategy | 兵法组 | `将 计 势 变` | Corners |
| Restraint | 牵制组 | `风 林 火 山` | Four edges |
| Offense/Defense | 攻守组 | `矛 盾 弹 雷` | Upper triangle |
| Mobility | 机动组 | `士 卒 马 车` | Down-pointing pentagon for Red; up-pointing pentagon for Black |

### Overlapping formations

All neighboring formation effects are combined from the same pre-action
position and take effect simultaneously.

- Each effect selects specific abilities and grants or removes them.
- If overlapping effects disagree about one selected ability, removal wins.
- Different abilities are combined independently.
- Control abilities follow the same rule. Since ally and enemy are relative to
  each piece's owner, a piece may end up controlled by both players or by neither.

## Movement geometry

A movement-phase action starting from a board point requires all of the
following:

- the game is unfinished and in the movement phase;
- the current player controls the effective piece at the origin;
- the destination is in bounds and differs from the origin;
- the piece has a matching effective direction;
- the distance is permitted; and
- every required path and leg point is clear.

The three directions are:

- **Orthogonal Move**: one horizontal or vertical step.
- **Diagonal Move**: one diagonal step.
- **Broad Step**: one knight step, with the orthogonal leg point empty.

With **Swift Move**, the piece may repeat the same step vector. Orthogonal Move and
diagonal lines cannot pass through occupied points. A long **Broad Step** line chains
knight steps on one 1:2 slope; every intermediate landing and every leg point
must be empty.

An ordinary move and a pull require an empty destination. Capture, push, and
draw require an occupied destination. The action's declared intent is part of
the rule; reaching an occupied point does not automatically choose how to
interact with it.

## Movement-phase actions

### Move

The controlled piece moves to an empty destination. No other piece changes.

### Capture

A capture first requires legal movement geometry and an occupied target. Piece
ownership does not restrict capture: allied pieces may capture each other.

The interaction is permitted when at least one of these conditions holds:

- the attacker has **Capture** and the target has **Capturable**;
- the attacker has **Force Capture**; or
- the target has **Counter Capture**.

After permission is established, capture demotion is checked. If the attacker
has **Overt Capture** or the target has
**Hard Capture**, and the target has a valid one-step landing
beyond the destination, the capture becomes a push. Push permissions are not
checked again. If that landing is invalid, the action remains a capture.

For a resolved capture, the attacker occupies the target point and the target
leaves the board. Force Capture and Counter Capture remove both pieces only when the
capture is initiated through one of those abilities — the normal
**Capture**–**Capturable** pairing is missing. A normal capture never removes
the attacker, and an escalated push always resolves as a plain capture (see
Push).

### Push

The pusher moves onto the occupied destination and moves the target one movement
step farther along the same vector. The target's own movement directions are
irrelevant because the push supplies the force. The target landing must be in
bounds, empty, and have a clear Broad Step leg when applicable.

- Pushing an ally requires the pusher's **Push Ally** **or** the target's
  **Ally Push**.
- Pushing an enemy requires the pusher's **Push Enemy** **and** the target's
  **Enemy Push**.

If the target has no valid landing, the push is blocked. It becomes a capture
when the pusher has **Hidden Capture** or the target has
**Easy Capture**. This escalation bypasses normal Capture,
Capturable, and ownership restrictions. An escalated capture always resolves as
a plain capture: the pusher occupies the target point, the target leaves the
board, and Force Capture or Counter Capture never applies. Without an escalation
ability, the action is illegal.

### Pull

The active piece moves to an empty destination. The piece one movement step
behind the origin—opposite the active movement vector—moves into the vacated
origin.

- Pulling an ally requires the mover's **Pull Ally** **or** the target's
  **Ally Pull**.
- Pulling an enemy requires the mover's **Pull Enemy** **and** the target's
  **Enemy Pull**.

The pulled piece's own movement directions are irrelevant. Its source must be
in bounds and occupied, and a Broad Step pull must have a clear leg from that
source to the origin. A blocked or missing pull source makes the action illegal.
Pull never converts into capture or push.

### Draw

A draw action uses normal movement geometry and path checks, but not capture or
push permissions.

- The mover must be owned by the current player, not merely controlled by that
  player.
- The mover must have **Peace Talk**.
- The occupied target must be an opposing player's piece with **Leader**.

On success, the two pieces exchange positions and the result becomes a draw
immediately. Neither piece leaves the board, and no capture, material, or
loss of **Leader** pieces replaces this result.

### Resign

**Resign** is target-based during movement. The target must be a piece with **Leader**
controlled by the current player. The target piece's owner loses, and the other
player wins. Normally a player resigns their own General; control from `计` can
also make the opposing General a legal resignation target. During placement,
the current player may resign directly and loses without selecting an on-board
piece.

These rules do not end a game because of repetition or time. Tournament
regulations may add such ending conditions.

## Standard pieces

Base abilities follow **common defaults → group additions → piece traits**.
Later rules override earlier ones.

### Common to all pieces

Every standard piece defaults to **Initiative**, **Capturable**, **Enemy Push**, and **Enemy Pull**. All other abilities are absent by default.

### Group commonalities

| Group | Pieces | Pattern | Added abilities |
|---|---|---|---|
| Strategy | `将 计 势 变` | Corners | Diagonal Move, Swift Move, Push Ally, Push Enemy, Pull Ally, Pull Enemy |
| Restraint | `风 林 火 山` | Four edges | Orthogonal Move, Swift Move, Push Ally, Push Enemy, Pull Ally, Pull Enemy |
| Offense/Defense | `矛 盾 弹 雷` | Up-pointing triangle for Red; down-pointing for Black | One-step Broad Step, Capture |
| Mobility | `士 卒 马 车` | Opposite pentagons: down-pointing for Red; up-pointing for Black | Capture |

### Piece traits

Each piece itself gains what its formation grants allies and loses what its
formation removes from allies; this may override the common defaults. Beyond
that, only General has **Leader**, and Rook additionally has **Orthogonal Move**.

| Piece | Formation effect |
|---|---|
| **General `将`** | Allies gain Peace Talk; enemies lose Peace Talk |
| **Stratagem `计`** | Enemy pieces gain Passivity; allied pieces lose Passivity |
| **Momentum `势`** | Allies gain Hidden Capture and lose Easy Capture; enemies do the reverse |
| **Adaptation `变`** | Allies gain Hard Capture and lose Overt Capture; enemies do the reverse |
| **Wind `风`** | Allies gain Pull Ally and Pull Enemy; enemies lose Pull Ally and Pull Enemy |
| **Forest `林`** | Allies gain Ally Pull and lose Enemy Pull; enemies gain Enemy Pull and lose Ally Pull |
| **Fire `火`** | Allies gain Push Ally and Push Enemy; enemies lose Push Ally and Push Enemy |
| **Mountain `山`** | Allies gain Ally Push and lose Enemy Push; enemies gain Enemy Push and lose Ally Push |
| **Spear `矛`** | Allies gain Capture; enemies lose Capture |
| **Shield `盾`** | Allies lose Capturable; enemies gain Capturable |
| **Shell `弹`** | Allies gain Force Capture; enemies lose Force Capture |
| **Mine `雷`** | Allies gain Counter Capture; enemies lose Counter Capture |
| **Scholar `士`** | Allies gain Diagonal Move; enemies lose Diagonal Move |
| **Pawn `卒`** | Allies gain Orthogonal Move; enemies lose Orthogonal Move |
| **Horse `马`** | Allies gain Broad Step; enemies lose Broad Step |
| **Rook `车`** | Allies gain Swift Move; enemies lose Swift Move |

## Turn completion and game end

A legal action applies all prescribed changes and changes the player to act. An
illegal action has no effect on the board, pools, player, or result.

Except for Draw and Resign, the result depends on which sides retain a Leader
piece after the action:

- neither side retains its piece with **Leader** → Draw;
- only Black retains a piece with **Leader** → Black wins;
- only Red retains a piece with **Leader** → Red wins;
- both retain a piece with **Leader** → the game remains unfinished.

A piece with **Leader** still waiting in a placement pool counts as retained. Once a
game result is decided, no further action is legal.

## Position-reading checklist

1. **Check control first.** Ownership and control are separate; `计` may let a
   player command an opposing piece.
2. **Read effective abilities, not names.** A formation can remove a familiar
   movement or combat ability, or grant one to a different piece.
3. **Verify geometry before interaction.** Check the active path, every Broad Step
   leg, the push landing, and the pull source.
4. **Resolve conversions before counting material.** A blocked push may capture;
   a clear capture may push; Force Capture and Counter Capture may remove both pieces.
5. **Recalculate formations after the action.** Moving either the active piece
   or an externally moved piece can add or remove several abilities at once.
6. **Track Leader outcomes, Peace Talk exchanges, and resignation targets.** These can
   end the game without a normal capture of the current player's own General.

## Glossary

- **Owner / side** — the Red or Black player printed on a piece.
- **Control** — which player may command a piece in the current position.
- **Ally / enemy** — same-owner / different-owner relation, independent of
  control.
- **Base ability** — an ability a piece has before formation effects.
- **Effective ability** — the base ability after all current neighboring
  formation effects are combined.
- **Formation** — a local ability effect projected onto selected neighboring
  points.
- **Placement pool** — a side's pieces that have not yet entered the board.
- **Action intent** — Move, Capture, Push, Pull, or Draw as explicitly declared.
- **Leader** — the ability used by the win condition.
