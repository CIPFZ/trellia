# trellia（门面）使用说明

## 依赖方式

游戏应用（如 `pvz-app`）依赖 `trellia`；游戏模拟层（如 `pvz-core`）只依赖 `trellia-core`（R4）。

```toml
[dependencies]
trellia = { version = "0.1", default-features = false, features = ["egui"] }  # 发布：去掉 devtools
```

## 推荐的插件顺序

1. `AssetsPlugin`（资产根目录、`Assets<Image>` / `Assets<AudioBytes>`）
2. `GfxPlugin`（视口、DrawList；FixedUpdate 首位推进动画；Render 首位清空命令）
3. `AudioPlugin`
4. 游戏资产 / 规则插件（在 `build` 中同步加载，失败 `record_startup_error`）
5. `ScenePlugin::new(初始场景)`
6. `DevtoolsPlugin`（可选）

## 两个入口

- `run_egui(app, RunConfig)`：窗口运行，返回 `anyhow::Result`。
- `run_headless(app) -> HeadlessRunner`：无窗口，测试用（`frame` / `click` / `press_key` /
  `last_draw_list` / `take_audio_log` / `snapshot`）。

## prelude

`trellia::prelude::*` 汇集 App / Plugin / Stage / World / Time / Timer / GameRng / Input、
资产、绘制、动画、音频、场景、headless 与（feature 开启时）调试类型；`trellia::egui` 为 egui 核心 crate。
