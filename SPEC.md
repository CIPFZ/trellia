# Trellia 引擎规格说明书（v1）

> 状态：草案 v1，2026-10-13
> 适用范围：Trellia 引擎 v1（用于 PvZ-like 植物塔防游戏的最小可用版本）
> 读者：负责从 0 实现引擎的 AI 开发者与项目负责人
> 配套文档：`AGENT.md`（AI 开发者工作手册）、`GAME-SPEC.md`（游戏规格，引擎 M5 完成后另行编写）

---

## 0. 阅读指南与硬性约定

本文档中的关键词按以下含义理解：

- **必须（MUST）**：不满足即视为实现不合格。
- **禁止（MUST NOT）**：出现即视为实现不合格。
- **应当（SHOULD）**：默认遵守，偏离时需在代码注释与 PR 描述中说明理由。
- **可以（MAY）**：可选项。

实现者必须遵守的全局约定：

1. 本文档是唯一需求来源。遇到文档未覆盖的情形，选择**最简单且不破坏第 1 章设计原则**的实现，并在 `docs/decisions/` 下追加一条 ADR（Architecture Decision Record）记录。
2. 禁止为"未来可能的需求"预留抽象。每一个公共类型、trait、函数必须能指向第 2 章用例清单中的至少一条用例，否则不得加入。
3. 所有第三方依赖版本必须通过 `cargo add <crate>` 获取 crates.io 当前最新稳定版，并由 `Cargo.lock` 锁定。禁止凭记忆手写版本号。
4. 所有 crate 必须声明 `#![forbid(unsafe_code)]`。
5. CI 必须通过：`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`cargo doc --workspace --no-deps`。
6. 自然语言（文档、注释、提交信息）使用中文；标识符使用英文。

---

## 1. 定位、目标与非目标

### 1.1 一句话定位

Trellia 是一个用纯 Rust 编写的轻量 2D 游戏框架，面向"固定视角、网格世界、中等实体规模、强 UI"的游戏，首个消费者是 PvZ-like 植物塔防游戏。它复用成熟底层 crate（窗口、GPU、音频、解码），自研"底层设施之上、游戏逻辑之下"的框架层。

### 1.2 设计原则（按优先级排序）

1. **确定性优先**：固定步长模拟、种子化随机、游戏逻辑中以 tick 而非秒计时。同一关卡 + 同一种子 + 同一输入序列，运行结果必须逐 tick 一致。
2. **模拟与呈现严格分离**：游戏逻辑只依赖 `trellia-core`，不出现任何窗口、GPU、音频设备类型。模拟层必须能在无窗口环境（headless）下完整运行并被测试。
3. **数据驱动**：图集、动画等资产以 RON 文本描述，由 `serde` 反序列化，启动时校验。
4. **组合优于继承**：实体 = ID，行为由组件组合，系统对组件组合批处理（ECS）。
5. **模块即插件**：引擎每个子系统、游戏每个功能都以 `Plugin` 形式挂入 `App`。引擎核心对"植物""僵尸"一无所知。
6. **后端可替换**：渲染与音频通过 trait 抽象，egui 是第一个后端，headless 是第二个后端，二者必须同时存在。
7. **最小化**：只实现 PvZ v1 需要的功能。接口上留口子，代码上一行不多写。

### 1.3 从成熟引擎吸收的思想（落地对照）

| 来源 | 思想 | Trellia 中的落地 |
|---|---|---|
| Unity | GameObject + Component 组合 | `Entity` + 组件（hecs） |
| Unity | FixedUpdate / Update 分离、timeScale | `Stage::FixedUpdate` / `Stage::Update`、`Time::time_scale` |
| Unity | Coroutine | `Timer`（简化版，无栈式协程） |
| UE | Subsystem | `Resources`（全局单例，带类型键） |
| UE | Gameplay Effect | 引擎只提供 `Timer` 与组件机制；buff 系统由游戏层以常驻 `StatusEffects` 组件实现（见 4.3.5 设计建议） |
| UE / Godot | 委托 / Signal | `Events<T>` 类型化事件总线 |
| Godot | Scene Tree 实例化 | `SceneStack`（push / pop / replace，支持覆盖层） |
| Godot | Resource 一等公民 | `Handle<T>` + `Assets<T>` |
| Godot | AnimationPlayer 方法调用轨道 | `AnimationClip` 事件帧 → `AnimationEvent` |
| Godot | Tween | `Tween` 组件（线性 / ease-out） |
| Bevy | Plugin | `trait Plugin` |
| Bevy | Schedule 阶段 | `Stage` 枚举，固定顺序 |
| Bevy | Events 双缓冲 | `Events<T>` 存活两帧，自动清理 |
| Bevy / hecs | Archetype ECS 存储 | 直接使用 `hecs`，由 `World` 封装 |

### 1.4 明确的非目标（v1 禁止实现）

- 可视化编辑器、脚本语言（Lua / 蓝图）、运行时反射。
- 场景图父子变换层级、相机滚动 / 缩放、后处理、粒子系统。
- 物理引擎、通用碰撞检测（游戏层自己写车道一维碰撞）。
- 3D、WASM / Web、网络、代码热重载、资产热重载。
- 多线程系统调度、变更检测、Archetype 自研存储。
- 层级 Gameplay Tags、Action Map 输入映射、手柄支持。
- 音频总线 / 混音 / 空间音频。
- 输入录制与回放 UI（但第 12 章确定性规则必须保证将来可加）。

### 1.5 技术边界

- 纯 Rust。禁止 WASM 构建目标、WebView、JavaScript 运行时。
- 目标平台：Linux / macOS / Windows 桌面。
- 底层 crate 白名单：`eframe` / `egui`（窗口 + 绘制 + UI）、`rodio`（音频）、`image`（PNG 解码）、`serde` + `ron`（数据）、`hecs`（ECS 存储）、`glam`（数学）、`rand` + `rand_pcg`（随机）、`log` + `env_logger`（日志）、`thiserror` / `anyhow`（错误）。新增依赖必须写 ADR。
- 素材边界：引擎仓库中的所有示例素材必须为原创占位（几何图形、自绘像素图、CC0 音效），禁止包含任何第三方版权角色、美术或音乐。

---

## 2. 驱动需求：PvZ 用例清单

以下用例来自 PvZ-like 游戏的核心玩法，是引擎每一项功能的存在理由。编号形如 `UC-xx`，第 4 章起每个模块的功能必须引用至少一条。

### 2.1 世界与实体

| 编号 | 用例 |
|---|---|
| UC-01 | 场地为 5 行 × 9 列网格，每格可放置一个植物，部分植物可叠加（南瓜头类）。网格定义由游戏层负责，引擎只提供逻辑坐标系。 |
| UC-02 | 同屏实体规模：植物 ≤ 45，僵尸 ≤ 60，弹丸 ≤ 100，阳光 ≤ 20，特效 ≤ 100，合计 ≤ 500。 |
| UC-03 | 一个植物由多种能力组合而成（射击 + 冰冻弹、生产阳光、阻挡、延迟爆炸），需要按组件组合而非类型继承表达。 |
| UC-04 | 遍历僵尸时需要生成弹丸、销毁死亡实体，必须支持遍历中延迟创建 / 销毁。 |
| UC-05 | 僵尸被冰冻、减速、定身等状态可叠加且有时长，同一实体同一时刻可处于多种状态。 |
| UC-06 | 僵尸失去护甲（铁桶被打掉）、植物被套上南瓜头，属于低频结构性变化。 |

### 2.2 时间与模拟

| 编号 | 用例 |
|---|---|
| UC-10 | 游戏逻辑以固定 60 tick/s 推进，帧率波动不影响游戏速度。 |
| UC-11 | 暂停时模拟停止但 UI 可交互；支持 2 倍速（开发调试用）。 |
| UC-12 | 卡牌冷却、僵尸出生倒计时、土豆雷武装延迟等"N tick 后触发"的计时需求。 |
| UC-13 | 同一关卡 + 同一随机种子 + 同一输入序列，两次运行逐 tick 一致，用于回归测试。 |
| UC-14 | 僵尸生成的随机性（选哪一行、哪一种）必须可复现。 |

### 2.3 渲染

| 编号 | 用例 |
|---|---|
| UC-20 | 设计分辨率固定（逻辑坐标 1280 × 720），窗口任意尺寸时等比缩放并留黑边（letterbox）。 |
| UC-21 | 精灵按"层 → 行（y）→ 提交顺序"排序绘制：背景在底，同一行内靠下者遮挡靠上者，UI 在顶。 |
| UC-22 | 精灵支持序列帧动画，循环或单次播放，播放速度可缩放（被冰冻时动画变慢）。 |
| UC-23 | 动画特定帧触发游戏事件（僵尸啃食动画的第 N 帧才真正扣血，射手出手帧才生成弹丸）。 |
| UC-24 | 精灵支持着色（受伤闪白、冰冻变蓝）、水平翻转、半透明（铲子悬停预览）。 |
| UC-25 | 需要绘制纯色矩形 / 圆（占位素材模式、调试碰撞框）与文字（阳光数、波次提示）。 |
| UC-26 | 渲染在两个 tick 之间插值，使 60 tick 模拟在 144 Hz 屏幕上依然平滑。 |
| UC-27 | 在素材尚未就绪时，以几何图形 + 文字标签运行完整游戏（占位渲染模式）。 |

### 2.4 资产

| 编号 | 用例 |
|---|---|
| UC-30 | 加载 PNG 纹理图集、RON 描述的图集元数据与动画片段、OGG 音频。 |
| UC-31 | 通过轻量句柄引用资产，组件中存句柄而非数据本体。 |
| UC-32 | 资产缺失或格式错误时在启动阶段报错并给出文件路径，而非运行中 panic。 |

### 2.5 输入

| 编号 | 用例 |
|---|---|
| UC-40 | 鼠标在逻辑坐标系中的位置、左 / 右键按下 / 刚按下 / 刚释放。 |
| UC-41 | 少量键盘键：Esc（暂停）、空格（继续）、数字键（快捷选卡）、F1（调试面板）。 |
| UC-42 | 输入在每帧开始时快照一次，一帧内所有系统看到一致的输入状态。 |

### 2.6 音频

| 编号 | 用例 |
|---|---|
| UC-50 | 播放一次性音效（发射、命中、种植、收集阳光），同一音效可短时间内重叠播放。 |
| UC-51 | 循环播放背景音乐，切换场景时切换 BGM。 |
| UC-52 | 分别调节音效与音乐音量。 |
| UC-53 | 模拟层发出"播放音效"意图而不直接依赖音频设备，headless 下可断言"某音效被请求播放"。 |

### 2.7 场景与流程

| 编号 | 用例 |
|---|---|
| UC-60 | 主菜单 → 关卡选择 → 选卡 → 战斗 → 结算，场景间切换。 |
| UC-61 | 暂停菜单覆盖在战斗场景之上，战斗场景不销毁、不更新、仍绘制。 |
| UC-62 | 场景切换请求可在任意系统中发出，在帧末统一执行。 |

### 2.8 工程与调试

| 编号 | 用例 |
|---|---|
| UC-70 | 开发时显示 FPS、帧耗时、tick 计数、实体数量。 |
| UC-71 | 开发时可拖动时间缩放、暂停、单步推进一个 tick。 |
| UC-72 | 开发时可叠加绘制游戏层提交的调试形状（碰撞区间、格子边界）。 |
| UC-73 | 对"豌豆射手 N 发打死普通僵尸"这类数值规则编写无窗口单元测试。 |
| UC-74 | 对"给定关卡与固定种植策略，第 N 波前不输"这类流程编写无窗口集成测试。 |

---

## 3. 架构总览

### 3.1 分层

```
┌─────────────────────────────────────────────────────────┐
│ 游戏层（独立仓库 / workspace，本文档不涉及实现）            │
│   pvz-core（规则、实体、关卡，仅依赖 trellia-core）         │
│   pvz-app（组装 Plugin、UI、入口）                         │
├─────────────────────────────────────────────────────────┤
│ Trellia 框架层                                           │
│   trellia-core   App / Plugin / Schedule / World / Time  │
│                  Events / Resources / Timer / Tween      │
│                  Rng / Input 状态                         │
│   trellia-assets Handle / Assets / 加载器                 │
│   trellia-gfx    DrawList / Sprite / Atlas / Animation   │
│   trellia-audio  AudioClip / AudioQueue                  │
│   trellia-scene  SceneStack                              │
│   trellia-devtools 调试面板                               │
│   trellia        门面 crate：re-export + prelude          │
├─────────────────────────────────────────────────────────┤
│ 后端层（实现框架层定义的 trait，驱动主循环）                 │
│   trellia-egui      eframe 窗口 + egui Painter + rodio    │
│   trellia-headless  无窗口，记录 DrawList 与音频请求       │
├─────────────────────────────────────────────────────────┤
│ 第三方 crate                                             │
│   eframe / egui / rodio / image / serde / ron / hecs     │
│   glam / rand / rand_pcg / log / thiserror / anyhow      │
└─────────────────────────────────────────────────────────┘
```

### 3.2 依赖规则（CI 强制）

| 规则 | 说明 |
|---|---|
| R1 | `trellia-core` 禁止依赖任何其他 trellia crate，禁止依赖 `egui` / `eframe` / `rodio` / `image`。允许 `hecs` / `glam` / `rand` / `rand_pcg` / `serde` / `log` / `thiserror`。 |
| R2 | `trellia-assets` / `trellia-gfx` / `trellia-audio` / `trellia-scene` 只允许依赖 `trellia-core`（`trellia-gfx` 额外允许依赖 `trellia-assets`）。彼此之间禁止互相依赖。 |
| R3 | `egui`（核心 crate，不含窗口）允许出现在 `trellia-scene`（`Scene::ui` 钩子）、`trellia-devtools` 和后端中。`eframe` / `egui-winit` / `rodio` / `image` 只允许出现在后端 crate 与 `trellia-assets`（`image` 仅用于解码）。 |
| R4 | 游戏层的 `pvz-core` 只允许依赖 `trellia-core`。这是模拟层 headless 可测的根本保障。 |
| R5 | 后端 crate 禁止被框架层 crate 依赖。 |

CI 中以脚本检查各 crate 的 `Cargo.toml` `[dependencies]` 是否符合上表（实现方式：`cargo metadata` + 白名单比对）。

关于 UI 的决策说明：Trellia 将 `egui` 指定为内置 UI 工具包（类比 Godot 的 Control 节点），HUD、菜单、卡槽等 UI 直接使用 egui 编写；世界内绘制（精灵、特效）走 `DrawList` 抽象以保证渲染后端可替换。`egui` 核心 crate 不依赖窗口系统，headless 后端可用 `egui::Context::run` 驱动 UI 逻辑而不渲染，因此该决策不破坏 headless 可测性。

### 3.3 Workspace 布局

```
trellia/
├── SPEC.md                    # 本文档（唯一需求来源）
├── AGENT.md                   # AI 开发者工作手册
├── Cargo.toml                 # workspace，统一 [workspace.dependencies]
├── rust-toolchain.toml        # 锁定 stable 通道
├── .github/workflows/ci.yml   # 或等价 CI 配置
├── crates/
│   ├── trellia-core/
│   ├── trellia-assets/
│   ├── trellia-gfx/
│   ├── trellia-audio/
│   ├── trellia-scene/
│   ├── trellia-devtools/
│   ├── trellia-egui/
│   ├── trellia-headless/
│   └── trellia/               # 门面
├── examples/
│   ├── hello_window/          # M3 验收
│   ├── sprites/               # M3 验收：动画方块
│   └── lane_demo/             # M5 验收：垂直切片
├── assets/
│   ├── icon/                  # 项目图标
│   └── demo/*.png *.ron *.ogg # 示例用原创占位素材
├── docs/
│   ├── decisions/             # ADR
│   ├── crates/                # 每个 crate 一页使用说明（M 阶段产出）
│   └── progress.md            # 里程碑进度（AGENT.md 要求维护）
└── scripts/
    └── check_deps.sh          # 依赖规则检查
```

每个 crate 必须包含：`src/lib.rs`（顶部模块级文档说明职责与用例引用）、`README.md`（50 行以内的用法）、至少一个 `#[cfg(test)]` 模块。

### 3.4 帧流程

每一帧由后端驱动，调用 `App::frame(FrameInput) -> &FrameOutput`：

```
后端采集窗口事件、真实 dt
    │
    ▼
App::frame
    ├─ Stage::First        Events 双缓冲翻页；Input 快照写入
    ├─ Stage::PreUpdate    资产后处理、场景栈 on_enter 等
    ├─ Stage::FixedUpdate  (0..=N 次，N 由累加器决定，上限 5)
    │     每次 tick 前：Transform2D.prev ← pos；Time.fixed_tick += 1
    │     运行所有注册在 FixedUpdate 的系统 + 顶层 Scene::fixed_update
    │     tick 末：CommandBuffer 应用（延迟 spawn/despawn）
    ├─ Stage::Update       每帧一次：Tween 推进（视觉）、Scene::update
    ├─ Stage::PostUpdate   CommandBuffer 应用；SceneStack 应用切换请求
    ├─ Stage::Render       精灵系统生成 DrawList；Scene::draw；Devtools 叠加
    └─ Scene::ui           egui UI（由后端在合适时机回调）
    │
    ▼
FrameOutput { draw_list, audio_requests, exit_requested }
    │
    ▼
后端提交 DrawList 到 GPU / 丢弃（headless），执行音频请求
```

固定步长累加器规则：`accumulator += real_dt * time_scale`；当 `accumulator >= fixed_dt` 时执行一个 tick 并减去 `fixed_dt`；单帧最多 5 个 tick，超出部分丢弃（防止"死亡螺旋"）。`Time::paused == true` 时不累加。渲染插值因子 `alpha = accumulator / fixed_dt`，范围 `[0, 1)`。

---

## 4. trellia-core

职责：应用骨架、ECS 封装、时间、事件、资源、计时器、补间、随机数、输入状态。它是唯一允许被游戏模拟层依赖的 crate。

### 4.1 App 与 Plugin

```rust
pub struct App { /* 私有 */ }

