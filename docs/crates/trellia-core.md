# trellia-core 使用说明

`trellia-core` 是游戏模拟层（如 `pvz-core`）唯一允许依赖的引擎 crate（R1 / R4）。

## 组装

```rust
use trellia_core::{App, Plugin, Stage, World, GameRng};

struct Level { seed: u64 }
impl Plugin for Level {
    fn name(&self) -> &'static str { "Level" }
    fn build(&self, app: &mut App) {
        app.insert_resource(GameRng::from_seed(self.seed))
            .add_system(Stage::FixedUpdate, rules);
    }
}
fn rules(_world: &mut World) {}
```

## 借用模式

`World` 字段公开：`entities`（hecs）、`resources`、`events`、`commands`。
系统内可同时借用不同字段，例如一边 `query_mut` 一边 `commands.despawn`。

## 确定性清单（SPEC.md 12）

- 计时用 `Timer`（tick），速度累计用整数；禁止 FixedUpdate 中读 `Time::real_dt`。
- 随机只来自 `GameRng`；禁止依赖 HashMap 迭代顺序；浮点比较用阈值。
- 遍历中创建 / 销毁用 `world.commands`，tick 末按记录顺序应用。
- 高频状态（冰冻、减速）放常驻组件，不要频繁增删组件（4.3.4）。

## 时间控制

`Time.paused` 暂停模拟；`Time.time_scale` ∈ [0, 4]；`Time.step_once` 单步一个 tick。
`Time.alpha` 为渲染插值因子，配合 `Transform2D::lerp` 使用。

## 输入

`Input` 每帧快照一次；`just_pressed` 只在一帧内为真，同帧多个 tick 共享。
`mouse_pos` 在添加 `GfxPlugin` 后为逻辑坐标（ADR-003）。
