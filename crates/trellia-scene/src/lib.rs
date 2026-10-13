//! # trellia-scene
//!
//! 职责：场景栈（`SceneStack`）、场景生命周期（`Scene`）、覆盖层语义与帧末统一切换（`SceneCommands`）。
//!
//! 对应用例：UC-60、UC-61、UC-62。
//!
//! 不负责：场景图父子变换层级、场景序列化、过场动画。

#![forbid(unsafe_code)]