impl App {
    pub fn new() -> Self;
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self;
    pub fn add_system(&mut self, stage: Stage, system: impl System) -> &mut Self;
    pub fn insert_resource<R: Resource>(&mut self, res: R) -> &mut Self;
    pub fn world(&self) -> &World;
    pub fn world_mut(&mut self) -> &mut World;

    /// 后端每帧调用一次。返回本帧产出，由后端消费。
    pub fn frame(&mut self, input: FrameInput) -> &FrameOutput;
    /// 便捷方法：仅推进 n 个固定 tick，不经过真实时间累加（headless 测试用）。
    pub fn step_ticks(&mut self, n: u32);
}

pub trait Plugin: 'static {
    fn name(&self) -> &'static str;
    fn build(&self, app: &mut App);
}

pub type System = fn(&mut World);
```

规则：

- 系统签名统一为 `fn(&mut World)`，不做参数注入（避免宏与运行时借用检查）。系统内部通过 `world.resources` / `world.entities` / `world.events` 字段分别借用。
- 同一 `Stage` 内系统按注册顺序执行。Plugin 的注册顺序由 `add_plugin` 调用顺序决定。不提供 before / after 标签（UC 中无此需求）。
- 同名 Plugin 重复添加时 panic，错误信息包含插件名。
- `FrameInput` / `FrameOutput` 定义见 4.9。

### 4.2 Stage

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stage {
    Startup,      // 仅第一帧执行一次
    First,
    PreUpdate,
    FixedUpdate,  // 每 tick 执行，可能 0 次或多次
    Update,
    PostUpdate,
    Render,
}
```

