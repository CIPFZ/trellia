# Trellia 进度记录

## 会话记录（新会话追加在最上方）

- 2026-10-13：首次会话。计划：通读 SPEC.md / AGENT.md，完成 M0 工程骨架，随后按顺序推进 M1 起的里程碑。

## 当前状态

- 当前里程碑：M5 垂直切片 lane_demo
- 上次中断点：M4 完成（M3 / M4 实机人工复核待负责人确认，见下）

## 待负责人确认的问题

1. **`Key` 枚举缺少字母键 F**（SPEC.md 4.9 “v1 仅此” vs 13 M3 “F 键翻转”）。两种解读导致公共 API 不同（是否新增 `Key::F`），
   按 AGENT.md 3.3 需人类裁决。当前 `sprites` 示例暂用 `Key::F2`（`examples/sprites/src/lib.rs` 中的 `FLIP_KEY` 常量）。
2. **M3 / M4 人工验收**：本机无显示器、无声卡，只能在 Xvfb + Mesa 软件渲染下自动化核对 Linux；Windows / macOS 只做了交叉 `cargo check`。
   需负责人在实机（Linux + 另一平台）上手动运行 `sprites` 复核画面、输入与**实际发声**（本机音频设备不可用，RodioBackend 已降级为静默）。


- Linux 构建 `rodio` 需要 ALSA 开发包：TencentOS / Fedora 系 `dnf install alsa-lib-devel`，Debian / Ubuntu 系 `apt-get install libasound2-dev`（CI 已配置）。
- `check_deps.sh` 需要 `python3`（解析 `cargo metadata` 的 JSON）。
- 本机 `/data` 分区接近写满（其他 workspace 占用），构建目录改用 `CARGO_TARGET_DIR=/root/trellia-target`。
- 无 GPU / 显示器时可用 `Xvfb :99` + `LIBGL_ALWAYS_SOFTWARE=1` 运行示例，`xdotool` 注入输入、`import` 截图。
- 直接运行编译产物（非 `cargo run`）时需设置 `TRELLIA_ASSETS=<仓库>/assets`。
- workspace `members` 在 M0 暂为 `["crates/*"]`（cargo 将无匹配的 glob 视为字面路径并报错），M1 加入首个示例后已恢复为 `["crates/*", "examples/*"]`。
- 示例均为 `examples/<name>/` 下的独立包：`src/lib.rs`（可测试逻辑）+ `[[example]] path = "src/main.rs"`（`autobins = false`），因此根目录 `cargo run --example <name>` 可用。
- workspace 依赖 `trellia` 设置 `default-features = false`，示例按需开启 `egui` / `devtools`。
- 原始图标位于 `asserts/icon/`（目录名疑似笔误），已复制到 SPEC.md 3.3 规定的 `assets/icon/`；`asserts/` 未纳入版本控制，待负责人确认后删除。

## 里程碑

### M0 工程骨架

- 状态：✅ 完成（2026-10-13）
- 交付：
  - [x] workspace `Cargo.toml`（`[workspace.dependencies]` 含 1.5 白名单全部 14 个依赖，版本由 `cargo add` 获取；`[workspace.lints]` 配置 `unsafe_code = forbid`、`missing_docs`、`clippy::must_use_candidate`、`clippy::missing_errors_doc`）
  - [x] `rust-toolchain.toml`（`channel = "stable"`）
  - [x] 九个 crate 空壳（`Cargo.toml` 依赖按 R1 ～ R5 预先声明、`lib.rs` 含 `#![forbid(unsafe_code)]` 与 `//!` 职责文档、`README.md`）
  - [x] 门面 `trellia`：`pub mod prelude {}`，features `egui` / `devtools`（均默认开启）
  - [x] `scripts/check_deps.sh`（`cargo metadata` + 硬编码允许表）
  - [x] CI：`.github/workflows/ci.yml`，矩阵 ubuntu / windows / macos
  - [x] `ADR-000-template.md`、`ADR-001-hecs-types-in-public-api.md`、`ADR-002-dependency-rules-clarification.md`
  - [x] `docs/progress.md`
- 验收：
  - [x] `cargo build --workspace` 通过
  - [x] `./scripts/check_deps.sh` 对空壳通过：`check_deps: 9 个 crate 全部符合依赖规则 R1 ～ R5`，exit 0
  - [x] 临时在 `trellia-core` 加入 `egui.workspace = true` 后失败：`trellia-core -> egui  违反 R1`，exit 1；撤销后恢复 exit 0
  - [x] fmt / clippy `-D warnings` / test / doc 零警告
- 行数：每个 crate 的 `lib.rs` 为 10 ～ 15 行文档，暂无代码
- 测试数：0
- 遗留问题：`examples/*` 尚未加入 workspace members（M1 已解决）

### M1 核心

