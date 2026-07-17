//! The minimal session from README.md: start a standard game and play
//! the first two placement moves.

use formation_chess_core::game::Game;
use formation_chess_core::game::GameConfig;
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let action = NotationResolver::new(game.board()).parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