### 4.3 World、Entity 与组件

`World` 是三个独立容器的聚合，字段公开以便系统分别借用：

```rust
pub struct World {
    pub entities: hecs::World,
    pub resources: Resources,
    pub events: EventBus,
    pub commands: CommandBuffer,   // 对 hecs::CommandBuffer 的薄封装
}

pub use hecs::Entity;

impl World {
    pub fn spawn(&mut self, bundle: impl hecs::DynamicBundle) -> Entity;
    pub fn despawn(&mut self, e: Entity) -> Result<(), NoSuchEntity>;
    pub fn get<T: Component>(&self, e: Entity) -> Option<hecs::Ref<'_, T>>;
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<hecs::RefMut<'_, T>>;
    pub fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<'_, Q>;
    pub fn query_mut<Q: hecs::Query>(&mut self) -> hecs::QueryMut<'_, Q>;
    pub fn contains(&self, e: Entity) -> bool;
    pub fn entity_count(&self) -> usize;

    /// 应用延迟命令。由 App 在每个 tick 末与 PostUpdate 自动调用，系统不应手动调用。
    pub(crate) fn apply_commands(&mut self);
}

pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}
```

#### 4.3.1 hecs 使用约定

- hecs 的类型可以出现在 `trellia-core` 公共 API 中（`Entity`、`Query`、`DynamicBundle`），因为自研替换存储不在 v1 范围，没必要为此付出双层封装的代价。此决定记录为 ADR-001。
- 禁止在系统中持有 `QueryBorrow` 的同时调用 `world.spawn` / `despawn`。遍历中需要创建 / 销毁时必须使用 `world.commands`（UC-04）。

#### 4.3.2 CommandBuffer

```rust
pub struct CommandBuffer { /* 封装 hecs::CommandBuffer */ }

impl CommandBuffer {
    pub fn spawn(&mut self, bundle: impl hecs::DynamicBundle);
    pub fn despawn(&mut self, e: Entity);
    pub fn insert(&mut self, e: Entity, bundle: impl hecs::DynamicBundle);
    pub fn remove<T: hecs::Bundle + 'static>(&mut self, e: Entity);
}
```

应用时机：每个 FixedUpdate tick 结束后、PostUpdate 结束后。对已销毁实体执行 `insert` / `remove` 时静默忽略并记录 `log::warn!`。

#### 4.3.3 内置组件

`trellia-core` 只定义一个与渲染无关的空间组件：

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub pos: glam::Vec2,       // 逻辑坐标，像素单位，原点左上，y 向下
    pub prev_pos: glam::Vec2,  // 上一 tick 的 pos，由 App 在每 tick 开始时同步（UC-26）
}

impl Transform2D {
    pub fn new(pos: Vec2) -> Self;          // prev_pos = pos
    pub fn teleport(&mut self, pos: Vec2);  // 同时设置 pos 与 prev_pos，避免插值拖影
    pub fn lerp(&self, alpha: f32) -> Vec2; // prev_pos + (pos - prev_pos) * alpha
}
```

渲染相关组件（`Sprite`、`Animator`）定义在 `trellia-gfx`。

#### 4.3.4 组件设计建议（写给游戏层）

- 高频开关的状态（冰冻、减速、定身，UC-05）禁止用加减组件表达，应做成常驻 `StatusEffects` 组件，内部记录各效果剩余 tick。原因：hecs 为 Archetype 存储，加减组件会触发实体搬迁。
- 低频结构性变化（UC-06）可以使用 `commands.insert` / `commands.remove`。
- 组件应为 `Copy` 或小型 `Clone` 的纯数据，不含 `Box<dyn Trait>`、闭包或句柄以外的引用。

### 4.4 Resources

```rust
pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

pub struct Resources { /* HashMap<TypeId, Box<dyn Any + Send + Sync>> */ }

impl Resources {
    pub fn insert<R: Resource>(&mut self, r: R) -> Option<R>;
    pub fn remove<R: Resource>(&mut self) -> Option<R>;
    pub fn get<R: Resource>(&self) -> Option<&R>;
    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R>;
    /// 不存在时 panic，错误信息包含类型名。用于引擎保证一定存在的资源（Time、Input、GameRng）。
    pub fn expect<R: Resource>(&self) -> &R;
    pub fn expect_mut<R: Resource>(&mut self) -> &mut R;
    pub fn contains<R: Resource>(&self) -> bool;
}
```

引擎保证在 `Startup` 前插入的资源：`Time`、`Input`、`GameRng`、`Viewport`（后者由 `trellia-gfx` 插入）。

### 4.5 Events

```rust
pub trait Event: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Event for T {}

pub struct Events<T: Event> { /* 双缓冲：current, previous */ }

impl<T: Event> Events<T> {
    pub fn send(&mut self, e: T);
    /// 迭代上一帧与本帧的全部事件（先旧后新）。
    pub fn iter(&self) -> impl Iterator<Item = &T>;
    /// 取走本帧与上一帧全部事件，常用于只应被一个消费者处理的事件。
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_;
    pub fn is_empty(&self) -> bool;
    pub(crate) fn swap(&mut self);  // Stage::First 调用：previous ← current，current 清空
}

pub struct EventBus { /* HashMap<TypeId, Box<dyn AnyEvents>> */ }

impl EventBus {
    pub fn register<T: Event>(&mut self);     // Plugin::build 中调用；重复注册忽略
    pub fn send<T: Event>(&mut self, e: T);    // 未注册时 panic，信息含类型名
    pub fn get<T: Event>(&self) -> &Events<T>;
    pub fn get_mut<T: Event>(&mut self) -> &mut Events<T>;
}
```

生命周期：事件在发送后的"本帧剩余部分 + 下一整帧"内可读，之后自动丢弃（UC 中音效、UI 消费模拟事件的典型模式：FixedUpdate 发送，同帧 Render 或下一帧 Update 消费）。

事件必须为纯数据（`Clone + Debug`），禁止携带引用或句柄以外的资源。

### 4.6 Time

```rust
pub struct Time {
    pub fixed_dt: f32,          // 固定为 1.0 / 60.0，只读
    pub fixed_tick: u64,        // 自 Startup 起已执行的 tick 总数
    pub frame: u64,             // 已渲染帧数
    pub real_dt: f32,           // 本帧真实秒数（未缩放，未钳制）
    pub time_scale: f32,        // 默认 1.0；范围 [0.0, 4.0]
    pub paused: bool,           // true 时 FixedUpdate 不执行，Update / Render 照常
    pub alpha: f32,             // 本帧渲染插值因子 [0, 1)
    pub step_once: bool,        // 为 true 时无视 paused 执行恰好 1 tick 后复位（UC-71）
}
```

游戏逻辑中的时长一律以 tick 为单位（`u32`），禁止在 FixedUpdate 系统中使用 `real_dt`。

### 4.7 Timer 与 Tween

```rust
/// 以 tick 计数的倒计时（UC-12）。纯数据，由持有者每 tick 调用 tick()。
#[derive(Clone, Copy, Debug)]
pub struct Timer {
    pub remaining: u32,
    pub duration: u32,
    pub repeating: bool,
}

