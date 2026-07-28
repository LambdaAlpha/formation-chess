//! The custom-position example from README.md: parse a snapshot of the
//! text protocol into a validated game.

use formation_chess_core::game::Game;

fn main() -> Result<(), String> {
    let game: Game = "行棋方：黑
红方：[弹 马]
黑方：[将 士 盾]
白方：0
胜负：未分
棋盘：
零[一路 二路 三路 四路 五路]
一[一一 黑车 一一 一一 一一]
二[一一 一一 黑卒 一一 一一]
三[红将 一一 一一 一一 一一]
四[一一 一一 红车 一一 一一]
"
    .parse()?;

    assert!(game.is_placement_phase(), "pools are not empty so phase must be placement");
    Ok(())
}
