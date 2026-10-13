//! 视口与逻辑坐标系（SPEC.md 6.1）。

use glam::Vec2;
use trellia_core::{Input, World};

use crate::Rect;

/// 逻辑分辨率到窗口像素的等比缩放与 letterbox（UC-20）。
///
/// 逻辑坐标：原点左上，x 向右，y 向下，单位为逻辑像素。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// 设计分辨率（如 1280 × 720）。
    pub logical_size: Vec2,
    /// 当前窗口物理像素尺寸，每帧由 `Stage::First` 从输入更新。
    pub window_size_px: Vec2,
    /// `min(window.x / logical.x, window.y / logical.y)`。
    pub scale: f32,
    /// 逻辑区域左上角在窗口中的像素偏移（黑边宽度）。
    pub offset_px: Vec2,
}

impl Viewport {
    /// 以设计分辨率创建，窗口尺寸初始等于逻辑尺寸（ADR-004）。
    #[must_use]
    pub fn new(logical_size: Vec2) -> Self {
        Self {
            logical_size,
            window_size_px: logical_size,
            scale: 1.0,
            offset_px: Vec2::ZERO,
        }
    }

    /// 逻辑坐标 → 窗口像素。
    #[must_use]
    pub fn logical_to_px(&self, p: Vec2) -> Vec2 {
        self.offset_px + p * self.scale
    }

    /// 窗口像素 → 逻辑坐标。
    #[must_use]
    pub fn px_to_logical(&self, p: Vec2) -> Vec2 {
        (p - self.offset_px) / self.scale
    }

    /// 逻辑区域 `(0, 0)` – `logical_size`。
    #[must_use]
    pub fn logical_rect(&self) -> Rect {
        Rect::from_min_size(Vec2::ZERO, self.logical_size)
    }

    /// 更新窗口尺寸并重算缩放与偏移；任一分量为 0 时保持不变（ADR-004）。
    pub(crate) fn set_window_size(&mut self, size_px: Vec2) {
        if size_px.x <= 0.0 || size_px.y <= 0.0 {
            return;
        }
        self.window_size_px = size_px;
        self.scale = (size_px.x / self.logical_size.x).min(size_px.y / self.logical_size.y);
        self.offset_px = (size_px - self.logical_size * self.scale) * 0.5;
    }
}

/// `Stage::First`：按本帧窗口尺寸更新视口，并把鼠标位置改写为逻辑坐标（ADR-003）。
pub(crate) fn viewport_system(world: &mut World) {
    let (window, mouse_px) = {
        let input = world.resources.expect::<Input>();
        (input.window_size_px, input.mouse_pos_px)
    };
    let Some(viewport) = world.resources.get_mut::<Viewport>() else {
        return;
    };
    viewport.set_window_size(window);
    let viewport = *viewport;
    world.resources.expect_mut::<Input>().mouse_pos = mouse_px.map(|p| viewport.px_to_logical(p));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport(w: f32, h: f32) -> Viewport {
        let mut v = Viewport::new(Vec2::new(1280.0, 720.0));
        v.set_window_size(Vec2::new(w, h));
        v
    }

    #[test]
    fn viewport_letterbox_wide() {
        let v = viewport(1920.0, 720.0);
        assert!((v.scale - 1.0).abs() < 1e-6);
        assert!((v.offset_px - Vec2::new(320.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn viewport_letterbox_tall() {
        let v = viewport(1280.0, 1440.0);
        assert!((v.scale - 1.0).abs() < 1e-6);
        assert!((v.offset_px - Vec2::new(0.0, 360.0)).length() < 1e-4);
    }

    #[test]
    fn viewport_roundtrip() {
        for (w, h) in [
            (1920.0, 1080.0),
            (800.0, 600.0),
            (3000.0, 900.0),
            (1280.0, 720.0),
        ] {
            let v = viewport(w, h);
            for p in [Vec2::ZERO, Vec2::new(640.0, 360.0), Vec2::new(1279.0, 1.5)] {
                assert!((v.px_to_logical(v.logical_to_px(p)) - p).length() < 1e-3);
            }
        }
        assert_eq!(
            viewport(800.0, 600.0).logical_rect().max,
            Vec2::new(1280.0, 720.0)
        );
    }

    #[test]
    fn zero_window_keeps_previous_scale() {
        let mut v = viewport(640.0, 360.0);
        v.set_window_size(Vec2::ZERO);
        assert!((v.scale - 0.5).abs() < 1e-6);
    }
}