impl Timer {
    pub fn once(ticks: u32) -> Self;
    pub fn repeating(ticks: u32) -> Self;
    /// 推进一个 tick。返回 true 表示本 tick 触发。repeating 触发后自动重置。
    pub fn tick(&mut self) -> bool;
    pub fn reset(&mut self);
    pub fn finished(&self) -> bool;
    pub fn progress(&self) -> f32;  // 0.0 → 1.0
}

/// 视觉补间（阳光飘落、卡牌弹入），按真实帧时间推进，不参与确定性（UC 视觉）。
#[derive(Clone, Copy, Debug)]
pub enum Easing { Linear, EaseOutQuad }

#[derive(Clone, Copy, Debug)]
pub struct TweenF32 {
    pub from: f32, pub to: f32,
    pub duration_secs: f32, pub elapsed_secs: f32,
    pub easing: Easing,
}

impl TweenF32 {
    pub fn new(from: f32, to: f32, duration_secs: f32, easing: Easing) -> Self;
    pub fn advance(&mut self, dt: f32) -> f32;  // 返回当前值
    pub fn value(&self) -> f32;
    pub fn finished(&self) -> bool;
}

pub type TweenVec2 = /* 同上，from/to 为 Vec2 */;
```

Tween 是纯数据工具类型，引擎不提供自动推进系统；游戏层在 `Stage::Update` 中自行推进。规则：Tween 的结果禁止影响任何 FixedUpdate 逻辑（例如"阳光飘到计数器后才加分"应改为"点击即加分，飘落仅为视觉"）。

### 4.8 GameRng

```rust
pub struct GameRng(rand_pcg::Pcg32);

impl GameRng {
    pub fn from_seed(seed: u64) -> Self;
    pub fn seed(&self) -> u64;
    pub fn gen_range<T>(&mut self, range: impl SampleRange<T>) -> T;
    pub fn gen_bool(&mut self, p: f64) -> bool;
    pub fn shuffle<T>(&mut self, slice: &mut [T]);
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T>;
}
```

规则：FixedUpdate 系统中的一切随机性必须来自 `GameRng`（UC-13 / UC-14）。禁止 `rand::thread_rng()`、禁止用时间做种子（种子由游戏层在进入关卡时显式提供，默认 `0`）。视觉用随机（粒子抖动等）可使用独立的 `VisualRng` 资源，由游戏层自行插入。

### 4.9 Input 与帧 IO

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton { Left, Right, Middle }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    Escape, Space, Enter, F1, F2, F3,
    Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Digit0,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    // v1 仅此。新增需引用用例。
}

pub struct Input {
    /// 鼠标在逻辑坐标系中的位置；光标不在窗口内时为 None。
    pub mouse_pos: Option<Vec2>,
    /* 私有：按钮 / 按键的 down / just_pressed / just_released 集合 */
}

impl Input {
    pub fn mouse_down(&self, b: MouseButton) -> bool;
    pub fn mouse_just_pressed(&self, b: MouseButton) -> bool;
    pub fn mouse_just_released(&self, b: MouseButton) -> bool;
    pub fn key_down(&self, k: Key) -> bool;
    pub fn key_just_pressed(&self, k: Key) -> bool;
    pub fn key_just_released(&self, k: Key) -> bool;
}

/// 后端每帧传入。坐标为物理窗口像素，由 App 结合 Viewport 转为逻辑坐标。
pub struct FrameInput {
    pub real_dt: f32,
    pub window_size_px: (u32, u32),
    pub mouse_pos_px: Option<(f32, f32)>,
    pub events: Vec<InputEvent>,
}

pub enum InputEvent {
    MousePressed(MouseButton), MouseReleased(MouseButton),
    KeyPressed(Key), KeyReleased(Key),
    CloseRequested,
}

pub struct FrameOutput {
    pub exit_requested: bool,
    // draw_list 与 audio_requests 由 trellia-gfx / trellia-audio 以资源形式存放在 World 中，
    // 后端自行读取（避免 trellia-core 依赖这两个 crate）。
}
```

`just_pressed` / `just_released` 仅在事件所在帧的 `First` 之后到下一帧 `First` 之前为真（UC-42）。多个 FixedUpdate tick 共享同一帧输入快照，游戏层若需"每帧只响应一次点击"，应在首个 tick 处理后自行记录。

### 4.10 错误与日志

- 引擎内部错误类型使用 `thiserror`，按 crate 各定义一个 `Error` 枚举。
- 可恢复错误（资产缺失、RON 解析失败）必须以 `Result` 返回到 `App` 启动阶段并终止启动，错误信息包含文件路径与原因。
- 不可恢复的编程错误（访问未注册事件、重复 Plugin）使用 `panic!`，信息包含类型名或插件名。
- 日志使用 `log` 门面，后端负责初始化 `env_logger`。模拟层每 tick 的日志不得高于 `trace` 级别。

### 4.11 测试要求

`trellia-core` 必须包含以下单元测试：

| 测试 | 断言 |
|---|---|
| `fixed_step_accumulates` | `real_dt = 1/30` 时一帧恰好执行 2 tick；`real_dt = 1.0` 时执行 5 tick（上限） |
| `paused_skips_fixed_update` | `paused = true` 时 `fixed_tick` 不增长，`frame` 增长 |
| `step_once_runs_exactly_one_tick` | 暂停状态下 `step_once = true` 执行 1 tick 后复位 |
| `events_live_two_frames` | 第 N 帧发送，第 N 与 N+1 帧可读，第 N+2 帧不可读 |
| `commands_apply_at_tick_end` | tick 中 `commands.spawn` 后 `entity_count` 不变，tick 末变化 |
| `rng_deterministic` | 同种子两次生成序列相同；不同种子不同 |
| `transform_prev_synced_each_tick` | tick 开始 `prev_pos == 上一 tick 的 pos` |
| `input_just_pressed_single_frame` | 同一事件在下一帧 `just_pressed == false` |
| `timer_repeating_resets` | 周期 3 的 Timer 在第 3、6、9 tick 触发 |
| `determinism_smoke` | 一个含 RNG、Timer、commands 的小型系统集合，两次 `step_ticks(600)` 后 World 快照（排序后的组件序列）逐字节相同 |


---

## 5. trellia-assets

职责：资产句柄、同步加载、格式解码。v1 不做异步加载与热重载（UC-30 / UC-31 / UC-32）。

### 5.1 Handle 与 Assets

```rust
/// 轻量、Copy、带类型的资产句柄。内部为 u32 索引，不做代际校验（v1 资产不卸载）。
pub struct Handle<T> { id: u32, _marker: PhantomData<fn() -> T> }
// 手动实现 Clone / Copy / PartialEq / Eq / Hash / Debug，不要求 T 满足这些 trait。

pub trait Asset: Send + Sync + 'static {}

pub struct Assets<T: Asset> { items: Vec<T>, by_path: HashMap<String, Handle<T>> }

impl<T: Asset> Assets<T> {
    pub fn add(&mut self, item: T) -> Handle<T>;
    pub fn add_with_path(&mut self, path: &str, item: T) -> Handle<T>;  // 同路径重复加载返回已有句柄
    pub fn get(&self, h: Handle<T>) -> Option<&T>;
    pub fn get_mut(&mut self, h: Handle<T>) -> Option<&mut T>;
    pub fn expect(&self, h: Handle<T>) -> &T;                           // 不存在时 panic
    pub fn handle_by_path(&self, path: &str) -> Option<Handle<T>>;
    pub fn len(&self) -> usize;
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)>;
}
```

每种资产类型的 `Assets<T>` 作为独立 `Resource` 存入 `World.resources`。

### 5.2 AssetServer

```rust
pub struct AssetServer { root: PathBuf }

impl AssetServer {
    /// root 解析顺序：环境变量 TRELLIA_ASSETS → 可执行文件同级 assets/ → CARGO_MANIFEST_DIR/assets/
    pub fn new() -> Result<Self, AssetError>;
    pub fn with_root(root: impl Into<PathBuf>) -> Self;
    pub fn root(&self) -> &Path;
    pub fn read_bytes(&self, rel: &str) -> Result<Vec<u8>, AssetError>;
    pub fn read_string(&self, rel: &str) -> Result<String, AssetError>;
    /// 通用 RON 加载。T 为任意 DeserializeOwned。
    pub fn load_ron<T: DeserializeOwned>(&self, rel: &str) -> Result<T, AssetError>;
    /// PNG → CPU 侧 RGBA8 图像。GPU 上传由后端在首次绘制时惰性完成。
    pub fn load_image(&self, rel: &str) -> Result<Image, AssetError>;
    /// OGG / WAV 原始字节（不解码，交给音频后端）。
    pub fn load_audio(&self, rel: &str) -> Result<AudioBytes, AssetError>;
}

pub struct Image { pub width: u32, pub height: u32, pub rgba: Vec<u8> }
pub struct AudioBytes { pub bytes: Arc<[u8]>, pub kind: AudioKind }
pub enum AudioKind { Ogg, Wav }

#[derive(thiserror::Error, Debug)]
pub enum AssetError {
    #[error("资产根目录不存在: {0}")] RootNotFound(PathBuf),
    #[error("读取失败 {path}: {source}")] Io { path: PathBuf, source: std::io::Error },
    #[error("RON 解析失败 {path}: {source}")] Ron { path: PathBuf, source: ron::error::SpannedError },
    #[error("图像解码失败 {path}: {source}")] Image { path: PathBuf, source: image::ImageError },
    #[error("不支持的音频格式 {path}")] UnsupportedAudio { path: PathBuf },
    #[error("校验失败 {path}: {reason}")] Validation { path: PathBuf, reason: String },
}
```

