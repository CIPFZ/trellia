# trellia

门面 crate：重新导出全部框架 crate（`core` / `assets` / `gfx` / `audio` / `scene` / `headless`，
以及 feature 控制的 `devtools` / `egui_backend`），提供 `prelude` 与统一后端入口。

| feature | 默认 | 内容 |
|---|---|---|
| `egui` | 是 | `run_egui`、`RunConfig`、`egui_backend::WindowStatus` |
| `devtools` | 是 | `DevtoolsPlugin`、`DebugDraw` |

## 最小示例

```rust,no_run
use trellia::prelude::*;

fn main() -> anyhow::Result<()> {
    let cfg = trellia::RunConfig::default();
    let mut app = App::new();
    app.add_plugin(AssetsPlugin { root: cfg.asset_root.clone() })
        .add_plugin(GfxPlugin::new(cfg.logical_size))
        .add_plugin(AudioPlugin);
    trellia::run_egui(app, cfg)          // 或 trellia::run_headless(app) 做无窗口测试
}
```

完整示例见 `examples/lane_demo`。
