# trellia-core

应用骨架、ECS 封装、时间、事件、资源、计时器、补间、随机数与输入状态；
唯一允许被游戏模拟层依赖的 crate（SPEC.md 第 4 章）。

## 最小示例

```rust
use trellia_core::{App, Plugin, Stage, Timer, Transform2D, Vec2, World};

struct Spawner(Timer);

fn spawn_system(world: &mut World) {
    if world.resources.expect_mut::<Spawner>().0.tick() {
        // 遍历中创建 / 销毁一律走 commands（UC-04），tick 末统一应用
        world.commands.spawn((Transform2D::new(Vec2::new(1300.0, 400.0)),));
    }
}

struct GamePlugin;
impl Plugin for GamePlugin {
    fn name(&self) -> &'static str { "GamePlugin" }
    fn build(&self, app: &mut App) {
        app.insert_resource(Spawner(Timer::repeating(600)))
            .add_system(Stage::FixedUpdate, spawn_system);
    }
}

let mut app = App::new();
app.add_plugin(GamePlugin);
app.step_ticks(600);
assert_eq!(app.world().entity_count(), 1);
```

## 要点

- 帧流程：`First → PreUpdate → FixedUpdate × (0..=5) → Update → PostUpdate → Render`。
- 系统签名统一为 `fn(&mut World)`；同一 Stage 按注册顺序执行。
- 时长用 `Timer`（整数 tick）；随机只用 `GameRng`；FixedUpdate 中禁止读 `real_dt`。
- `Events<T>` 需先 `world.events.register::<T>()`，事件可读两帧。
- 退出：置 `AppExit.requested = true`。
