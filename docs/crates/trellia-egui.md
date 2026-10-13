# trellia-egui 使用说明

## 入口

通常经门面调用：`trellia::run_egui(app, trellia::RunConfig { .. })`。

组装顺序：`AssetsPlugin { root: cfg.asset_root.clone() }` → `GfxPlugin::new(cfg.logical_size)` →
游戏插件 → `run_egui`。`run_egui` 不会替你添加插件（ADR-005）。

## 每帧流程

1. 从 egui 输入构造 `FrameInput`（物理像素；过滤按键重复；指针在 egui 上时不转发按下）。
2. `App::frame`。
3. 把排序后的 `DrawList` 画到根 UI 的 Painter：先整屏填黑，再按视口裁剪绘制；
   连续同纹理精灵合并为一个 Mesh。
4. 更新 `WindowStatus.cpu_frame_ms`，标题变化时下发 `ViewportCommand::Title`。
5. `exit_requested` 时关闭窗口；`request_repaint` 保持持续重绘。

## 帧耗时

`WindowStatus.cpu_frame_ms` 取 eframe 的 `cpu_usage`（整帧 CPU 时间，不含 vsync 等待），
可显示在标题栏：

```rust
use trellia::egui_backend::WindowStatus;
fn title(world: &mut trellia::core::World) {
    if let Some(s) = world.resources.get_mut::<WindowStatus>() {
        s.title = format!("game | {:.2} ms", s.cpu_frame_ms);
    }
}
```

## 无 GPU 环境

使用 glow，可在 Xvfb + Mesa 软件渲染下运行：
`xvfb-run cargo run --example sprites`（Linux CI / 远程机器冒烟检查）。
