//! 基础绘制类型（SPEC.md 6.2）。

use glam::Vec2;

/// 轴对齐矩形，逻辑坐标。对应 UC-21、UC-25、UC-72。
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// 左上角。
    pub min: Vec2,
    /// 右下角。
    pub max: Vec2,
}

impl Rect {
    /// 由左上角与尺寸构造。
    #[must_use]
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    /// 由中心与尺寸构造。
    #[must_use]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self::from_min_size(center - size * 0.5, size)
    }

    /// 尺寸。
    #[must_use]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// 中心。
    #[must_use]
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// 点是否在矩形内（含左上边界，不含右下边界）。
    #[must_use]
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.y >= self.min.y && p.x < self.max.x && p.y < self.max.y
    }

    /// 两矩形是否有面积大于 0 的重叠。
    #[must_use]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    /// 平移。
    #[must_use]
    pub fn translate(&self, d: Vec2) -> Rect {
        Rect {
            min: self.min + d,
            max: self.max + d,
        }
    }
}

/// RGBA 颜色，分量为线性光强度 0..1（ADR-004）。对应 UC-24、UC-25。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// 红。
    pub r: f32,
    /// 绿。
    pub g: f32,
    /// 蓝。
    pub b: f32,
    /// 不透明度。
    pub a: f32,
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl Color {
    /// 白色（着色时不改变原色）。
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);
    /// 黑色。
    pub const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);
    /// 全透明。
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    /// 不透明颜色（线性分量）。
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// 带不透明度的颜色（线性分量）。
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// 由 sRGB 十六进制 `0xRRGGBB` 构造，转换为线性分量。
    #[must_use]
    pub fn from_hex(hex: u32) -> Self {
        let [_, r, g, b] = hex.to_be_bytes();
        Self::rgb(srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b))
    }

    /// 替换不透明度（半透明预览，UC-24）。
    #[must_use]
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// 绘制层，小者先绘制（UC-21）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer(pub i16);

impl Layer {
    /// 背景。
    pub const BACKGROUND: Layer = Layer(-100);
    /// 世界实体。
    pub const WORLD: Layer = Layer(0);
    /// 弹丸。
    pub const PROJECTILE: Layer = Layer(10);
    /// 特效。
    pub const FX: Layer = Layer(20);
    /// 世界内覆盖层（选中框、预览）。
    pub const OVERLAY: Layer = Layer(100);
    /// 调试绘制（UC-72）。
    pub const DEBUG: Layer = Layer(1000);
}

/// 文字水平对齐方式。对应 UC-25。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// `pos` 为文字左端。
    Left,
    /// `pos` 为文字中心。
    Center,
    /// `pos` 为文字右端。
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_geometry() {
        let r = Rect::from_center_size(Vec2::new(10.0, 10.0), Vec2::new(4.0, 2.0));
        assert_eq!(r.min, Vec2::new(8.0, 9.0));
        assert_eq!(r.size(), Vec2::new(4.0, 2.0));
        assert_eq!(r.center(), Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::new(8.0, 9.0)));
        assert!(!r.contains(Vec2::new(12.0, 10.0)));
        let moved = r.translate(Vec2::new(4.0, 0.0));
        assert!(!r.intersects(&moved), "仅接触边界不算重叠");
        assert!(r.intersects(&r.translate(Vec2::new(3.0, 1.0))));
    }

    #[test]
    fn color_from_hex_is_linear() {
        let white = Color::from_hex(0xFF_FFFF);
        assert!((white.r - 1.0).abs() < 1e-6 && (white.a - 1.0).abs() < 1e-6);
        let mid = Color::from_hex(0x80_8080);
        assert!((mid.g - 0.2158).abs() < 1e-3, "sRGB 0x80 ≈ 线性 0.2158");
        let red = Color::from_hex(0xFF_0000).with_alpha(0.5);
        assert!(red.g.abs() < 1e-6 && (red.a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn layers_order() {
        assert!(Layer::BACKGROUND < Layer::WORLD && Layer::OVERLAY < Layer::DEBUG);
    }
}
