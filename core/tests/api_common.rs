#![allow(dead_code)]

use formation_chess_core::action::Action;
use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;

pub const SIMPLE: &str = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[黑将 一一 一一 一一 一一]
二[一一 一一 一一 一一 一一]
三[红卒 一一 红卒 一一 一一]
四[一一 一一 一一 一一 一一]
五[红将 一一 一一 一一 一一]
";

pub const SWAP_STATE: &str = "行棋方：红
红方：[]
黑方：[]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[红将 一一 一一 一一 一一]
二[红车 一一 一一 一一 一一]
三[一一 一一 一一 一一 一一]
四[一一 红马 一一 一一 黑将]
五[一一 一一 一一 一一 一一]
";

pub fn game_with(player: Player, pieces: &[(Piece, (u8, u8))], width: u8, height: u8) -> Game {
    let mut board = Board::new(width, height);
    for &(p, at) in pieces {
        board[at] = Some(p);
    }
    Game::new(GameConfig {
        player,
        board,
        red_pool: vec![],
        black_pool: vec![],
        white: Piece::WHITE,
        white_pool: 0,
        result: GameResult::Unfinished,
    })
    .expect("valid")
}

pub fn game_one(player: Player, piece: Piece, at: (u8, u8)) -> Game {
    game_with(
        player,
        &[(piece, at), (Piece::RED_GENERAL, (0, 4)), (Piece::BLACK_GENERAL, (4, 0))],
        5,
        5,
    )
}

pub fn game_one_3x3(player: Player, piece: Piece, at: (u8, u8)) -> Game {
    game_with(
        player,
        &[(piece, at), (Piece::RED_GENERAL, (0, 2)), (Piece::BLACK_GENERAL, (2, 0))],
        3,
        3,
    )
}

pub fn game_with_white_pool(player: Player, pieces: &[(Piece, (u8, u8))], white_pool: u8) -> Game {
    let mut board = Board::new(9, 10);
    for &(p, at) in pieces {
        board[at] = Some(p);
    }
    Game::new(GameConfig {
        player,
        board,
        red_pool: vec![],
        black_pool: vec![],
        white: Piece::WHITE,
        white_pool,
        result: GameResult::Unfinished,
    })
    .expect("valid")
}

pub fn assert_moves(actions: &[Action], targets: &[(u8, u8)]) {
    let mut found: Vec<(u8, u8)> = actions
        .iter()
        .filter_map(|a| if let Action::Move(m) = a { Some(m.to) } else { None })
        .collect();
    found.sort_unstable();
    let mut expected: Vec<_> = targets.to_vec();
    expected.sort_unstable();
    assert_eq!(found, expected, "move targets mismatch");
}

pub fn assert_captures(actions: &[Action], targets: &[(u8, u8)]) {
    let mut found: Vec<(u8, u8)> = actions
        .iter()
        .filter_map(|a| if let Action::Capture(m) = a { Some(m.to) } else { None })
        .collect();
    found.sort_unstable();
    let mut expected: Vec<_> = targets.to_vec();
    expected.sort_unstable();
    assert_eq!(found, expected, "capture targets mismatch");
}

pub fn assert_pushes(actions: &[Action], targets: &[(u8, u8)]) {
    let mut found: Vec<(u8, u8)> = actions
        .iter()
        .filter_map(|a| if let Action::Push(m) = a { Some(m.to) } else { None })
        .collect();
    found.sort_unstable();
    let mut expected: Vec<_> = targets.to_vec();
    expected.sort_unstable();
    assert_eq!(found, expected, "push targets mismatch");
}
