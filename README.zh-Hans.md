# 阵棋

[English](README.md)

阵棋（Formation Chess）是一款以局部影响力为核心的双人抽象策略棋类游戏。它采用 9×10 棋盘和一组受象棋启发的棋名，但没有九宫、楚河、将军、将死或固定开局。

每枚棋子都会向相邻特定位置投射**阵法**。处在阵法生效点上的棋子可能获得或失去移动方向、移动距离、控制权、推动、拉动、捉子或特殊转换能力。行动始终按当前有效能力判定，因此同名棋子会随局面变化表现出完全不同的作用。

标准对局从空棋盘开始。红黑双方各自在己方半场布置 16 枚互不重复的棋子，之后轮流行棋。四组棋子为：

```text
兵法组：将 计 势 变
牵制组：风 林 火 山
攻守组：矛 盾 弹 雷
机动组：士 卒 马 车
```

行棋阶段支持普通移动、同色或异色捉子、推动、拉动、要害交换和棋、按兵，以及带目标的认负。

## 从这里开始

- **[游戏规则](docs/rules.zh-Hans.md)**：布阵、能力、阵法、全部行动类型、当前棋子分组和终局条件。
- **[文本记谱法](docs/notation.zh-Hans.md)**：规范的局面快照、行动、行动结果，以及完整棋谱交换约定。
- **[Web 内嵌规则](web/static/rules.zh-Hans.md)**：本地浏览器客户端内置的中文规则文本。

## 工作区 crate

- **`core/` — `formation-chess-core`**：无外部依赖的规则引擎，提供合法行动枚举、撤销、局面快照和中文文本记谱法。
- **`agent/` — `formation-chess-agent`**：按阶段进行候选行动排序和受校验的回合执行，并提供支持固定种子的 Random、纯 MCTS 与 Min Agent。
- **`arena/` — `formation-chess-arena`**：可复现赛程、JSONL 对局记录、严格回放校验、指标和数据集分析。
- **`tui/` — `formation-chess-tui`**：交互式终端客户端，提供标准模式、随机布局模式和快照加载模式。
- **`web/` — `formation-chess-web`**：本地 Axum 服务端与嵌入式浏览器界面，红黑双方可分别选择 Human 或 Min AI 控制。
- **`docs/`**：规则书和记谱规范源文档。

TUI 与 Web 都是本地参考界面。仓库不提供网络匹配、在线服务、身份认证或 Web 对局持久化。Arena 只有在命令行明确指定输出目录时才会写入数据集。

## 快速运行

构建并测试整个工作区：

```sh
cargo build --workspace
cargo test --workspace
```

运行终端客户端：

```sh
cargo run -p formation-chess-tui
```

运行浏览器客户端：

```sh
cargo run -p formation-chess-web
```

Web 服务监听 `127.0.0.1`；未指定端口时自动选择可用端口，并尝试打开默认浏览器。也可以显式指定端口：

```sh
cargo run -p formation-chess-web -- 4000
```

查看 Arena 命令行帮助：

```sh
cargo run -p formation-chess-arena -- --help
```

规则引擎附带两个可执行示例：

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

前者从标准开局开始执行两步布子，后者加载并校验一个自定义文本局面。

## 最小引擎会话

记谱必须基于当前游戏解析，因为阶段、盘上身份、相对移动、按兵和目标化认负都依赖这份局面。

```rust
use formation_chess_core::game::{Game, GameConfig};
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let resolver = NotationResolver::new(&game);
        let action = resolver.parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
```

公共 API 边界、可撤销反应和自定义快照说明见 [`core/README.md`](core/README.md)。

## 自定义棋盘

`GameConfig` 与快照协议支持最大 16×16 的矩形棋盘，也允许描述不必能从标准开局到达、但内部规则一致的局面。校验仍会检查棋池所属方、布阵半场、交替棋池大小、要害棋子数量和声明结果。可接受格式见[文本记谱法](docs/notation.zh-Hans.md)。

## 开发检查

仓库的完整验证命令为：

```sh
cargo +nightly fmt --all -- --check
cargo +nightly test --workspace
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
```

## 许可证

您可以选择使用：

- Apache 2.0 许可证
  （[LICENSE-APACHE](LICENSE-APACHE) 或
  <https://www.apache.org/licenses/LICENSE-2.0>）
- MIT 许可证
  （[LICENSE-MIT](LICENSE-MIT) 或
  <https://opensource.org/licenses/MIT>）

## 贡献

除非您明确说明，否则您有意提交以纳入作品的任何贡献（如 Apache-2.0 许可证中所定义），均应按照上述方式获得双重许可，不附加任何其他条款或条件。
