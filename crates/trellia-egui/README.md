# trellia-egui

基于 eframe（glow 渲染器）的窗口后端：输入翻译为 `FrameInput`、驱动 `App::frame`、
用 egui Painter 绘制 `DrawList`（等比缩放 + 黑边 letterbox）、首次绘制时惰性上传纹理（`NEAREST`）。

## 最小示例

```rust,no_run
use trellia_core::{App, Vec2};
use trellia_gfx::GfxPlugin;
use trellia_egui::{run, RunConfig};

fn main() -> anyhow::Result<()> {
    let cfg = RunConfig { title: "demo".into(), ..RunConfig::default() };
    let mut app = App::new();
    app.add_plugin(GfxPlugin::new(cfg.logical_size)); // 必须先添加，run 会校验
    run(app, cfg)
}
```

## 要点

- 启动期错误（`App::startup_errors`）、缺少 `GfxPlugin`、`logical_size` 不一致时 `run` 返回 `Err`。
- 指针在 egui 窗口上（场景 UI、调试面板）时，鼠标按下不转发给游戏；文本框聚焦时不转发键盘。
- 资源 `WindowStatus { title, cpu_frame_ms }`：改 `title` 即改窗口标题；`cpu_frame_ms` 为近 60 帧整帧 CPU 耗时均值。
- 日志：`run` 初始化 `env_logger`（默认 `info`，`RUST_LOG` 覆盖）。
- Linux 构建需要 ALSA 开发包（`libasound2-dev` / `alsa-lib-devel`）。
- `RunConfig.font_path`：追加一个 TTF / OTF 作为回退字体（如 CJK 子集），DrawList 文字与 egui 控件共用（ADR-010）。
