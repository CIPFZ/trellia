# ADR-006：音频、场景与调试的细节补充

- 编号：ADR-006
- 标题：trellia-audio / trellia-scene / trellia-devtools / 后端 UI 回调中 SPEC.md 未写清的细节
- 日期：2026-10-13
- 状态：已接受

## 背景与决策

### 1. `Scene` 需要 `Send + Sync`（SPEC.md 8）

`SceneCommands` 与 `SceneStack` 被规定为 Resource（`Send + Sync + 'static`），其中存放
`Box<dyn Scene>`。若 `Scene` 只要求 `'static`，二者无法成为 Resource。唯一自洽的解读是
`pub trait Scene: Send + Sync + 'static`。

### 2. `ScenePlugin` 的构造

`Plugin::build(&self, ..)` 只拿到 `&self`，无法把 `pub initial: Box<dyn Scene>` 移出。
决策：`ScenePlugin::new(initial: impl Scene)`，内部以私有 `RefCell<Option<Box<dyn Scene>>>` 保存，
`build` 时取出（同名插件不可重复添加，故只会取一次）。

### 3. 场景生命周期时机

- 初始场景在 `Stage::Startup` 中入栈并调用 `on_enter`，因此第一帧的 `fixed_update` 已能运行。
- 切换命令由 `ScenePlugin` 注册的 `PostUpdate` 系统执行；在其之后注册的其他 `PostUpdate`
  系统发出的命令顺延到下一帧（文档已说明）。
- `Replace`：对旧栈顶调用 `on_exit`，对新场景调用 `on_enter`；不对下方场景调用
  `on_resume` / `on_pause`（避免无意义的一次恢复 + 暂停）。
- `Clear`：自顶向下依次 `on_exit`。
- 应用命令后若栈为空，置 `AppExit.requested = true`。

### 4. 后端 UI 回调（SPEC.md 8 `Scene::ui`、9 “注册到后端的 UI 回调”）

框架 crate 不能依赖后端（R5），后端也不应硬依赖可选的 devtools。

决策：`trellia-scene` 提供资源 `UiHooks`（`fn(&mut World, &egui::Context)` 列表）与
`run_ui_hooks(world, ctx)`。`ScenePlugin` 注册场景 UI，`DevtoolsPlugin` 注册调试面板；
两个后端在每帧 `App::frame` 之后调用 `run_ui_hooks`（egui 后端在真实上下文中，headless 用
`egui::Context::run_ui` 驱动、不渲染）。UI 中的交互结果（如按钮发出的场景命令）在下一帧生效。

### 5. DebugDraw

- 默认关闭；`set_enabled(bool)` 供调试面板开关与测试使用；关闭时 push 为空操作。
- `rect` 画 1 逻辑像素描边（碰撞框），`line_h` 为 1 像素高的实心矩形，`text` 为 14 号白字。
- 在 `Stage::Render` 中写入 `Layer::DEBUG` 后清空（`DevtoolsPlugin` 应在 `GfxPlugin` 之后添加）。

### 6. 调试面板数据

- FPS / 帧耗时：最近 60 帧 `Time::real_dt` 的均值（框架层无法获得后端 CPU 耗时，见 ADR-005）。
- 本帧 tick 数：两帧之间 `Time::fixed_tick` 的差值。

### 7. 音频

- `AudioPlugin` 只在 `Assets<AudioClip>` 不存在时插入（它与 `AssetsPlugin` 插入的 `Assets<AudioBytes>`
  是同一类型，避免覆盖已加载的音频）。
- 未被后端 drain 的请求在下一帧 `Stage::First` 被清空，首次发生时 `log::warn!` 一次。
- `RodioBackend`：每个音效一个 `rodio::Player`，播完后在下次处理请求时回收；同一 clip 同时最多 8 路，
  超出则忽略新请求；音乐使用单一常驻 `Player`。音效实际音量 = 请求音量 × 音效总线音量；
  `SetBusVolume` 立即作用于对应总线上所有正在播放的 `Player`。设备打开失败时 `log::error!`
  并降级为静默（所有请求被忽略）。

### 8. HeadlessRunner 输入与音频

- `click(pos)`：同一帧内注入 `MousePressed(Left)` + `MouseReleased(Left)`，鼠标位置设为 `pos`
  并在之后的帧中保持；`press_key(k)` 同理注入按下 + 释放。
- 每帧结束后把 `AudioQueue` 全部 drain 进累积日志，`take_audio_log` 取走并清空日志。

## 后果

- 游戏层的场景类型需满足 `Send + Sync`（纯数据场景天然满足）。
- 场景 / 调试 UI 在 headless 下同样执行，可对 UI 逻辑做无窗口测试。

## 替代方案

- （1）放宽 `Resource` 为不要求 `Send + Sync`：改变 core 的公共约束，否决。
- （4）后端直接依赖 devtools：发布构建无法去掉调试面板，否决。
