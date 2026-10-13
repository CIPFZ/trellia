//! # trellia-headless
//!
//! 职责：无窗口后端 `HeadlessRunner`：以固定 `real_dt` 驱动 `App`、注入输入、
//! 记录排序后的 `DrawList` 与音频请求、生成可逐字节比较的 World 快照。
//!
//! 对应用例：UC-13、UC-53、UC-73、UC-74。
//!
//! 不负责：实际渲染与音频播放、输入录制与回放 UI。

#![forbid(unsafe_code)]
