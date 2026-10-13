//! # trellia-gfx
//!
//! 职责：逻辑坐标系与视口（`Viewport`）、后端无关的绘制命令列表（`DrawList`）、
//! 精灵与图集（`Sprite` / `Atlas`）、以 tick 推进的序列帧动画（`Animator`）。
//!
//! 对应用例：UC-20、UC-21、UC-22、UC-23、UC-24、UC-25、UC-26、UC-27。
//!
//! 不负责：GPU 资源、窗口、相机滚动 / 缩放、场景图层级变换、粒子、后处理。

#![forbid(unsafe_code)]

mod anim;
mod draw;
mod load;
mod sprite;
mod types;
mod viewport;

pub use anim::{AnimationClip, AnimationEvent, AnimationFrame, Animator, load_clip};
pub use draw::{DrawCmd, DrawCmdBuilder, DrawKind, DrawList};
pub use load::{load_atlas_into, load_clip_into};
pub use sprite::{Atlas, AtlasFrame, Sprite, load_atlas};
pub use types::{Color, Layer, Rect, TextAlign};
pub use viewport::Viewport;

use glam::Vec2;
use trellia_assets::Assets;
use trellia_core::{App, Plugin, Stage};

/// 绘制插件。对应 UC-20、UC-21、UC-22、UC-23、UC-26。
///
/// `build`：插入 `Viewport`、`DrawList`、`Assets<Atlas>`、`Assets<AnimationClip>`；注册
/// `Events<AnimationEvent>`；添加系统：
///
/// - `Stage::First`：更新视口并把鼠标位置换算为逻辑坐标；
/// - `Stage::FixedUpdate`：动画推进（应先于游戏系统添加本插件，使同 tick 的游戏系统读到事件）；
/// - `Stage::Render`：清空 `DrawList`（应先于其他 Render 系统），然后提交精灵。
///
/// 排序由 `DrawList::sorted` 在后端读取时完成。
#[derive(Clone, Copy, Debug)]
pub struct GfxPlugin {
    /// 设计分辨率，如 `(1280, 720)`。
    pub logical_size: Vec2,
}

impl GfxPlugin {
    /// 以设计分辨率创建。
    #[must_use]
    pub fn new(logical_size: Vec2) -> Self {
        Self { logical_size }
    }
}

impl Plugin for GfxPlugin {
    fn name(&self) -> &'static str {
        "GfxPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(Viewport::new(self.logical_size))
            .insert_resource(DrawList::default())
            .insert_resource(Assets::<Atlas>::default())
            .insert_resource(Assets::<AnimationClip>::default());
        app.world_mut().events.register::<AnimationEvent>();
        app.add_system(Stage::First, viewport::viewport_system)
            .add_system(Stage::FixedUpdate, anim::animation_system)
            .add_system(Stage::Render, draw::clear_draw_list)
            .add_system(Stage::Render, sprite::sprite_render_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::{FrameInput, Input};

    #[test]
    fn mouse_is_converted_to_logical() {
        let mut app = App::new();
        app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)));
        // 2560 × 1440 窗口：缩放 2，无黑边；1920 × 720：缩放 1，左右黑边 320
        app.frame(FrameInput {
            window_size_px: (1920, 720),
            mouse_pos_px: Some((330.0, 10.0)),
            ..FrameInput::default()
        });
        let input = app.world().resources.expect::<Input>();
        assert_eq!(input.mouse_pos, Some(Vec2::new(10.0, 10.0)));
        assert!((app.world().resources.expect::<Viewport>().offset_px.x - 320.0).abs() < 1e-4);
    }
}
