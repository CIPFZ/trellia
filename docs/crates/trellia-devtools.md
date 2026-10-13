# trellia-devtools 使用说明

## 接入

`app.add_plugin(DevtoolsPlugin::default())`，放在 `GfxPlugin` 之后。发布构建关闭门面
feature `devtools` 即可整体去掉；游戏层对 `DebugDraw` 的访问应写成可选：

```rust
fn debug(world: &mut trellia_core::World) {
    #[cfg(feature = "devtools")]
    if let Some(dbg) = world.resources.get_mut::<trellia_devtools::DebugDraw>() {
        dbg.text(trellia_core::Vec2::new(8.0, 700.0), "debug");
    }
}
```

## 面板（F1）

- 时间缩放滑杆写入 `Time.time_scale`；暂停写 `Time.paused`；单步写 `Time.step_once`。
- DebugDraw 复选框调用 `DebugDraw::set_enabled`。
- 面板以 UI 回调形式注册（`trellia_scene::UiHooks`），不占用场景栈；headless 下同样运行。

## DebugDraw

`rect`（1 像素描边）、`line_h`（1 像素高实心）、`text`（14 号白字），Render 阶段画到
`Layer::DEBUG` 后清空。默认关闭。
