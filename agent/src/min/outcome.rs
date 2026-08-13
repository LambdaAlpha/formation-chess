use formation_chess_core::ability::Ability;
use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::action::PositionChange;
use formation_chess_core::action::PositionChanges;
use formation_chess_core::board::Board;
use formation_chess_core::piece::Player;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum ResolvedActionKind {
    QuietMove,
    Capture,
    Push,
    Pull,
    Other,
}

pub(super) fn resolved_action_kind(
    board: &Board, action: Action, changes: PositionChanges,
) -> ResolvedActionKind {
    let destination_occupied =
        if let Some(move_) = action_move(action) { board.get(move_.to).is_some() } else { false };
    resolved_action_kind_with_destination(action, changes, destination_occupied)
}

pub(super) fn resolved_action_kind_with_destination(
    action: Action, changes: PositionChanges, destination_occupied: bool,
) -> ResolvedActionKind {
    if matches!(action, Action::Draw(_) | Action::Resign(..) | Action::Place(_)) {
        return ResolvedActionKind::Other;
    }
    if changes_remove_piece(changes.as_slice()) {
        return ResolvedActionKind::Capture;
    }
    if destination_occupied {
        return ResolvedActionKind::Push;
    }
    if matches!(action, Action::Pull(_)) {
        return ResolvedActionKind::Pull;
    }
    ResolvedActionKind::QuietMove
}

pub(super) fn changes_remove_piece(changes: &[PositionChange]) -> bool {
    let old_count = changes.iter().filter(|change| change.old.is_some()).count();
    let new_count = changes.iter().filter(|change| change.new.is_some()).count();
    new_count < old_count
}

pub(super) fn result_after_changes(changes: &[PositionChange]) -> GameResult {
    let red_alive = vital_survives(changes, Player::Red);
    let black_alive = vital_survives(changes, Player::Black);
    match (red_alive, black_alive) {
        (false, false) => GameResult::Draw,
        (false, true) => GameResult::BlackWin,
        (true, false) => GameResult::RedWin,
        (true, true) => GameResult::Unfinished,
    }
}

pub(super) fn action_move(action: Action) -> Option<formation_chess_core::action::Move> {
    match action {
        Action::Move(move_)
        | Action::Capture(move_)
        | Action::Push(move_)
        | Action::Pull(move_)
        | Action::Draw(move_) => Some(move_),
        Action::Place(_) | Action::Resign(..) => None,
    }
}

fn vital_survives(changes: &[PositionChange], player: Player) -> bool {
    let mut removed = false;
    let mut added = false;
    for change in changes {
        if let Some(old) = change.old
            && old.player == player
            && old.ability.has(Ability::LEADER)
        {
            removed = true;
        }
        if let Some(new) = change.new
            && new.player == player
            && new.ability.has(Ability::LEADER)
        {
            added = true;
        }
    }
    added || !removed
}

#[cfg(test)]
mod tests {
    use formation_chess_core::ability::Ability;
    use formation_chess_core::action::Action;
    use formation_chess_core::action::Move;
    use formation_chess_core::board::Board;
    use formation_chess_core::piece::Piece;

    use super::ResolvedActionKind;
    use super::resolved_action_kind;

    #[test]
    fn capture_demotion_is_resolved_as_push() {
        let mut attacker = Piece::RED_ROOK;
        attacker.ability |= Ability::OVERT_CAPTURE;
        let mut target = Piece::BLACK_ROOK;
        target.ability |= Ability::ENEMY_PUSH;
        let mut board = Board::new(3, 3);
        board[(0, 1)] = Some(attacker);
        board[(1, 1)] = Some(target);
        let action = Action::Capture(Move { from: (0, 1), to: (1, 1) });
        let changes = board.try_capture((0, 1), (1, 1)).expect("capture must demote to push");

        assert_eq!(resolved_action_kind(&board, action, changes), ResolvedActionKind::Push);
    }

    #[test]
    fn blocked_push_escalation_is_resolved_as_capture() {
        let mut attacker = Piece::RED_ROOK;
        attacker.ability |= Ability::PUSH_ENEMY | Ability::HIDDEN_CAPTURE;
        let mut target = Piece::BLACK_ROOK;
        target.ability |= Ability::ENEMY_PUSH;
        let mut board = Board::new(3, 3);
        board[(1, 1)] = Some(attacker);
        board[(2, 1)] = Some(target);
        let action = Action::Push(Move { from: (1, 1), to: (2, 1) });
        let changes = board.try_push((1, 1), (2, 1)).expect("blocked push must escalate");

        assert_eq!(resolved_action_kind(&board, action, changes), ResolvedActionKind::Capture);
    }
}
