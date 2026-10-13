# Trellia 进度记录

## 会话记录（新会话追加在最上方）

- 2026-10-13 游戏补充 E-3（Rootguard SPEC 15）：`DrawList::rect_sorted / rect_stroke_sorted / circle_sorted / text_sorted`（ADR-009）。
- 2026-10-13 游戏补充 E-2（Rootguard SPEC 15）：新增 `load_atlas_into` / `load_clip_into` / `load_audio_into`（ADR-008），`lane_demo` 改用之，golden 不变。
- 2026-10-13（续）：推送至 github.com/CIPFZ/trellia；首次 CI（run 38376052953）三平台全绿。
- 2026-10-13（续）：负责人提供 Windows 桌面环境，完成第二平台实机验证；裁决翻转键（ADR-007）。
- 2026-10-13：首次会话。计划：通读 SPEC.md / AGENT.md，完成 M0 工程骨架，随后按顺序推进 M1 起的里程碑。

## 当前状态

- 当前里程碑：M0 ～ M5 全部完成（含 Windows 实机验证）
- 上次中断点：M5 完成，`docs/engine-v1-report.md` 已提交

## 待负责人确认的问题

1. ~~`Key` 枚举缺少字母键 F~~ —— **已裁决（2026-10-13，ADR-007）**：不新增 `Key::F`，SPEC.md 13 M3 措辞修正为“F2 键翻转”，`FLIP_KEY = Key::F2` 保持不变。
2. **M3 / M4 / M5 人工验收**：Windows 实机已完成（见下方“Windows 实机验证”）；Linux 仅在 Xvfb + Mesa 下核对，
   如需 Linux 桌面实机复核，请负责人在带显示器的 Linux 上运行 `sprites` 与 `lane_demo`；macOS 已在 CI（arm64）中通过全部测试，但未运行窗口示例。
3. ~~golden 文件~~ —— **已确认（2026-10-13）**：负责人认可 `tests/golden/lane_demo_seed42.snap` 为基准；之后任何导致其变化的改动必须先提问。
4. ~~`asserts/` 目录~~ —— **已删除（2026-10-13）**：经负责人同意；图标与 `assets/icon/trellia-icon.png` 逐字节相同。

## CI（2026-10-13）

首次运行 <https://github.com/CIPFZ/trellia/actions/runs/38376052953>（提交 `7491d36`）：ubuntu-latest、windows-latest、
macos-latest（arm64）三个任务全部成功，耗时 4 分钟；每个任务均执行 fmt / clippy `-D warnings` / test / doc，非 Windows 任务另执行
`check_deps.sh`。`lane_demo_determinism` 在 macOS arm64 上同样与 golden 逐字节一致。仅有的注解：`actions/checkout@v4`
运行在 Node.js 20 上的弃用警告（不影响结果）与 runner 镜像迁移通知。

## Windows 实机验证（2026-10-13）

环境：Windows Server 2022（10.0.20348），x64，Rust 1.99.0 `x86_64-pc-windows-msvc` + MSVC BuildTools，
有声卡（High Definition Audio / USB Audio Device，Audiosrv 运行中）。源码为提交 `584ab5c` 的 `git archive`，
工作目录 `D:\trellia-verify`（独立 `CARGO_HOME` / `CARGO_TARGET_DIR`，未改动机器上共享的 Rust 工具链）。
输入以 pyautogui 注入，画面以桌面截图核对。

- [x] `cargo test --workspace`：100 通过 / 0 失败；**`lane_demo_determinism` 与 Linux 生成的 golden 逐字节一致**（跨平台确定性成立）
- [x] `cargo build --release` 三个窗口示例成功（1 分 08 秒）
- [x] M3 `sprites`：窗口最大化（客户区 1920 × 1017）后逻辑区域 x = 56 ～ 1863，缩放 1.4125，左右各 56 px 黑边；
  点击逻辑 (640, 360) / (200, 600) 的圆心实测 (959.3, 530.0) / (337.3, 869.0)，期望 (960, 531.5) / (338.5, 870.5)，误差 < 2 px；
  F2 全体翻转；标题栏帧耗时 0.14 ～ 0.29 ms（1000 精灵预算 4 ms）
- [x] M4：Esc 推入半透明暂停覆盖层、下方冻结可见；F1 面板（60 FPS / vsync）；暂停时点 3 次 Step，tick 12440 → 12443；
  滑杆拖到 4.00 后 `ticks/frame 4`；点击播放 blip：日志显示每次点击 symphonia 解码 vorbis 流，无设备错误
- [x] **实际发声**：以 Windows Core Audio 会话峰值表采样 `lane_demo` 进程的音频会话 3 秒，`max_peak = 0.92`（BGM 信号确实送达系统输出端点）
- [x] M5 `lane_demo`：无炮塔时进入 GAME OVER 覆盖层；点击 egui “Restart” 按钮重开（该点击未被游戏误当作放置）；
  放置格 0 / 1 两座炮塔后 5/5 击败进入 VICTORY（资源 275、2 枚在途弹丸，与 golden 记录的终局状态一致）；
  回车重开、Esc 暂停（2 秒内仅标题耗时文字变化）、空格继续；标题栏帧耗时 0.15 ～ 0.18 ms（预算 2 ms）

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

