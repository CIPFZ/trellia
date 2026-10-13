//! # trellia-egui
//!
//! 职责：基于 eframe 的窗口后端：采集输入并翻译为 `FrameInput`、驱动 `App::frame`、
//! 用 egui Painter 绘制 `DrawList`（letterbox）、调用场景 UI、用 rodio 执行音频请求。
//!
//! 对应用例：UC-20、UC-24、UC-25、UC-40、UC-41、UC-42、UC-50、UC-51、UC-52。
//!
//! 不负责：游戏逻辑、WASM / Web、手柄输入、自定义着色器。

#![forbid(unsafe_code)]