`AssetsPlugin` 在 `Startup` 前插入 `AssetServer`、`Assets<Image>`、`Assets<AudioBytes>`。游戏层在自己的 Plugin `build` 中同步加载全部资产；加载失败通过 `App::frame` 第一帧前返回错误终止程序。

### 5.3 测试要求

| 测试 | 断言 |
|---|---|
| `handle_is_copy_and_typed` | `Handle<Image>` 与 `Handle<AudioBytes>` 不可互换（编译期），`Copy` 成立 |
| `add_with_path_dedups` | 同路径两次 `add_with_path` 返回相同句柄且 `len == 1` |
| `missing_file_error_contains_path` | 加载不存在文件的错误信息包含完整路径 |
| `load_ron_roundtrip` | 写入临时 RON 后 `load_ron` 得到等价结构 |
| `load_png_dimensions` | 加载 `assets/demo/placeholder_8x8.png` 得到 8 × 8 且 `rgba.len() == 256` |

---

## 6. trellia-gfx

职责：逻辑坐标系与视口、绘制命令列表、精灵与图集、序列帧动画。本 crate 不接触 GPU。

### 6.1 Viewport 与坐标系

```rust
pub struct Viewport {
    pub logical_size: Vec2,       // 固定 (1280, 720)，由 GfxPlugin::new(logical_size) 指定
    pub window_size_px: Vec2,     // 每帧由 App 从 FrameInput 更新
    pub scale: f32,               // min(window.x / logical.x, window.y / logical.y)
    pub offset_px: Vec2,          // letterbox 偏移（UC-20）
}

impl Viewport {
    pub fn logical_to_px(&self, p: Vec2) -> Vec2;
    pub fn px_to_logical(&self, p: Vec2) -> Vec2;
    pub fn logical_rect(&self) -> Rect;  // (0,0)-(1280,720)
}
```

逻辑坐标：原点左上，x 向右，y 向下，单位为逻辑像素。所有游戏层坐标、`Transform2D`、`DrawList` 均使用逻辑坐标。`Input.mouse_pos` 已转换为逻辑坐标。

### 6.2 基础类型

```rust
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect { pub min: Vec2, pub max: Vec2 }
impl Rect {
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self;
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self;
    pub fn size(&self) -> Vec2; pub fn center(&self) -> Vec2;
    pub fn contains(&self, p: Vec2) -> bool;
    pub fn intersects(&self, other: &Rect) -> bool;
    pub fn translate(&self, d: Vec2) -> Rect;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }  // 线性 0..1
impl Color {
    pub const WHITE: Color; pub const BLACK: Color; pub const TRANSPARENT: Color;
    pub fn rgb(r: f32, g: f32, b: f32) -> Self;
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;
    pub fn from_hex(hex: u32) -> Self;      // 0xRRGGBB
    pub fn with_alpha(self, a: f32) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer(pub i16);
impl Layer {
    pub const BACKGROUND: Layer = Layer(-100);
    pub const WORLD: Layer = Layer(0);
    pub const PROJECTILE: Layer = Layer(10);
    pub const FX: Layer = Layer(20);
    pub const OVERLAY: Layer = Layer(100);
    pub const DEBUG: Layer = Layer(1000);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign { Left, Center, Right }
```

### 6.3 DrawList

```rust
pub struct DrawList { cmds: Vec<DrawCmd>, next_seq: u32 }

#[derive(Clone, Debug)]
pub struct DrawCmd {
    pub layer: Layer,
    pub sort_y: f32,        // 同层内按此排序，小者先绘制（UC-21）
    pub seq: u32,           // 提交顺序，前两者相同时的稳定排序键
    pub kind: DrawKind,
}

#[derive(Clone, Debug)]
pub enum DrawKind {
    Sprite { image: Handle<Image>, src_px: Rect, dst: Rect, tint: Color, flip_x: bool },
    Rect   { rect: Rect, color: Color, stroke: Option<f32> },   // stroke=None 实心
    Circle { center: Vec2, radius: f32, color: Color, stroke: Option<f32> },
    Text   { pos: Vec2, text: String, size: f32, color: Color, align: TextAlign },
}

impl DrawList {
    pub fn clear(&mut self);
    pub fn sprite(&mut self, layer: Layer, sort_y: f32, image: Handle<Image>, src_px: Rect, dst: Rect) -> &mut DrawCmdBuilder;
    pub fn rect(&mut self, layer: Layer, rect: Rect, color: Color);
    pub fn rect_stroke(&mut self, layer: Layer, rect: Rect, color: Color, width: f32);
    pub fn circle(&mut self, layer: Layer, center: Vec2, radius: f32, color: Color);
    pub fn text(&mut self, layer: Layer, pos: Vec2, text: impl Into<String>, size: f32, color: Color);
    pub fn text_aligned(&mut self, /* 同上 */, align: TextAlign);
    /// 排序后供后端消费。排序键 (layer, sort_y, seq)，稳定排序。
    pub fn sorted(&mut self) -> &[DrawCmd];
    pub fn len(&self) -> usize;
}
```

`DrawList` 作为 `Resource` 存在 World 中，`Stage::Render` 开始时由 `GfxPlugin` 清空，结束时排序。后端读取 `world.resources.expect::<DrawList>().sorted()`。`Rect` / `Circle` / `Text` 三类命令直接满足 UC-25 与 UC-27 占位渲染模式。

### 6.4 Sprite 与 Atlas

```rust
/// 图集元数据，RON 文件描述（格式见第 10 章）。
pub struct Atlas {
    pub image: Handle<Image>,
    pub frames: Vec<AtlasFrame>,
    pub by_name: HashMap<String, usize>,
}
pub struct AtlasFrame {
    pub name: String,
    pub src_px: Rect,          // 在图像中的像素矩形
    pub anchor: Vec2,          // 归一化锚点 (0..1)，(0.5, 1.0) 表示底部中心
}
impl Atlas { pub fn frame(&self, name: &str) -> Option<usize>; }

/// 渲染组件。与 Transform2D 配合使用。
#[derive(Clone, Debug)]
pub struct Sprite {
    pub atlas: Handle<Atlas>,
    pub frame: usize,          // 当前帧索引（Animator 会写入）
    pub layer: Layer,
    pub tint: Color,           // 默认 WHITE（UC-24）
    pub flip_x: bool,
    pub scale: f32,            // 默认 1.0
    pub visible: bool,         // 默认 true
    pub sort_y_offset: f32,    // 默认 0；加到 Transform2D.pos.y 上作为 sort_y
}
```

`sprite_render_system`（注册在 `Stage::Render`）：对所有 `(Transform2D, Sprite)` 实体，取 `pos = transform.lerp(time.alpha)`，查 atlas 帧，按锚点与 scale 计算 `dst`，`sort_y = pos.y + sort_y_offset`，提交 `DrawKind::Sprite`。`visible == false` 跳过。

### 6.5 Animation

```rust
/// 动画片段，RON 描述。持续时间以 tick 计，保证确定性（UC-22 / UC-23）。
pub struct AnimationClip {
    pub name: String,
    pub frames: Vec<AnimationFrame>,
    pub looping: bool,
}
pub struct AnimationFrame {
    pub atlas_frame: usize,         // 加载时由 frame 名解析为索引
    pub duration_ticks: u32,        // ≥ 1
    pub event: Option<String>,      // 进入该帧时触发（UC-23）
}

#[derive(Clone, Debug)]
pub struct Animator {
    pub clip: Handle<AnimationClip>,
    pub frame_index: usize,
    pub ticks_in_frame: u32,
    pub speed_num: u32,     // 速度 = speed_num / speed_den，默认 1/1；冰冻时 1/2（UC-22）
    pub speed_den: u32,
    pub playing: bool,
    pub finished: bool,     // 非循环片段播完置 true
}

impl Animator {
    pub fn new(clip: Handle<AnimationClip>) -> Self;
    /// 切换片段并从第 0 帧开始；若 clip 相同则不重置（避免攻击动画被重复打断）。
    pub fn play(&mut self, clip: Handle<AnimationClip>);
    pub fn restart(&mut self);
    pub fn set_speed(&mut self, num: u32, den: u32);
}

/// 动画事件，由 animation_system 发送。
#[derive(Clone, Debug)]
pub struct AnimationEvent { pub entity: Entity, pub event: String }
```

`animation_system`（注册在 `Stage::FixedUpdate`，由 `GfxPlugin` 添加，应当是 FixedUpdate 中首个系统以便同 tick 的游戏系统读到事件）：对所有 `(Animator, Sprite)`，按整数分数速度推进 `ticks_in_frame`（累加 `speed_num`，每达到 `speed_den` 算一个有效 tick），超过当前帧时长则进入下一帧，写入 `sprite.frame`，若新帧有 `event` 则发送 `AnimationEvent`。非循环播完停在最后一帧并置 `finished`。

