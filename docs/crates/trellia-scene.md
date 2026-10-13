# trellia-scene 使用说明

## 生命周期

| 时机 | 回调 |
|---|---|
| Push | 下方 `on_pause` → 新场景 `on_enter` |
| Pop | 栈顶 `on_exit` → 下方 `on_resume` |
| Replace | 旧栈顶 `on_exit` → 新场景 `on_enter` |
| Clear | 自顶向下 `on_exit`，随后 App 请求退出 |

初始场景在 `Stage::Startup` 入栈；切换命令在 `ScenePlugin` 的 PostUpdate 系统中按入队顺序执行。

## 覆盖层

暂停菜单：`blocks_update_below = true`（默认，战斗不更新）、`blocks_draw_below = false`（默认，战斗仍绘制）。
需要“冻结动画”时在 `on_enter` 置 `Time.paused = true`，`on_exit` 复位。

## UI

`Scene::ui(world, ctx)` 使用 egui（`trellia_scene::egui` 已重新导出）。建议用 `egui::Area` /
`egui::Window` 而非 `CentralPanel`，以免遮住世界绘制。UI 交互结果在下一帧生效。

```rust
use trellia_scene::{egui, Scene, SceneCommands};
struct GameOver;
impl Scene for GameOver {
    fn name(&self) -> &'static str { "GameOver" }
    fn ui(&mut self, world: &mut trellia_core::World, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("over")).show(ctx, |ui| {
            if ui.button("Restart").clicked() {
                world.resources.expect_mut::<SceneCommands>().pop();
            }
        });
    }
}
```
