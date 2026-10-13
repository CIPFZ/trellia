//! # trellia-core
//!
//! 职责：应用骨架（`App` / `Plugin` / `Stage`）、ECS 封装（`World`，基于 hecs）、
//! 时间（`Time`）、事件（`Events<T>`）、资源（`Resources`）、计时器（`Timer`）、
//! 补间（`TweenF32` / `TweenVec2`）、种子化随机数（`GameRng`）与输入状态（`Input`）。
//! 它是唯一允许被游戏模拟层依赖的 crate（SPEC.md 3.2 R1 / R4）。
//!
//! 对应用例：UC-03、UC-04、UC-05、UC-06、UC-10、UC-11、UC-12、UC-13、UC-14、
//! UC-40、UC-41、UC-42、UC-73、UC-74。
//!
//! 不负责：渲染、音频、资产加载、场景栈、窗口与设备。
//!
//! ```
//! use trellia_core::{App, Stage, Time, World};
//!
//! fn count_ticks(world: &mut World) {
//!     *world.resources.expect_mut::<u64>() += 1;
//! }
//!
//! let mut app = App::new();
//! app.insert_resource(0_u64).add_system(Stage::FixedUpdate, count_ticks);
//! app.step_ticks(60);
//! assert_eq!(*app.world().resources.expect::<u64>(), 60);
//! assert_eq!(app.world().resources.expect::<Time>().fixed_tick, 60);
//! ```

#![forbid(unsafe_code)]

mod app;
mod events;
mod input;
mod resources;
mod rng;
mod time;
mod tween;
mod world;

pub use app::{App, AppExit, Plugin, Stage, System};
pub use events::{Event, EventBus, Events};
pub use input::{FrameInput, FrameOutput, Input, InputEvent, Key, MouseButton};
pub use resources::{Resource, Resources};
pub use rng::GameRng;
pub use time::{Time, Timer};
pub use tween::{Easing, TweenF32, TweenVec2};
pub use world::{CommandBuffer, Component, Transform2D, World};

/// 数学库重新导出，游戏层无需单独依赖 glam（ADR-003）。
pub use glam;
pub use glam::Vec2;
/// ECS 存储重新导出，游戏层无需单独依赖 hecs 即可书写查询（ADR-001）。
pub use hecs;
pub use hecs::{Entity, NoSuchEntity};