整数分数速度而非 `f32` 是确定性要求：禁止用浮点累加决定帧切换。

### 6.6 GfxPlugin

```rust
pub struct GfxPlugin { pub logical_size: Vec2 }
```

`build`：插入 `Viewport`、`DrawList`、`Assets<Atlas>`、`Assets<AnimationClip>`；注册 `Events<AnimationEvent>`；添加 `animation_system`（FixedUpdate）、`clear_draw_list`（Render 首个）、`sprite_render_system`（Render）、`sort_draw_list`（Render 末尾，实际由 App 在 Render 阶段结束后调用）。

提供加载辅助：

```rust
pub fn load_atlas(server: &AssetServer, images: &mut Assets<Image>, atlases: &mut Assets<Atlas>, rel_ron: &str) -> Result<Handle<Atlas>, AssetError>;
pub fn load_clip(server: &AssetServer, atlases: &Assets<Atlas>, clips: &mut Assets<AnimationClip>, atlas: Handle<Atlas>, rel_ron: &str) -> Result<Handle<AnimationClip>, AssetError>;
```

加载时校验：帧矩形不超出图像范围、动画引用的帧名存在、`duration_ticks ≥ 1`、非空帧列表；失败返回 `AssetError::Validation`（UC-32）。

### 6.7 测试要求

| 测试 | 断言 |
|---|---|
| `viewport_letterbox_wide` | 窗口 1920 × 720 时 `scale == 1.0`，`offset_px == (320, 0)` |
| `viewport_letterbox_tall` | 窗口 1280 × 1440 时 `scale == 1.0`，`offset_px == (0, 360)` |
| `viewport_roundtrip` | `px_to_logical(logical_to_px(p)) ≈ p` |
| `drawlist_sort_order` | 混合 layer / sort_y / seq 提交后 `sorted()` 顺序符合 (layer, sort_y, seq) |
| `animator_advances_by_ticks` | 帧时长 [3, 2] 的片段在第 3 tick 切到帧 1，第 5 tick 回到帧 0（循环） |
| `animator_half_speed` | speed 1/2 时上例在第 6 tick 才切到帧 1 |
| `animator_event_fires_once_per_entry` | 带 event 的帧进入时恰好发送 1 个 `AnimationEvent`，停留期间不重复 |
| `animator_non_loop_finishes` | 非循环片段播完 `finished == true` 且停在末帧 |
| `sprite_render_uses_anchor_and_lerp` | anchor (0.5, 1.0)、alpha 0.5 时 `dst` 中心 x 等于 prev 与 pos 的中点 |
| `atlas_validation_rejects_out_of_bounds` | 帧矩形越界时返回 `Validation` 错误 |

---

## 7. trellia-audio

职责：音频请求队列与音频后端 trait。模拟层只发"意图"，不触碰设备（UC-50 ～ UC-53）。

```rust
pub type AudioClip = AudioBytes;   // re-export 自 trellia-assets

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioBus { Sfx, Music }

#[derive(Clone, Debug)]
pub enum AudioRequest {
    PlaySfx { clip: Handle<AudioClip>, volume: f32 },
    PlayMusic { clip: Handle<AudioClip>, looped: bool },
    StopMusic,
    SetBusVolume { bus: AudioBus, volume: f32 },
}

/// Resource。游戏层通过它发请求；后端每帧 drain。
pub struct AudioQueue { requests: Vec<AudioRequest> }
impl AudioQueue {
    pub fn play_sfx(&mut self, clip: Handle<AudioClip>);
    pub fn play_sfx_with_volume(&mut self, clip: Handle<AudioClip>, volume: f32);
    pub fn play_music(&mut self, clip: Handle<AudioClip>, looped: bool);
    pub fn stop_music(&mut self);
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32);
    pub fn drain(&mut self) -> std::vec::Drain<'_, AudioRequest>;
    pub fn peek(&self) -> &[AudioRequest];   // headless 测试断言用（UC-53）
}

/// 由后端实现。
pub trait AudioBackend {
    fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>);
}

pub struct AudioPlugin;   // build：插入 AudioQueue、Assets<AudioClip>
```

规则：同一 tick 内多次 `play_sfx` 同一 clip 允许重叠播放（UC-50），由后端决定是否限制并发数（建议上限 8 路相同 clip）。`AudioQueue` 在后端 drain 后为空；若后端未 drain（headless 不做断言的测试），`App` 在 `Stage::First` 清空并 `log::warn!` 一次。

---

## 8. trellia-scene

职责：场景栈、场景生命周期、覆盖层语义、帧末切换（UC-60 ～ UC-62）。

```rust
pub trait Scene: 'static {
    fn name(&self) -> &'static str;
    fn on_enter(&mut self, world: &mut World) {}
    fn on_exit(&mut self, world: &mut World) {}
    /// 上面被 push 了新场景时调用；默认什么也不做。
    fn on_pause(&mut self, world: &mut World) {}
    fn on_resume(&mut self, world: &mut World) {}
    fn fixed_update(&mut self, world: &mut World) {}
    fn update(&mut self, world: &mut World) {}
    /// 向 DrawList 提交绘制（世界内容）。
    fn draw(&mut self, world: &mut World) {}
    /// egui UI。headless 后端也会调用（用 egui::Context::run 驱动，不渲染）。
    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {}
    /// 覆盖层语义（UC-61）
    fn blocks_update_below(&self) -> bool { true }   // true：下方场景不执行 fixed_update / update
    fn blocks_draw_below(&self) -> bool { false }     // true：下方场景不执行 draw / ui
}

pub enum SceneCommand {
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),   // Pop 当前 + Push 新场景
    Clear,                     // 弹出所有
}

/// Resource。任意系统可发出命令；PostUpdate 末尾由 ScenePlugin 统一执行。
pub struct SceneCommands(Vec<SceneCommand>);
impl SceneCommands {
    pub fn push(&mut self, s: impl Scene);
    pub fn pop(&mut self);
    pub fn replace(&mut self, s: impl Scene);
    pub fn clear(&mut self);
}

/// Resource。只读查询。
pub struct SceneStack { /* Vec<Box<dyn Scene>> */ }
impl SceneStack {
    pub fn depth(&self) -> usize;
    pub fn top_name(&self) -> Option<&'static str>;
    pub fn names(&self) -> Vec<&'static str>;
}

pub struct ScenePlugin { pub initial: Box<dyn Scene> }
```

执行规则：

- `fixed_update` / `update`：从栈顶向下遍历，调用每个场景直到遇到 `blocks_update_below() == true` 的场景（含该场景）。
- `draw` / `ui`：从栈底向上遍历，但跳过"被某个 `blocks_draw_below() == true` 的上层场景覆盖"的场景。`ui` 调用顺序与 `draw` 一致，后调用者在 egui 中位于上层。
- 场景切换命令在 `PostUpdate` 末尾按入队顺序执行；`Push` 时调用下方场景 `on_pause` 与新场景 `on_enter`；`Pop` 时调用 `on_exit` 与下方场景 `on_resume`；栈为空时 `App` 置 `exit_requested`。
- 场景切换期间禁止再发出新命令（会被放入下一帧处理）。

借用说明：`ScenePlugin` 在调用场景方法前会将 `SceneStack` 从 `World.resources` 中临时取出，调用完毕放回，因此场景方法内可以自由借用 `world`，但不能访问 `SceneStack` 本身（只能访问 `SceneCommands`）。

### 8.1 测试要求

| 测试 | 断言 |
|---|---|
| `push_pauses_below` | Push 后下方场景 `on_pause` 被调用且不再收到 `fixed_update` |
| `overlay_still_draws_below` | 默认覆盖层下方场景仍收到 `draw` |
| `blocks_draw_hides_below` | `blocks_draw_below == true` 时下方不收到 `draw` |
| `commands_applied_at_frame_end` | 在 `fixed_update` 中 push，同帧 `update` 时 `depth` 未变，下一帧已变 |
| `empty_stack_requests_exit` | Pop 到空栈后 `FrameOutput.exit_requested == true` |

---

## 9. trellia-devtools

职责：开发期 egui 调试面板与调试绘制（UC-70 ～ UC-72）。发布构建可整体不链接（`pvz-app` 以 feature 控制）。

```rust
/// Resource。游戏层在任意阶段 push 调试形状，Render 阶段被绘制到 Layer::DEBUG 后清空。
pub struct DebugDraw { /* Vec<DrawKind> */ }
impl DebugDraw {
    pub fn rect(&mut self, rect: Rect, color: Color);
    pub fn line_h(&mut self, y: f32, x0: f32, x1: f32, color: Color);  // 以细矩形实现
    pub fn text(&mut self, pos: Vec2, text: impl Into<String>);
    pub fn enabled(&self) -> bool;   // 面板中可开关；关闭时 push 为空操作
}

pub struct DevtoolsPlugin { pub toggle_key: Key /* 默认 F1 */ }
```

