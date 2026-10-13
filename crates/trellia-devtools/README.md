# trellia-devtools

开发期 egui 调试面板（F1）与调试形状 `DebugDraw`（UC-70 ～ UC-72）。发布构建可通过门面
feature `devtools` 整体去掉。

## 最小示例

```rust
use trellia_core::{App, Vec2, World};
use trellia_devtools::{DebugDraw, DevtoolsPlugin};
use trellia_gfx::{Color, GfxPlugin, Rect};

fn hitboxes(world: &mut World) {
    let dbg = world.resources.expect_mut::<DebugDraw>(); // 关闭时为空操作
    dbg.rect(Rect::from_min_size(Vec2::new(100.0, 350.0), Vec2::new(100.0, 100.0)), Color::WHITE);
}

let mut app = App::new();
app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))   // 先于 DevtoolsPlugin
    .add_plugin(DevtoolsPlugin::default())                 // toggle_key = F1
    .add_system(trellia_core::Stage::Update, hitboxes);
```

## 面板内容

FPS 与帧耗时（近 60 帧 `real_dt` 均值）、`fixed_tick`、本帧 tick 数、实体数、DrawList 命令数、
时间缩放滑杆 `[0, 4]`、暂停 / 继续、单步、DebugDraw 开关、场景栈。