- 状态：✅ 完成（2026-10-13）：Linux（Xvfb）+ Windows 实机核对通过
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
  - [x] Windows 实机核对（见“Windows 实机验证”）
  - [x] 自动化测试：egui 7（输入映射 / 坐标 / 颜色 / 启动校验）、hello_window 1、sprites 3（精灵数、点击圆存活 1 秒、翻转）
- 行数（`*.rs`）：trellia-egui 579、hello_window 84、sprites 286
- 测试数：累计 75
- 遗留问题：~~翻转键待确认~~（ADR-007 已裁决）；未加入 CJK 字体（ADR-005 §6，示例仅 ASCII）

### M4 音频、场景、调试

- 状态：✅ 完成（2026-10-13）：Windows 实机交互与实际发声已核对
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
  - [x] Windows 实机发声与交互（会话峰值 0.92）
- 行数（`*.rs`）：audio 215、scene 562、devtools 298、egui 731、headless 453、sprites 427、trellia 77
- 测试数：累计 91（`cargo test --workspace` 通过数合计）
- 遗留问题：~~翻转键待确认~~（ADR-007 已裁决）

### M5 垂直切片 `lane_demo`

- 状态：✅ 完成（2026-10-13）：Windows 实机完整对局（胜 / 负 / 暂停 / 重开）已核对
- 交付：
  - [x] `examples/lane_demo`：`rules.rs`（常量、组件、每 tick 模拟）、`scenes.rs`（Battle / Paused / GameOver / Victory）、
    `render.rs`（占位渲染 `Placeholder` → `Rect + Text`、HUD、DebugDraw 格子与碰撞区间）
  - [x] `tests/golden/lane_demo_seed42.snap`（211 字节）
  - [x] `crates/trellia-headless/tests/lane_demo.rs`（`trellia-headless` 以 dev-dependency 引用 `lane_demo`）
  - [x] 修复：`AudioPlugin` 曾在同一帧 `Stage::First` 清掉 `Startup` 中发出的请求（BGM 丢失），改为只丢弃跨过帧末仍未被取走的请求
  - [x] 门面补齐 `audio` / `scene` / `devtools` / `egui` 重新导出；`docs/crates/trellia.md`；`docs/engine-v1-report.md`
- 验收：
  1. [x] 手动（Xvfb 等价核对）：放置两座炮塔、行者入场、弹丸射击；F1 面板 + DebugDraw 显示格子边界与行者碰撞区间；
     胜利后显示半透明 “VICTORY” 覆盖层与 Restart 按钮，点击后重开为干净的新对局（实体数 1、`scenes: Battle`）。
     暂停 / 重开 / 失败流程另由 `lane_demo_pause_and_restart` 覆盖。窗口缩放沿用 M3 已验证的 letterbox。实机复核待负责人。
  2. [x] `lane_demo_determinism`：两次运行快照逐字节相同，且与 golden 相同。golden 人工核对（逐字段解码）：
     炮塔 2 座（格 0 / 1，hp 3）、行者 0、在途弹丸 2、`LaneState { tick: 2876, money: 275, shots: 37, hits: 25,
     schedule: [143, 743, 1323, 1949, 2530], spawned: 5, killed: 5, outcome: Won }`；时间线：每个行者 5 次命中毙命，
     无行者到达炮塔，第 2876 tick 胜利
  3. [x] `lane_demo_win_with_two_turrets`：第 2876 tick 进入 Victory（< 4000）
  4. [x] `lane_demo_lose_without_turrets`：2500 tick 内进入 GameOver
  5. [x] `lane_demo_bite_uses_event_frame`：到达当 tick 炮塔 hp 仍为 3，恰在到达后第 20 tick（`bite_hit` 帧）降为 2
  6. [x] `lane_demo_audio`：`PlayMusic` 1 条，`PlaySfx(blip)` 37 条（≥ 10）
  7. [x] 性能：release + Xvfb 软件渲染下标题栏 `lane_demo | 0.36 ms` ～ `0.60 ms`（< 2 ms）
- 行数（`*.rs`）：lane_demo 991、headless 666（含集成测试）、audio 265、trellia 84
- 测试数：累计 100（含 3 个文档测试），SPEC 第 4 ～ 8、13 章列出的 37 个测试全部实现
- 遗留问题：见“待负责人确认的问题”与 `docs/engine-v1-report.md` 第 3 节

## 完成标准（AGENT.md 9）核对

- [x] M0 ～ M5 验收条件逐条打勾并附证据（手动核对项：Windows 实机 + Linux Xvfb）
- [x] `cargo test --workspace` 全绿，100 个测试 ≥ SPEC 列出的 37 个
- [x] `tests/golden/lane_demo_seed42.snap` 已提交
- [x] `docs/engine-v1-report.md` 已提交
- [x] 无 SPEC.md 1.4 非目标实现（无编辑器 / 脚本 / 反射 / 场景图层级 / 相机 / 粒子 / 物理 / 通用碰撞 / 网络 / 热重载 /
  多线程调度 / 变更检测 / Action Map / 手柄 / 音频总线混音 / 录制回放 UI；lane_demo 的一维车道碰撞属于游戏层）
