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

/// 常用类型的统一导入入口。
pub mod prelude {}
