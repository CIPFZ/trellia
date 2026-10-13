//! # trellia-audio
//!
//! 职责：音频请求队列（`AudioQueue`）与音频后端 trait（`AudioBackend`）。
//! 模拟层只发出“播放意图”，不触碰音频设备。
//!
//! 对应用例：UC-50、UC-51、UC-52、UC-53。
//!
//! 不负责：音频设备、解码、音频总线 / 混音、空间音频。

#![forbid(unsafe_code)]
