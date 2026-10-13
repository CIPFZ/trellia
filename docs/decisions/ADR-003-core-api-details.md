# ADR-003：trellia-core API 细节补充

- 编号：ADR-003
- 标题：trellia-core 中 SPEC.md 未写清的 API 细节
- 日期：2026-10-13
- 状态：已接受

## 背景

实现 SPEC.md 第 4 章时遇到以下未定义 / 无法字面实现之处：

1. 4.1 `pub type System = fn(&mut World);` 与 `add_system(..., system: impl System)` 不兼容：
   类型别名不是 trait，不能写 `impl System`。
2. 4.9 “坐标为物理窗口像素，由 App 结合 Viewport 转为逻辑坐标”，但 `Viewport` 在 `trellia-gfx`，
   而 R1 禁止 core 依赖 gfx；6.1 又要求 `Viewport.window_size_px` “每帧由 App 从 FrameInput 更新”。
3. 8 “栈为空时 App 置 `exit_requested`”：`trellia-scene` 需要一种向 App 请求退出的方式，
   而 `FrameOutput` 由 App 私有持有。
4. R4 规定游戏模拟层只依赖 `trellia-core`，但公共 API 使用了 `glam::Vec2` 与 hecs 查询类型。
5. 3.4 固定步长累加器：`real_dt = 1/30` 时 f32 运算 `1/30 - 1/60 < 1/60`，
   字面实现只会执行 1 个 tick，`fixed_step_accumulates` 无法通过。

## 决策

1. `add_system(&mut self, stage: Stage, system: System)`，参数为函数指针；普通 `fn` 项自动转换。
2. `Input` 增加两个公开字段 `mouse_pos_px: Option<Vec2>`、`window_size_px: Vec2`，存放本帧
   `FrameInput` 的原始物理像素值。App 在 `Stage::First` 开头写入，并先令 `mouse_pos = mouse_pos_px`
   （无视口时等同逻辑坐标）；`GfxPlugin` 在 `Stage::First` 注册系统，据此更新 `Viewport`
   并把 `Input.mouse_pos` 改写为逻辑坐标。
3. 新增资源 `AppExit { pub requested: bool }`，`App::new` 插入。任意系统 / 插件置 `requested = true`
   请求退出；`InputEvent::CloseRequested` 也会置位。App 在帧末把它写入 `FrameOutput.exit_requested`。
   一旦置位保持为 true。
4. `trellia-core` 重新导出 `pub use glam;`、`pub use glam::Vec2;`、`pub use hecs;`（见 ADR-001）。
5. 累加器比较使用显式阈值 `accumulator + 1e-5 >= fixed_dt`（SPEC.md 12.6 “浮点比较用显式阈值”），
   tick 后 `accumulator` 下限钳到 0。触发 5 tick 上限后若仍有余量，`accumulator` 清零（“超出部分丢弃”）。
   `step_once == true` 时本帧恰好执行 1 个 tick（无论是否暂停），不改动累加器。
6. `App::step_ticks(n)`：首次调用时先执行 `Startup`；之后只执行 n 次“tick 前同步 prev_pos →
   `fixed_tick += 1` → FixedUpdate 系统 → 应用命令”，不翻页事件、不处理输入、无视 `paused`。

## 后果

- 游戏层只需依赖 `trellia-core` 即可使用 `Vec2` 与 hecs 查询语法。
- 未添加 `GfxPlugin` 时 `Input.mouse_pos` 为物理像素坐标（core 单独使用时的合理退化）。

## 替代方案

- （2）在 core 新增 `Window` 资源存放原始窗口信息：多一个公共类型，否决。
- （3）以 `Events<AppExit>` 表示退出：事件两帧后消失，`exit_requested` 不是粘滞状态，否决。
- （5）累加器改用 f64：只是把误差推后，仍需阈值，否决。
