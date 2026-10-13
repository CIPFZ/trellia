//! # trellia-assets
//!
//! 职责：带类型的轻量资产句柄（`Handle<T>`）、资产容器（`Assets<T>`）、
//! 同步加载与解码（`AssetServer`：RON / PNG / 音频原始字节）。
//!
//! 对应用例：UC-30、UC-31、UC-32。
//!
//! 不负责：异步加载、资产热重载、资产卸载、GPU 上传、音频解码。

#![forbid(unsafe_code)]