面板内容（egui 窗口，F1 开关）：FPS 与帧耗时（最近 60 帧均值）、`fixed_tick`、本帧 tick 数、实体数、DrawList 命令数、`time_scale` 滑杆 `[0, 4]`、暂停按钮、单步按钮（设 `step_once`）、`DebugDraw` 开关、场景栈名称列表。

`DevtoolsPlugin` 自身作为一个永远在栈底之下的"伪场景"实现（实际为 `Stage::Render` 系统 + 一个注册到后端的 UI 回调），不占用 `SceneStack`。


---

## 10. 数据格式（RON）

所有数据文件使用 RON，由 `serde` 派生结构反序列化。字段名即规范，禁止别名。

### 10.1 图集 `*.atlas.ron`

```ron
Atlas(
    image: "demo/walker.png",          // 相对资产根目录
    frames: [
        (name: "walk_0", x: 0,  y: 0, w: 64, h: 96, anchor: (0.5, 1.0)),
        (name: "walk_1", x: 64, y: 0, w: 64, h: 96, anchor: (0.5, 1.0)),
        (name: "bite_0", x: 0,  y: 96, w: 64, h: 96, anchor: (0.5, 1.0)),
    ],
)
```

对应反序列化结构 `AtlasDef { image: String, frames: Vec<AtlasFrameDef> }`，`AtlasFrameDef { name, x, y, w, h: u32, anchor: (f32, f32) }`。`anchor` 缺省为 `(0.5, 1.0)`。

### 10.2 动画片段 `*.anim.ron`

```ron
AnimationClip(
    name: "walker_bite",
    looping: true,
    frames: [
        (frame: "bite_0", ticks: 20),
        (frame: "bite_1", ticks: 10, event: "bite_hit"),
        (frame: "bite_0", ticks: 20),
    ],
)
```

对应 `AnimationClipDef { name: String, looping: bool, frames: Vec<AnimationFrameDef> }`，`AnimationFrameDef { frame: String, ticks: u32, event: Option<String> }`。`frame` 在加载时按所属 atlas 的 `by_name` 解析为索引。

### 10.3 占位渲染约定

引擎不规定游戏层的数据格式（植物 / 僵尸 / 关卡定义属于游戏 spec）。但为支持 UC-27，示例与游戏层应当遵守：每个可渲染实体要么有 `Sprite`，要么有一个游戏层定义的 `Placeholder { size: Vec2, color: Color, label: &'static str }` 组件，由游戏层的 `placeholder_render_system` 转为 `Rect + Text` 命令。引擎 `examples/lane_demo` 必须演示此模式。

---

## 11. 后端

### 11.1 后端契约

后端是驱动 `App` 的宿主，职责：创建窗口（或不创建）、采集输入、调用 `App::frame`、消费 `DrawList` 与 `AudioQueue`、驱动 egui。后端通过门面 crate 暴露统一入口：

```rust
// trellia 门面 crate
pub struct RunConfig {
    pub title: String,
    pub logical_size: Vec2,        // 传给 GfxPlugin
    pub initial_window_px: (u32, u32),
    pub vsync: bool,
    pub asset_root: Option<PathBuf>,
}

pub fn run_egui(app: App, cfg: RunConfig) -> Result<(), anyhow::Error>;      // feature = "egui"（默认）
pub fn run_headless(app: App) -> HeadlessRunner;                               // 始终可用
```

### 11.2 trellia-egui

- 使用 `eframe::run_native`，在 `eframe::App::update` 中：把 egui 原始输入翻译为 `FrameInput`（键鼠映射表见 `src/input_map.rs`，仅覆盖第 4.9 节 `Key` 枚举），调用 `App::frame`，用 `egui::Painter`（`CentralPanel` 全屏、无边距）绘制 `DrawList`，调用场景 `ui`，drain `AudioQueue` 交给 `RodioBackend`，最后 `ctx.request_repaint()` 保证持续重绘。
- 纹理：维护 `HashMap<Handle<Image>, egui::TextureHandle>`，首次遇到某 `Handle<Image>` 时从 `Assets<Image>` 上传（`TextureOptions::NEAREST`，保证像素风清晰）。
- 坐标：`DrawList` 中的逻辑坐标经 `Viewport::logical_to_px` 转为窗口像素；letterbox 区域填充黑色。
- 文字：使用 egui 默认字体；若 `RunConfig` 未来扩展字体，需 ADR。v1 示例只用 ASCII 与少量中文（通过在后端 `setup_fonts` 加载仓库内一个开源 CJK 字体文件，字体许可证须允许再分发并随仓库附带 LICENSE）。
- 音频：`RodioBackend` 持有 `OutputStream`，`PlaySfx` 用 `Sink` 播一次并在结束后自动丢弃，`PlayMusic` 用单一常驻 `Sink` 替换，`SetBusVolume` 影响对应 Sink 的 `set_volume`。音频设备初始化失败时记录 `log::error!` 并降级为静默后端，不得阻止游戏启动。
- 退出：`FrameOutput.exit_requested` 为 true 或窗口关闭时结束。

### 11.3 trellia-headless

```rust
pub struct HeadlessRunner { app: App, egui_ctx: egui::Context, /* … */ }

impl HeadlessRunner {
    pub fn app(&self) -> &App;
    pub fn app_mut(&mut self) -> &mut App;
    /// 以固定 real_dt = 1/60 推进一帧（因此恰好一个 tick）。
    pub fn frame(&mut self) -> &FrameOutput;
    pub fn frames(&mut self, n: u32);
    pub fn frame_with(&mut self, input: FrameInput) -> &FrameOutput;
    /// 便捷输入注入（逻辑坐标）。
    pub fn click(&mut self, logical_pos: Vec2);
    pub fn press_key(&mut self, key: Key);
    /// 上一帧排序后的 DrawList 快照。
    pub fn last_draw_list(&self) -> &[DrawCmd];
    /// 自上次调用以来所有音频请求（不被清空的累积记录）。
    pub fn take_audio_log(&mut self) -> Vec<AudioRequest>;
    /// 对 World 做可比较的快照：按 Entity id 排序后，对实现了 Snapshot trait 的组件序列化为字节。
    pub fn snapshot(&self) -> Vec<u8>;
}

/// 游戏层为参与确定性快照的组件实现（通常用 serde 序列化）。
pub trait Snapshot { fn write(&self, out: &mut Vec<u8>); }
```

`snapshot` 的具体机制：`HeadlessRunner::register_snapshot::<T: Component + Snapshot>()` 登记组件类型，`snapshot()` 时依次按类型、按 `Entity` 升序写出。`Transform2D` 默认已登记。

---

## 12. 确定性规则（强制）

以下规则适用于一切注册在 `Stage::FixedUpdate` 的系统与 `Scene::fixed_update`，违反任意一条即为缺陷：

1. 禁止读取 `Time::real_dt`、系统时钟、帧率。
2. 一切随机来自 `GameRng`。
3. 禁止依赖 `HashMap` / `HashSet` 的迭代顺序影响结果；需要有序时使用 `BTreeMap`、`Vec` + 排序、或 hecs 查询（hecs 查询顺序在相同操作序列下稳定）。
4. 时长、冷却、速度累计用整数 tick 或固定点整数；位置可用 `f32`，但同一运算必须以同一顺序执行（禁止依赖并行或 `par_iter`）。
5. 禁止在 FixedUpdate 中读取 `Tween`、`Animator.frame_index` 以外的"视觉"状态做逻辑判断（`Animator` 以 tick 推进，属于确定性状态，允许）。
6. 任何浮点比较使用显式阈值，禁止 `==`。
7. 实体创建顺序必须确定（例如同一 tick 内多个 spawn 来源的先后由系统注册顺序决定，而非 HashMap）。

验证方式：`determinism_smoke`（core）与 `lane_demo_determinism`（第 13 章）两项测试在 CI 中必须通过。

---

## 13. 里程碑与验收

每个里程碑的验收条件全部满足后方可进入下一个。每个里程碑提交时必须附带：对应 crate 的 `README.md`、本阶段的 ADR（若有）、CI 绿色。

### M0 工程骨架

交付：workspace、全部 crate 空壳（`lib.rs` 含模块文档）、`rust-toolchain.toml`、CI 配置、`scripts/check_deps.sh`、`trellia` 门面 crate 的 `prelude` 模块、`docs/decisions/ADR-001-hecs-types-in-public-api.md`。

验收：`cargo build --workspace` 通过；`check_deps.sh` 对当前空壳通过；故意在 `trellia-core` 加入 `egui` 依赖时 `check_deps.sh` 失败。

### M1 核心

交付：`trellia-core` 全部功能（第 4 章）；`trellia-headless` 的 `HeadlessRunner::frame / frames / snapshot`（不含 egui ctx 与 DrawList 部分）。

验收：第 4.11 节全部测试通过；`examples/` 下无需窗口的 `core_counter` 示例在 headless 中运行 600 tick 后打印确定的计数结果。

### M2 资产、绘制、动画（headless）

