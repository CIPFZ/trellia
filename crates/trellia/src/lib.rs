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

pub use trellia_core as core;
pub use trellia_headless as headless;

use trellia_core::App;
use trellia_headless::HeadlessRunner;

/// 常用类型的统一导入入口。对应 UC-73、UC-74。
pub mod prelude {
    pub use trellia_core::{
        App, AppExit, CommandBuffer, Component, Easing, Entity, Event, EventBus, Events,
        FrameInput, FrameOutput, GameRng, Input, InputEvent, Key, MouseButton, Plugin, Resource,
        Resources, Stage, System, Time, Timer, Transform2D, TweenF32, TweenVec2, Vec2, World,
    };
    pub use trellia_headless::{HeadlessRunner, Snapshot};
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
