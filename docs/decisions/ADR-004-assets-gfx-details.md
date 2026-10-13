# ADR-004：资产、绘制与动画的 API 细节补充

- 编号：ADR-004
- 标题：trellia-assets / trellia-gfx 中 SPEC.md 未写清的细节
- 日期：2026-10-13
- 状态：已接受

## 背景与决策

### 1. 启动期错误如何“返回到 App 启动阶段”（SPEC.md 4.10 / 5.2，UC-32）

`Plugin::build` 返回 `()`，`App::frame` 返回 `&FrameOutput`，二者都无法携带 `Result`。

决策：`App` 新增

```rust
pub fn record_startup_error(&mut self, err: impl std::fmt::Display);
pub fn startup_errors(&self) -> &[String];
```

插件在 `build` 中遇到可恢复错误（资产缺失、RON 解析失败）时调用 `record_startup_error`，
错误文本须含文件路径与原因（`AssetError` 的 `Display` 已满足）。后端在第一帧前检查：
`run_egui` 返回 `Err`；`HeadlessRunner::new` 以包含全部错误文本的信息 panic（headless 只用于测试，
构造即启动阶段）。

### 2. 资产根目录解析（SPEC.md 5.2）

`cargo run --example <x>` / `cargo test` 时运行期 `CARGO_MANIFEST_DIR` 指向成员包目录
（如 `examples/lane_demo`），而素材位于 workspace 根 `assets/`。

决策：顺序为 `TRELLIA_ASSETS` → 可执行文件同级 `assets/` → `CARGO_MANIFEST_DIR/assets/`
→ **`CARGO_MANIFEST_DIR` 各级祖先目录下的 `assets/`**。`TRELLIA_ASSETS` 已设置但目录不存在时
直接返回 `RootNotFound(该路径)`；全部候选都不存在时返回 `RootNotFound(最后一个候选)`。

### 3. RON 细节（SPEC.md 10）

- ron 会校验结构体名，`AtlasDef` / `AnimationClipDef` 以 `#[serde(rename = "Atlas")]` /
  `#[serde(rename = "AnimationClip")]` 匹配文件中的 `Atlas(...)` / `AnimationClip(...)`（字段名不变，无别名）。
- `AssetServer::load_ron` 启用 `IMPLICIT_SOME` 扩展，使 10.2 中 `event: "bite_hit"` 的写法合法
  （`Some("bite_hit")` 同样合法）。

### 4. walker.png 尺寸（SPEC.md 13 M2）

“64 × 96 像素、4 帧”按 10.1 示例理解为**每帧 64 × 96**，2 × 2 排布（`walk_0` / `walk_1` / `bite_0` / `bite_1`），
整图 128 × 192。

### 5. clippy 要求的配套方法

`Assets<T>` 与 `DrawList` 提供 `len` 时，`clippy::len_without_is_empty` 要求同时提供 `is_empty`；
`Assets<T>` 实现 `Default`。

另外，`ron::error::SpannedError` 为 104 字节，使 `AssetError` 触发 `clippy::result_large_err`。
决策：`AssetError::Ron` 的 `source` 字段为 `Box<ron::error::SpannedError>`（其余变体与 SPEC.md 5.2 一致），
不使用 `#[allow]` 压制。

### 6. `DrawList::sprite` 返回值

SPEC.md 6.3 写作 `-> &mut DrawCmdBuilder`，但列表中存的是 `DrawCmd`，安全 Rust 无法凭空返回
对另一类型的 `&mut`。决策：返回 `DrawCmdBuilder<'_>`（内部持有 `&mut DrawCmd`），提供
`tint(&mut self, Color) -> &mut Self` 与 `flip_x(&mut self, bool) -> &mut Self`。
调用写法 `list.sprite(..).tint(c).flip_x(true);` 与 SPEC 一致。

### 7. 非精灵命令的 `sort_y`

`rect` / `rect_stroke` / `circle` / `text` 没有 `sort_y` 参数。决策：其 `sort_y = 0.0`，
同层内按提交顺序绘制（占位渲染“先矩形后文字”自然成立）。

### 8. Animator 细节（SPEC.md 6.5）

- 整数分数速度的累加器为 `Animator` 的私有字段；`ticks_in_frame` 保持“有效 tick 数”语义。
- `speed_den == 0` 视为 1。
- `new` / `play`（换片段时）/ `restart` 后的首个处理 tick 视为“进入第 0 帧”：写入 `sprite.frame`，
  若第 0 帧有 `event` 则发送一次。循环回到第 0 帧同样视为进入。
- 一个 tick 内可推进多帧（高速播放），每进入一帧都检查事件。
- `playing == false` 或 `finished == true` 时系统跳过该实体。

### 9. 颜色空间（SPEC.md 6.2 “线性 0..1”）

`Color` 分量为线性光强度；`from_hex` 把 sRGB 十六进制转换为线性值；后端（egui）经 `egui::Rgba`
（线性）转换为显示用 sRGB。

### 10. Viewport

- `Viewport::new(logical_size)` 构造（窗口尺寸初始等于逻辑尺寸）。
- 窗口尺寸任一分量为 0（最小化）时保持上一帧的 `scale` / `offset_px` 不变。
- 黑边区域内的鼠标位置仍换算为逻辑坐标（可能超出 `logical_rect`），由游戏层自行判断。

### 11. Sprite 默认值

`Sprite::new(atlas)`：`frame = 0`、`layer = Layer::WORLD`、`tint = WHITE`、`flip_x = false`、
`scale = 1.0`、`visible = true`、`sort_y_offset = 0.0`。

### 12. HeadlessRunner 虚拟窗口

存在 `Viewport` 时，虚拟窗口尺寸等于 `Viewport.logical_size`（物理像素 = 逻辑坐标）。

## 后果

- `trellia-core` 增加两个公共方法；其余均为 gfx / assets 内部或 SPEC 已列类型的细化。

## 替代方案

- （1）为 `Plugin::build` 改返回 `Result`：改变 SPEC.md 4.1 的 trait 形状，否决。
- （2）示例中硬编码 `concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets")`：每个消费者都要重复，否决。
- （6）返回 `&mut DrawCmd`：调用方需要直接改 `kind` 枚举，易用性差，否决。
