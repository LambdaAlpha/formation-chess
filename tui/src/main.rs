use std::io::BufRead;
use std::io::Write;
use std::io::stdin;
use std::io::stdout;

use formation_chess_core::action::GameResult;
use formation_chess_core::board::Board;
use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::notation::NotationResolver;
use formation_chess_core::piece::Piece;
use formation_chess_core::piece::Player;
use rand::seq::SliceRandom;

fn main() {
    loop {
        print_menu();
        let Some(choice) = read_line("> ") else {
            break;
        };
        match choice.trim() {
            "1" => {
                let game =
                    Game::new(GameConfig::default()).expect("default config should be valid");
                run_game(game);
            },
            "2" => match create_random_game() {
                Ok(game) => run_game(game),
                Err(e) => println!("创建随机游戏失败：{e}"),
            },
            "3" => {
                println!("请输入记谱法游戏状态（输入空行结束）：");
                match load_game() {
                    Ok(game) => run_game(game),
                    Err(e) => println!("加载失败：{e}"),
                }
            },
            "0" => break,
            "" => {},
            _ => println!("无效选择，请重新输入"),
        }
    }
}

fn print_menu() {
    println!();
    println!("===== 阵棋 =====");
    println!("1. 默认模式 —— 标准对局，从布局阶段开始");
    println!("2. 随机模式 —— 随机布局，直接进入行棋阶段");
    println!("3. 加载模式 —— 输入记谱法盘面继续对局");
    println!("0. 退出");
    println!();
}

fn run_game(mut game: Game) {
    println!();
    while game.result() == GameResult::Unfinished {
        println!("{game}");
        print!("> ");
        let _ = stdout().flush();
        let Some(input) = read_line("") else {
            break;
        };
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }
        match NotationResolver::new(&game).parse_action(&input) {
            Ok(action) => {
                if let Err(e) = game.action(action) {
                    println!("错误：{e}");
                }
            },
            Err(e) => println!("解析错误：{e}"),
        }
    }
    if game.result() != GameResult::Unfinished {
        println!();
        println!("游戏结束！{}", game.result());
        println!();
    }
}

fn create_random_game() -> Result<Game, String> {
    let width: u8 = 9;
    let height: u8 = 10;
    let half = height / 2;
    let midpoint = height.div_ceil(2);

    let mut rng = rand::rng();

    let mut red_positions: Vec<(u8, u8)> =
        (0 .. width).flat_map(|x| (midpoint .. height).map(move |y| (x, y))).collect();
    let mut black_positions: Vec<(u8, u8)> =
        (0 .. width).flat_map(|x| (0 .. half).map(move |y| (x, y))).collect();

    red_positions.shuffle(&mut rng);
    black_positions.shuffle(&mut rng);

    let mut board = Board::new(width, height);

    for (i, piece) in Piece::RED_PLAYER_PIECES.iter().enumerate() {
        board[red_positions[i]] = Some(*piece);
    }
    for (i, piece) in Piece::BLACK_PLAYER_PIECES.iter().enumerate() {
        board[black_positions[i]] = Some(*piece);
    }

    Game::new(GameConfig {
        player: Player::Red,
        board,
        red_pool: Vec::new(),
        black_pool: Vec::new(),
        result: GameResult::Unfinished,
    })
}

fn load_game() -> Result<Game, String> {
    let mut lines = String::new();
    let stdin = stdin();
    let mut handle = stdin.lock();
    loop {
        let mut line = String::new();
        let n = handle.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if line.trim().is_empty() {
            break;
        }
        lines.push_str(&line);
    }
    drop(handle);
    if lines.trim().is_empty() {
        return Err("未输入任何内容".into());
    }
    lines.parse()
}

fn read_line(prompt: &str) -> Option<String> {
    if !prompt.is_empty() {
        print!("{prompt}");
        let _ = stdout().flush();
    }
    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => Some(input),
        Err(_) => None,
    }
}
