# trellia-gfx

视口与逻辑坐标、后端无关的 `DrawList`、精灵 / 图集与以 tick 推进的序列帧动画
（UC-20 ～ UC-27）。本 crate 不接触 GPU。

## 最小示例

```rust
use trellia_core::{App, Plugin, Stage, World, Vec2};
use trellia_gfx::{Color, DrawList, GfxPlugin, Layer, Rect};

fn draw_placeholder(world: &mut World) {
    let list = world.resources.expect_mut::<DrawList>();
    let body = Rect::from_center_size(Vec2::new(640.0, 400.0), Vec2::new(60.0, 80.0));
    list.rect(Layer::WORLD, body, Color::from_hex(0x8E44AD));
    list.text(Layer::WORLD, body.min, "W", 24.0, Color::WHITE);
}

let mut app = App::new();
app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))   // 先于其他 Render 系统
    .add_system(Stage::Render, draw_placeholder);
```

## 要点

- 排序键 `(layer, sort_y, seq)`；精灵 `sort_y = pos.y + sort_y_offset`，形状与文字为 0。
- `Sprite` + `Transform2D` 由内置系统按 `Time.alpha` 插值绘制；锚点归一化。
- `Animator` 以整数分数速度推进（`set_speed(1, 2)` 半速），进入带 `event` 的帧时发送 `AnimationEvent`。
- `load_atlas` / `load_clip` 在加载时校验越界、帧名、`ticks ≥ 1`。
- 添加 `GfxPlugin` 后 `Input.mouse_pos` 为逻辑坐标（letterbox 已扣除）。
