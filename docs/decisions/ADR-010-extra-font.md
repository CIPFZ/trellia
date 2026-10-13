# ADR-010：追加字体 `RunConfig.font_path`（游戏 spec E-1）

- 编号：ADR-010
- 标题：`RunConfig.font_path` 与后端 `setup_fonts`
- 日期：2026-10-13
- 状态：提议（来自 Rootguard 游戏 spec 第 15 章 E-1，对应引擎 v1 报告第 3.2 条、ADR-005 §6）

## 背景

egui 默认字体不含 CJK 字形，游戏 HUD 需要中文。ADR-005 §6 约定：需要时另立 ADR，引入带 LICENSE 的
开源字体并在后端 `setup_fonts` 中加载。

## 决策

1. `RunConfig` 新增 `font_path: Option<PathBuf>`（默认 `None`，行为与 v1 相同）。
2. `run` 在启动窗口前读取该文件（失败返回错误，信息含路径），在 eframe 创建回调中调用
   `setup_fonts`：以 egui 默认字体为基础，把该字体**追加**到 `Proportional` 与 `Monospace`
   字体族末尾作为回退。默认字体覆盖的字符（拉丁字母等）外观不变，缺字时使用追加字体。
3. 公开 `font_definitions(bytes) -> egui::FontDefinitions` 与常量 `CJK_FONT_NAME`，便于测试与复用。
4. `DrawList` 的文字由后端用同一个 egui 上下文绘制，因此形状文字与 egui 控件同时获得 CJK 支持。
5. **字体文件随消费者提供**：子集的字符集取决于游戏文本，引擎仓库不内置 CJK 字体
   （避免为示例引入数 MB 的文件）；消费者负责字体的许可证与登记（如 Rootguard 的
   `assets/fonts/LICENSE-OFL.txt` 与 `assets/SOURCES.md`）。
6. headless 后端不加载字体（不渲染，不影响测试）。

## 后果

- 公共 API 增加一个 `RunConfig` 字段；使用结构体更新语法（`..RunConfig::default()`）的调用方不受影响。

## 替代方案

1. 内置一个 CJK 子集到引擎：子集字符集无法预知，内置全集体积过大，否决。
2. 替换而非追加默认字体：拉丁字母外观随 CJK 字体变化，且影响调试面板，否决。