交付：`trellia-assets`、`trellia-gfx` 全部功能；`assets/demo/` 下原创占位素材：`placeholder_8x8.png`、`walker.png`（64 × 96 像素、4 帧的几何人形占位图，自绘）、`walker.atlas.ron`、`walker_walk.anim.ron`、`walker_bite.anim.ron`；`HeadlessRunner::last_draw_list`。

验收：第 5.3、6.7 节全部测试通过；headless 集成测试 `sprite_appears_in_drawlist`：spawn 一个 `(Transform2D, Sprite, Animator)`，运行 30 tick，`last_draw_list` 中恰有 1 条 `Sprite` 命令且其 `src_px` 对应 `walk_1` 帧。

### M3 egui 后端

交付：`trellia-egui`（渲染 + 输入，不含音频）；`run_egui`；`examples/hello_window`（显示 "Trellia" 文字与一个矩形）；`examples/sprites`（10 个 walker 占位精灵以不同速度播放走路动画，窗口可任意缩放并保持 letterbox，F 键翻转，鼠标点击处画一个圆持续 1 秒）。

验收：三平台（至少 Linux + 另一平台）运行 `sprites` 示例，手动核对：缩放正确、动画流畅、点击位置准确（letterbox 偏移下仍准确）、`cargo run --release` 下 1000 个精灵帧耗时 < 4 ms（示例提供 `--count 1000` 参数并在标题栏显示帧耗时）。

### M4 音频、场景、调试

交付：`trellia-audio`、`RodioBackend`、`trellia-scene`、`trellia-devtools`；`assets/demo/` 下 CC0 或自制音效 `blip.ogg`、`hit.ogg` 与一段 10 秒自制循环 BGM `loop.ogg`（附来源与许可证说明文件）。

验收：第 8.1 节测试通过；`sprites` 示例扩展为：Esc 推入一个半透明暂停场景（下方画面冻结但可见），再按 Esc 弹出；F1 打开调试面板，时间缩放滑杆与单步按钮生效；点击播放 `blip.ogg`；headless 测试 `audio_request_recorded`：模拟点击后 `take_audio_log` 含 1 条 `PlaySfx`。

### M5 垂直切片 `lane_demo`

这是引擎 v1 的最终验收，也是游戏 spec 的起点。它是一个只有一条车道的极简塔防，所有实体使用占位渲染（几何图形 + 文字），不含任何第三方版权元素。

规则（固定，便于测试）：

| 项 | 值 |
|---|---|
| 逻辑分辨率 | 1280 × 720 |
| 车道 | 1 条，y 中心 400，格子 9 个，每格宽 100，起点 x = 100 |
| 炮塔（绿色方块 "T"） | 点击空格放置，花费 50 资源；每 90 tick 若同车道前方有敌人则发射弹丸 |
| 弹丸（黄色小圆） | 速度 6 逻辑像素 / tick，命中伤害 1，到达 x > 1280 销毁 |
| 行者（紫色方块 "W"） | 从 x = 1300 生成，速度 0.6 像素 / tick，生命 5；碰到炮塔格子停下，播放 `walker_bite` 动画，`bite_hit` 事件帧对炮塔造成 1 伤害，炮塔生命 3 |
| 生成 | 第 120 tick 生成第 1 个，之后每 600 tick 一个，共 5 个；生成车道固定（单车道），生成时间加 `GameRng.gen_range(0..30)` tick 抖动 |
| 资源 | 初始 150，每 300 tick +25，顶部文字显示 |
| 失败 | 任一行者 x < 60 → 推入 GameOver 覆盖场景（半透明、文字、"Restart" 按钮） |
| 胜利 | 5 个行者全部死亡 → 推入 Victory 覆盖场景 |
| 音效 | 发射 `blip`、命中 `hit`、BGM 循环 |
| 调试 | F1 面板；`DebugDraw` 开启时绘制格子边界与行者碰撞区间 |

验收：

1. 手动：egui 后端下完整玩一局可胜可负，暂停 / 重开正常，窗口缩放正常。
2. `lane_demo_determinism`（headless）：种子 42，脚本在第 1 tick 点击格子 0、第 400 tick 点击格子 1，运行 4000 tick，两次运行 `snapshot()` 逐字节相同，且与仓库中提交的 `tests/golden/lane_demo_seed42.snap` 相同（golden 文件在首次通过人工核对后提交；之后任何引擎改动导致 golden 变化必须在 PR 中说明）。
3. `lane_demo_win_with_two_turrets`（headless）：上述脚本在 4000 tick 内进入 Victory 场景（`SceneStack::top_name() == "Victory"`）。
4. `lane_demo_lose_without_turrets`（headless）：不放置任何炮塔，在 2500 tick 内进入 GameOver 场景。
5. `lane_demo_bite_uses_event_frame`（headless）：炮塔生命在行者到达后，恰好在 `bite_hit` 帧对应 tick 减少，而非到达当 tick。
6. `lane_demo_audio`（headless）：一局中 `take_audio_log` 含 `PlayMusic` 1 条、`PlaySfx(blip)` ≥ 10 条。
7. 性能：release 下 `lane_demo` 帧耗时 < 2 ms（5 行者 + 2 炮塔 + 弹丸规模）。

M5 通过即视为引擎 v1 完成，进入游戏 spec 编写。

---

## 14. 测试策略汇总

| 层次 | 位置 | 工具 | 覆盖 |
|---|---|---|---|
| 单元 | 各 crate `#[cfg(test)]` | `cargo test` | 第 4 ～ 8 章列出的全部表格测试 |
| 集成 | `crates/trellia-headless/tests/` | `HeadlessRunner` | DrawList、音频日志、场景栈、确定性快照 |
| 示例 | `examples/*` | 手动 + CI 仅编译 | 后端视觉与交互 |
| 黄金 | `tests/golden/*.snap` | 字节比对 | 确定性回归 |
| 依赖 | `scripts/check_deps.sh` | `cargo metadata` | 第 3.2 节 R1 ～ R5 |
| 性能 | 示例 `--count` 参数 | 手动 | M3 / M5 帧耗时预算 |

禁止在测试中 `sleep` 或依赖真实时间；一切时间推进通过 `HeadlessRunner::frame_with` 指定 `real_dt`。

---

## 15. 工程规范

- Rust edition：使用当前 stable 支持的最新 edition；`rust-toolchain.toml` 锁定 `stable`。
- 版本：workspace 统一 `version = "0.1.0"`，所有 trellia crate 同步版本号。
- 依赖：全部在 `[workspace.dependencies]` 声明，成员 crate 以 `workspace = true` 引用。
- Feature：`trellia` 门面 crate 提供 `egui`（默认开）、`devtools`（默认开）两个 feature；`trellia-headless` 无 feature。
- 文档：每个公共类型与函数必须有 `///` 文档；`cargo doc` 零警告。`lib.rs` 顶部 `//!` 说明职责、依赖的用例编号、不负责的事项。
- 提交：Conventional Commits（`feat(core): …`、`test(gfx): …`、`docs(adr): …`）。
- ADR 模板：`docs/decisions/ADR-000-template.md`，字段：编号、标题、日期、状态、背景、决策、后果、替代方案。
- 禁止：`unwrap()` 出现在非测试代码的资产加载路径；`unsafe`；`static mut`；全局可变状态（一切状态在 `World` 中）。
- Clippy：`-D warnings`，额外启用 `clippy::pedantic` 中的 `must_use_candidate`、`missing_errors_doc`（在 workspace `Cargo.toml` 的 `[workspace.lints]` 中配置）。

---

## 16. 术语表

| 术语 | 含义 |
|---|---|
| tick | 一次固定步长模拟，1/60 秒逻辑时间 |
| 帧（frame） | 一次后端渲染循环，可包含 0 ～ 5 个 tick |
| 逻辑坐标 | 1280 × 720 设计分辨率下的像素坐标 |
| letterbox | 等比缩放后用黑边填充的视口策略 |
| Plugin | 向 `App` 注册资源、事件、系统的模块单元 |
| Stage | 一帧内系统执行的阶段 |
| Resource | `World` 中以类型为键的全局单例 |
| Handle | 指向 `Assets<T>` 中某项的轻量句柄 |
| DrawList | 一帧内排序后的绘制命令列表，后端无关 |
| 占位渲染 | 以几何图形 + 文字代替美术素材的渲染模式 |
| 垂直切片 | 贯穿全部模块的最小可玩示例（`lane_demo`） |
| golden | 人工核对后提交的确定性快照基准文件 |

---

## 附录 A：实现顺序建议（写给 AI 开发者）

1. 严格按 M0 → M5 顺序，不跨里程碑实现。
2. 每个模块先写第 4 ～ 8 章表格中的测试（红），再实现（绿），再写 README。
3. 遇到 spec 未定义的 API 细节，选择最小实现并写 ADR；禁止"顺手"加功能。
4. 每完成一个 crate，在 `docs/crates/<name>.md` 写 50 行以内的使用说明，包含一个最小代码片段。
5. M5 完成后，输出 `docs/engine-v1-report.md`：各 crate 行数、测试数、已知限制、建议在游戏开发期补充的功能清单（仅记录，不实现）。
