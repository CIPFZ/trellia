# trellia-scene

场景栈、场景生命周期、覆盖层语义与帧末统一切换（UC-60 ～ UC-62），以及后端调用的
egui UI 回调表 `UiHooks`。

## 最小示例

```rust
use trellia_core::{App, World};
use trellia_scene::{Scene, SceneCommands, ScenePlugin};

struct Battle;
impl Scene for Battle {
    fn name(&self) -> &'static str { "Battle" }
    fn update(&mut self, world: &mut World) {
        // 任意时刻发出命令，PostUpdate 中统一执行
        world.resources.expect_mut::<SceneCommands>().push(Pause);
    }
}
struct Pause; // 默认 blocks_update_below = true，blocks_draw_below = false
impl Scene for Pause { fn name(&self) -> &'static str { "Pause" } }

let mut app = App::new();
app.add_plugin(ScenePlugin::new(Battle));
```

## 要点

- `fixed_update` / `update` 自顶向下，遇到 `blocks_update_below` 的场景为止；
  `draw` / `ui` 自底向上，跳过被 `blocks_draw_below` 场景遮住的部分。
- 场景方法执行期间 `SceneStack` 被临时取出；查询栈请在普通系统中读 `SceneStack`。
- 栈被清空后 App 请求退出。`Scene` 需满足 `Send + Sync`（ADR-006）。
