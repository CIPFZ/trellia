# Trellia 引擎 v1 报告

- 日期：2026-10-13
- 范围：SPEC.md 第 13 章 M0 ～ M5
- 结论：全部验收通过。手动核对项在 Windows Server 2022 实机（MSVC，带声卡）上完成，
  Linux 在 Xvfb + Mesa 软件渲染下核对；golden 快照在 Linux 与 Windows 上逐字节一致（见 `docs/progress.md`）。

## 1. 各 crate 规模

行数为 `*.rs` 总行数（含文档注释与测试），测试数为 `cargo test` 通过数（含集成与文档测试）。

| crate | 行数 | 测试 | 职责 |
|---|---:|---:|---|
| trellia-core | 1669 | 27 + 1 doc | App / Plugin / Stage / World / CommandBuffer / Time / Timer / Tween / GameRng / Input |
| trellia-assets | 576 | 9 + 1 doc | Handle / Assets / AssetServer / AssetsPlugin |
| trellia-gfx | 1397 | 18 | Viewport / DrawList / Sprite / Atlas / Animator / GfxPlugin |
| trellia-audio | 265 | 4 | AudioQueue / AudioRequest / AudioBackend / AudioPlugin |
| trellia-scene | 562 | 7 | Scene / SceneStack / SceneCommands / ScenePlugin / UiHooks |
| trellia-devtools | 298 | 2 | DevtoolsPlugin / DebugDraw |
| trellia-egui | 731 | 7 | eframe 后端、DrawList 绘制、输入映射、RodioBackend |
| trellia-headless | 666 | 3 + 9 集成 + 1 doc | HeadlessRunner / Snapshot；M2 / M4 / M5 集成测试 |
| trellia（门面） | 84 | 1 | re-export、prelude、run_egui / run_headless |
| examples/core_counter | 100 | 1 | M1 示例 |
| examples/hello_window | 84 | 1 | M3 示例 |
| examples/sprites | 427 | 6 | M3 / M4 示例 |
| examples/lane_demo | 991 | 2 | M5 垂直切片 |
| **合计** | **7850** | **100** | |

SPEC.md 第 4 ～ 8 章与第 13 章列出的测试共 37 个，全部以同名测试实现（见 `docs/progress.md` 各里程碑）。

## 2. 决策记录

ADR-001 hecs 进入公共 API；ADR-002 依赖规则细化（audio → assets、门面、示例）；ADR-003 core API 细节；
ADR-004 资产 / 绘制 / 动画细节；ADR-005 egui 后端细节；ADR-006 音频 / 场景 / 调试细节；ADR-007 不新增字母键（M3 翻转键为 F2）。

## 3. 已知限制

1. **字母键**：`Key` 枚举按 SPEC.md 4.9 不含字母键；M3 翻转键为 F2（ADR-007，SPEC 措辞已修正）。
2. **字体**：只有 egui 默认字体（无 CJK）；示例与调试面板文字均为 ASCII（ADR-005 §6）。
3. **平台覆盖**：Windows 实机（画面、输入、letterbox、发声、性能）与 Linux Xvfb 已核对；Linux 桌面实机与
   macOS 未运行（macOS 仅 `cargo check`）。音频“听感”未经人耳确认，只以系统会话峰值表证明信号送达输出端点。
4. **跨平台确定性**：golden 依赖 IEEE-754 f32 加减法的确定性，模拟中未使用超越函数；已在 x86_64 Linux 与
   x86_64 Windows（MSVC）上逐字节一致，ARM（如 Apple Silicon）尚未比对。
5. **headless UI**：headless 每帧驱动 egui UI 回调但不注入指针事件，无法“点击” egui 按钮；
   结算场景同时提供回车键重开以便测试。
6. **启动错误**以字符串记录（`App::record_startup_error`），丢失错误源链。
7. **插件顺序敏感**：`GfxPlugin` 须先于游戏插件（动画推进与 DrawList 清空的位置），`ScenePlugin`
   之后注册的 PostUpdate 系统发出的场景命令顺延一帧（ADR-006）。
8. **资产加载样板**：`load_atlas` / `load_clip` 需同时可变借用多个 `Assets<T>`，游戏层要临时从
   `Resources` 取出再放回（示例中各写了一遍）。
9. **DrawList**：矩形 / 圆 / 文字命令的 `sort_y` 恒为 0，同层只按提交顺序排序（ADR-004 §7）。
10. **eframe 首帧**：egui 首个 pass 结束前把指针视为位于 egui 上，首帧的鼠标按下不会转发给游戏。

## 4. 建议在游戏开发期补充的功能（仅记录，v1 未实现）

- 一个在 `World` 上直接加载图集 / 片段 / 音频的辅助函数，消除第 3.8 条样板。
- 带 LICENSE 的开源 CJK 字体（子集化）与后端 `setup_fonts`，用于中文 HUD。
- headless 向 egui 注入指针 / 键盘事件，以便无窗口测试选卡、菜单等 UI 流程。
- 形状命令可选的 `sort_y` 参数，使占位渲染也能按行（y）遮挡。
- 音乐淡入淡出、切换场景时的 BGM 交叉淡化。
- 游戏层的 `StatusEffects` 常驻组件（冰冻 / 减速 / 定身，SPEC.md 4.3.4 建议）与多车道网格工具。
- 关卡 / 波次数据的 RON 定义与启动时校验（属于游戏 spec）。
- 设置与存档持久化。
