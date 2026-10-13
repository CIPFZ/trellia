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

#![forbid(unsafe_code)]