- 状态：✅ 完成（2026-10-13）
- 交付：
  - [x] `trellia-core` 全部第 4 章功能：`App` / `Plugin` / `Stage` / `System`、`World` / `CommandBuffer` / `Transform2D`、`Resources`、`Events` / `EventBus`、`Time`、`Timer`、`TweenF32` / `TweenVec2`、`GameRng`、`Input` / `FrameInput` / `FrameOutput` / `InputEvent`
  - [x] `trellia-headless`：`HeadlessRunner::new / app / app_mut / frame / frames / frame_with / register_snapshot / snapshot`、`Snapshot` trait（`Transform2D` 默认登记）
  - [x] 门面：re-export `core` / `headless`、`prelude`、`run_headless`
  - [x] `examples/core_counter`
  - [x] ADR-003（core API 细节：`System` 函数指针、`Input` 原始像素字段、`AppExit`、glam / hecs 重新导出、累加器阈值、`step_ticks` 语义）
- 验收（SPEC.md 4.11，均位于 `crates/trellia-core/src/`）：
  - [x] `fixed_step_accumulates`（app.rs；另验证 1000 帧 1/30 s 恰好 2000 tick 无漂移）
  - [x] `paused_skips_fixed_update`（app.rs）
  - [x] `step_once_runs_exactly_one_tick`（app.rs）
  - [x] `events_live_two_frames`（app.rs：逐帧可见数 `[0, 0, 1, 1, 0]`）
  - [x] `commands_apply_at_tick_end`（app.rs）
  - [x] `rng_deterministic`（rng.rs）
  - [x] `transform_prev_synced_each_tick`（app.rs）
  - [x] `input_just_pressed_single_frame`（app.rs）
  - [x] `timer_repeating_resets`（time.rs）
  - [x] `determinism_smoke`（app.rs：RNG + Timer + commands，两次 `step_ticks(600)` 快照逐字节相同，不同种子不同）
  - [x] `core_counter` 示例 headless 运行 600 tick 打印确定结果，两次运行一致：
    `core_counter: seed=42 ticks=600 pulses=10 rng_sum=497 snapshot_bytes=244`
- 行数（`*.rs`，含测试与文档）：core 1645、headless 193、trellia 48、core_counter 100
- 测试数：core 26 单元 + 1 文档测试；headless 2 单元 + 1 文档测试；trellia 1；core_counter 1（合计 32）
- 遗留问题：无

### M2 资产、绘制、动画（headless）

- 状态：✅ 完成（2026-10-13）
- 交付：
  - [x] `trellia-assets`：`Handle` / `Asset` / `Assets`、`AssetServer`（RON / PNG / 音频字节）、`AssetError`、`AssetsPlugin`
  - [x] `trellia-gfx`：`Viewport`、`Rect` / `Color` / `Layer` / `TextAlign`、`DrawList` / `DrawCmd` / `DrawKind` / `DrawCmdBuilder`、`Atlas` / `AtlasFrame` / `Sprite`、`AnimationClip` / `AnimationFrame` / `Animator` / `AnimationEvent`、`GfxPlugin`、`load_atlas` / `load_clip`
  - [x] `trellia-core` 增加 `App::record_startup_error` / `startup_errors`（ADR-004）
  - [x] `assets/demo/`：`placeholder_8x8.png`、`walker.png`（每帧 64 × 96，2 × 2 共 4 帧，ADR-004）、`walker.atlas.ron`、`walker_walk.anim.ron`、`walker_bite.anim.ron`、`SOURCES.md`；生成脚本 `scripts/gen_demo_assets.py`
  - [x] `HeadlessRunner::last_draw_list`；虚拟窗口尺寸跟随 `Viewport.logical_size`；启动错误时构造即 panic
  - [x] ADR-004
- 验收：
  - [x] 5.3 `handle_is_copy_and_typed`（handle.rs 单元测试 + `Handle` 文档中的 `compile_fail` 文档测试）
  - [x] 5.3 `add_with_path_dedups`、`missing_file_error_contains_path`、`load_ron_roundtrip`、`load_png_dimensions`（8 × 8，`rgba.len() == 256`）
  - [x] 6.7 `viewport_letterbox_wide`、`viewport_letterbox_tall`、`viewport_roundtrip`
  - [x] 6.7 `drawlist_sort_order`
  - [x] 6.7 `animator_advances_by_ticks`（逐 tick 帧序列 `[0, 0, 1, 1, 0, 0]`）、`animator_half_speed`（`[0, 0, 0, 0, 0, 1]`）、`animator_event_fires_once_per_entry`、`animator_non_loop_finishes`
  - [x] 6.7 `sprite_render_uses_anchor_and_lerp`、`atlas_validation_rejects_out_of_bounds`
  - [x] 集成测试 `crates/trellia-headless/tests/sprite_appears_in_drawlist.rs`：30 tick 后恰 1 条 `Sprite` 命令，`src_px` 为 `walk_1`（`walker_walk` 两帧各 20 tick）
- 行数（`*.rs`，含测试与文档）：assets 576、gfx 1397、headless 300、core 1669
- 测试数：core 27、assets 9 + 1 文档、gfx 18、headless 3 + 1 集成 + 1 文档、trellia 1、core_counter 1（合计 63）
- 遗留问题：无

