//! # trellia
//!
//! 职责：门面 crate，re-export 各框架 crate 并提供 `prelude`，统一后端入口。
//!
//! Feature：`egui`（默认，egui 窗口后端）、`devtools`（默认，调试面板）。
//!
//! 对应用例：UC-73、UC-74（游戏层通过统一入口组装 App 并在 headless 下测试）。
//!
//! 不负责：任何自身逻辑；所有功能来自被 re-export 的 crate。

#![forbid(unsafe_code)]

pub use trellia_assets as assets;
pub use trellia_core as core;
/// egui 窗口后端（feature `egui`）。
#[cfg(feature = "egui")]
pub use trellia_egui as egui_backend;
/// 窗口运行配置（feature `egui`，ADR-005）。
#[cfg(feature = "egui")]
pub use trellia_egui::RunConfig;
pub use trellia_gfx as gfx;
pub use trellia_headless as headless;

use trellia_core::App;
use trellia_headless::HeadlessRunner;

/// 常用类型的统一导入入口。对应 UC-73、UC-74。
pub mod prelude {
    pub use trellia_assets::{
        AssetError, AssetServer, Assets, AssetsPlugin, AudioBytes, Handle, Image,
    };
    pub use trellia_audio::{AudioBus, AudioClip, AudioPlugin, AudioQueue, AudioRequest};
    pub use trellia_core::{
        App, AppExit, CommandBuffer, Component, Easing, Entity, Event, EventBus, Events,
        FrameInput, FrameOutput, GameRng, Input, InputEvent, Key, MouseButton, Plugin, Resource,
        Resources, Stage, System, Time, Timer, Transform2D, TweenF32, TweenVec2, Vec2, World,
    };
    #[cfg(feature = "devtools")]
    pub use trellia_devtools::{DebugDraw, DevtoolsPlugin};
    pub use trellia_gfx::{
        AnimationClip, AnimationEvent, Animator, Atlas, Color, DrawCmd, DrawKind, DrawList,
        GfxPlugin, Layer, Rect, Sprite, TextAlign, Viewport, load_atlas, load_clip,
    };
    pub use trellia_headless::{HeadlessRunner, Snapshot};
    pub use trellia_scene::{Scene, SceneCommands, ScenePlugin, SceneStack};
}

/// 以 egui 窗口后端运行 App，直到窗口关闭或请求退出（feature `egui`）。对应 UC-20、UC-40。
///
/// # Errors
///
/// 启动期错误、未添加 `GfxPlugin`、`logical_size` 不一致或窗口初始化失败时返回错误（ADR-005）。
#[cfg(feature = "egui")]
pub fn run_egui(app: App, cfg: RunConfig) -> Result<(), anyhow::Error> {
    trellia_egui::run(app, cfg)
}

/// 以无窗口后端运行 App（始终可用）。对应 UC-73、UC-74。
#[must_use]
pub fn run_headless(app: App) -> HeadlessRunner {
    HeadlessRunner::new(app)
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn run_headless_drives_app() {
        let mut runner = super::run_headless(App::new());
        runner.frames(2);
        assert_eq!(
            runner.app().world().resources.expect::<Time>().fixed_tick,
            2
        );
    }
}
