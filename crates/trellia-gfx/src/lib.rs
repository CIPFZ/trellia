//! # trellia-gfx
//!
//! 职责：逻辑坐标系与视口（`Viewport`）、后端无关的绘制命令列表（`DrawList`）、
//! 精灵与图集（`Sprite` / `Atlas`）、以 tick 推进的序列帧动画（`Animator`）。
//!
//! 对应用例：UC-20、UC-21、UC-22、UC-23、UC-24、UC-25、UC-26、UC-27。
//!
//! 不负责：GPU 资源、窗口、相机滚动 / 缩放、场景图层级变换、粒子、后处理。

#![forbid(unsafe_code)]