### M3 egui 后端

- 状态：🟡 自动化部分完成（2026-10-13）；人工验收待负责人在实机确认（见“待负责人确认的问题”）
- 交付：
  - [x] `trellia-egui`：`run`、`RunConfig`、`WindowStatus`；`src/input_map.rs`（仅 v1 `Key`）、`src/paint.rs`（DrawList → egui，纹理惰性上传 `NEAREST`，同纹理精灵合批）
  - [x] 门面：`run_egui`、`RunConfig` 重新导出、`egui_backend` 模块（feature `egui`）
  - [x] `examples/hello_window`、`examples/sprites`（`--count N`，标题栏显示帧耗时）
  - [x] ADR-005（RunConfig 语义、物理像素约定、指针归属、glow 渲染器、`WindowStatus`、字体、合批）
- 验收证据：
  - [x] Linux（Xvfb 1920 × 1080 + Mesa llvmpipe，release）：
    - `hello_window` 截图：深色背景上显示 "Trellia" 与蓝色矩形
    - `sprites` 缩放：`xdotool windowsize 1600 600` 后左右各约 267 px 黑边，画面等比缩放（scale 0.833）
    - 点击准确：在黑边偏移下点击窗口 (800, 300)，圆心出现在逻辑 (640, 360) 即画面中心
    - 翻转：按翻转键后全部行者朝向从左变为右（截图对比）
    - 动画：各行者以 1/2、1/1、3/2、2/1 倍速播放走路帧并以不同速度左移、循环
    - 性能：`--count 1000`，标题栏 `sprites | 1000 sprites | 0.70 ms`（eframe 整帧 CPU 耗时，软件渲染）< 4 ms
  - [x] Windows（`x86_64-pc-windows-msvc`）与 macOS（`aarch64-apple-darwin`）：`cargo check --example sprites` 通过（仅编译）
  - [ ] 实机人工核对（Linux 桌面 + 另一平台）——待负责人
  - [x] 自动化测试：egui 7（输入映射 / 坐标 / 颜色 / 启动校验）、hello_window 1、sprites 3（精灵数、点击圆存活 1 秒、翻转）
- 行数（`*.rs`）：trellia-egui 579、hello_window 84、sprites 286
- 测试数：累计 75
- 遗留问题：翻转键待确认（见上）；未加入 CJK 字体（ADR-005 §6，示例仅 ASCII）

### M4 音频、场景、调试

- 状态：✅ 自动化验收完成（2026-10-13）；实际发声与实机交互待负责人复核
- 交付：
  - [x] `trellia-audio`：`AudioClip`、`AudioBus`、`AudioRequest`、`AudioQueue`、`AudioBackend`、`AudioPlugin`
  - [x] `RodioBackend`（`trellia-egui/src/audio.rs`）：每个音效一个 Player、同 clip ≤ 8 路、单一音乐 Player、总线音量、无设备降级静默
  - [x] `trellia-scene`：`Scene`（`Send + Sync`）、`SceneCommand`、`SceneCommands`、`SceneStack`、`ScenePlugin::new`、`UiHooks` / `run_ui_hooks` / `add_ui_hook`
  - [x] `trellia-devtools`：`DebugDraw`、`DevtoolsPlugin`（F1 面板）
  - [x] 后端：egui 与 headless 每帧调用 UI 回调；headless 增加 `click` / `press_key` / `take_audio_log`
  - [x] `assets/demo/blip.ogg`、`hit.ogg`、`loop.ogg`（10.000 s），由 `scripts/gen_demo_audio.py` 合成，登记于 `SOURCES.md`
  - [x] ADR-006
- 验收：
  - [x] 8.1 `push_pauses_below`、`overlay_still_draws_below`、`blocks_draw_hides_below`、`commands_applied_at_frame_end`、`empty_stack_requests_exit`（`crates/trellia-scene/src/lib.rs`）
  - [x] `audio_request_recorded`（`examples/sprites/src/lib.rs`）：模拟点击后 `take_audio_log` 恰含 1 条 `PlaySfx(blip)`
  - [x] `sprites` 扩展（headless 测试 `escape_pauses_and_resumes`、`devtools_step_advances_one_tick_while_paused`）
  - [x] Xvfb 核对：F1 面板显示 FPS / tick / 实体数 / 命令数 / 滑杆 / 暂停 / 单步 / DebugDraw / 场景栈；
    Esc 推入半透明 “PAUSED” 覆盖层，下方画面冻结可见，面板显示 `scenes: Sprites > Paused`；暂停时点 3 次 Step，tick 203 → 206；
    滑杆拖到 4.00 后 `ticks/frame 2`；本机无声卡时日志 `音频设备初始化失败，降级为静默后端`，程序继续运行
  - [ ] 实机发声与交互复核——待负责人
- 行数（`*.rs`）：audio 215、scene 562、devtools 298、egui 731、headless 453、sprites 427、trellia 77
- 测试数：累计 91（`cargo test --workspace` 通过数合计）
- 遗留问题：翻转键待确认
