# 阵棋

[English](README.md)

阵棋是一种双人策略棋类游戏。它沿用了象棋的 9×10 棋盘与棋子名称，却改动了一件足以改变一切的事：每枚棋子都向周围投射一小片被称为**阵法**的影响力区域，重写附近棋子——无论敌我——的能力。

没有任何棋子永远"只是它自己"。卒站在车旁，忽然就能横扫全盘；车误入敌方车阵，便痛失千里，只能徐行一步。开局也没有固定阵型：第一步棋之前，双方先把各自的16 枚棋子自由布置在己方半场——每一局都始于你亲手设计的阵势。

## 学习游戏

- **[游戏规则](docs/rules.zh-Hans.md)** —— 完整规则书：棋盘、棋子、能力、阵法，以及对局的进行与胜负。
- **[文本记谱法](docs/notation.zh-Hans.md)** —— 如何用纯文本书写局面与着法，用于记录和分享对局。

## 项目状态

本仓库目前包含游戏的**规则引擎**：一个实现了完整规则与文本记谱法的程序库。可供游玩的前端已在计划中，尚未开发——现阶段只能通过引擎的编程接口或文本协议驱动对局。

## 面向开发者

引擎是位于 [`core/`](core) 的 Rust crate `formation-chess-core`，无外部依赖。构建与测试：

```sh
cargo build
cargo test
```

一次最小会话——开始一局标准对局并走出头两步布子（[core/examples/readme.rs](core/examples/readme.rs)，可用`cargo run --example readme` 运行）：

```rust
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
```

除标准开局外，引擎也接受自定义配置——最大 16×16 的棋盘尺寸与任意初始局面——以 `GameConfig` 值或文本协议快照的形式提供；见[记谱法文档](docs/notation.zh-Hans.md)。

仓库结构：

```
core/          规则引擎（crate formation-chess-core）
core/tests/    数据驱动的测试套件（*.txt 文件）
docs/          游戏文档（规则、记谱法）
```

## 许可证

您可以选择使用

* Apache 2.0 许可证
  ([LICENSE-APACHE](LICENSE-APACHE) 或 <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT 许可证
  ([LICENSE-MIT](LICENSE-MIT) 或 <http://opensource.org/licenses/MIT>)

## 贡献

除非您明确说明，否则您有意提交以纳入作品的任何贡献（如 Apache-2.0 许可证中所定义），均应按照上述方式获得双重许可，无任何附加条款或条件。
