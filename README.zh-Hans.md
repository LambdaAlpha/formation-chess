# 阵棋

[English](README.md)

阵棋（Formation Chess）是一款以**局部影响力**为核心的双人抽象策略棋类游戏。它采用 9×10 棋盘和一组受象棋启发的棋子名称，但没有象棋的九宫、楚河、将军、将死或固定开局。

每枚有色棋子都会向周围特定位置投射一个**阵法**；中立的白子没有生效阵法。站在阵法生效位置上的棋子，可能获得或失去移动方向、移动距离、控制权、推动、捉子以及特殊战斗效果。同名棋子在不同局面中，可能拥有完全不同的实际能力。

标准对局从空棋盘开始。红黑双方各自把 16 枚互不重复的棋子布置在己方半场，然后轮流行棋。被战斗移出棋盘的棋子会回收到共享白子池；拥有**分兵**能力的棋子可以把白子重新带回棋盘。

## 从这里开始

- **[游戏规则](docs/rules.zh-Hans.md)** —— 标准对局的完整规则：能力、阵法、移动、战斗、白子与终局。
- **[文本记谱法](docs/notation.zh-Hans.md)** —— 局面快照、行动、行动结果，以及已定义但尚未由引擎直接解析的棋谱格式。
- **[Web 内嵌规则](web/static/rules.zh-Hans.md)** —— 本地 Web 客户端内置的中文规则文本。

## 仓库内容

- **`core/` —— `formation-chess-core`**：无外部依赖的 Rust 规则引擎与文本记谱法实现。不包含 AI、持久化或用户界面。
- **`agent/` —— `formation-chess-agent`**：按阶段拆分的 Agent 接口、紧凑的几何布阵区域、移动阶段合法行动枚举、经过规则引擎校验的回合执行，以及支持固定种子的随机基线 Agent。
- **`tui/` —— `formation-chess-tui`**：终端客户端，提供标准模式、随机布局模式和加载快照模式。
- **`web/` —— `formation-chess-web`**：本地浏览器客户端与 HTTP 服务端；前端资源嵌入二进制，服务端维护一局内存中的对局。
- **`docs/`**：规则书与记谱法规范的源文档。
- **`core/tests/`**：规则引擎的数据驱动测试和 API 测试。

这些客户端是本地参考界面，尚未把随机 Agent 暴露为可选对手。仓库不包含网络匹配、在线服务或持久化存档。

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

Web 服务只监听本机回环地址；不指定端口时会自动选择可用端口并尝试打开浏览器。也可以指定端口：

```sh
cargo run -p formation-chess-web -- 4000
```

规则引擎附带两个可执行示例：

```sh
cargo run -p formation-chess-core --example readme
cargo run -p formation-chess-core --example readme_custom
```

前者从标准开局开始并执行两步布子，后者加载并验证一个自定义文本局面。

## 最小引擎会话

引擎会根据当前棋盘和阶段解析记谱法，再执行得到的行动：

```rust
use formation_chess_core::game::{Game, GameConfig};
use formation_chess_core::notation::NotationResolver;

fn main() -> Result<(), String> {
    let mut game = Game::new(GameConfig::default())?;

    for text in ["红将五十", "黑将五一"] {
        let resolver = NotationResolver::new(game.board(), game.phase());
        let action = resolver.parse_action(text)?;
        let reaction = game.action(action)?;
        println!("{text} → {}", reaction.game_result);
    }

    print!("{game}");
    Ok(())
}
```

API、快照校验和文本协议的说明见 [`core/README.md`](core/README.md)。

## 自定义棋盘

`GameConfig` 与文本快照协议可以描述最大 16×16 的矩形棋盘，也可以描述标准对局无法通过正常行动到达的局面。引擎仍会校验棋子颜色、要害棋子数量、布阵半场、棋池交替关系和声明的胜负状态。可接受的快照格式及限制见[记谱法文档](docs/notation.zh-Hans.md)。

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
