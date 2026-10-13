//! `DrawList` → egui Painter（SPEC.md 11.2，ADR-005）。

use std::collections::HashMap;

use egui::{Color32, Mesh, Painter, Pos2, Stroke, StrokeKind, TextureHandle, TextureId};
use trellia_assets::{Assets, Handle, Image};
use trellia_gfx::{Color, DrawCmd, DrawKind, Rect, TextAlign, Viewport};

/// 线性 `Color` → egui 显示用 sRGB `Color32`（ADR-004）。
pub(crate) fn to_color32(c: Color) -> Color32 {
    Color32::from(egui::Rgba::from_rgba_unmultiplied(c.r, c.g, c.b, c.a))
}

/// 逻辑坐标 → egui 点。
#[derive(Clone, Copy)]
pub(crate) struct Mapper {
    pub viewport: Viewport,
    pub pixels_per_point: f32,
}

impl Mapper {
    pub fn pos(&self, p: trellia_core::Vec2) -> Pos2 {
        let px = self.viewport.logical_to_px(p) / self.pixels_per_point;
        Pos2::new(px.x, px.y)
    }

    pub fn rect(&self, r: Rect) -> egui::Rect {
        egui::Rect::from_min_max(self.pos(r.min), self.pos(r.max))
    }

    /// 逻辑长度 → egui 点。
    pub fn size(&self, l: f32) -> f32 {
        l * self.viewport.scale / self.pixels_per_point
    }
}

/// 首次遇到的 `Handle<Image>` 上传为 egui 纹理（`NEAREST`，像素风清晰）。
pub(crate) fn upload_textures(
    ctx: &egui::Context,
    cmds: &[DrawCmd],
    images: Option<&Assets<Image>>,
    textures: &mut HashMap<Handle<Image>, TextureHandle>,
) {
    let Some(images) = images else {
        return;
    };
    for cmd in cmds {
        if let DrawKind::Sprite { image, .. } = &cmd.kind
            && !textures.contains_key(image)
            && let Some(img) = images.get(*image)
        {
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [img.width as usize, img.height as usize],
                &img.rgba,
            );
            let tex = ctx.load_texture(format!("{image:?}"), color, egui::TextureOptions::NEAREST);
            textures.insert(*image, tex);
        }
    }
}

/// 按排序后的顺序绘制全部命令。连续的同纹理精灵合并为一个 Mesh。
pub(crate) fn paint(
    painter: &Painter,
    cmds: &[DrawCmd],
    map: Mapper,
    textures: &HashMap<Handle<Image>, TextureHandle>,
) {
    let mut batch: Option<Mesh> = None;
    let flush = |batch: &mut Option<Mesh>| {
        if let Some(mesh) = batch.take() {
            painter.add(egui::Shape::mesh(mesh));
        }
    };
    for cmd in cmds {
        match &cmd.kind {
            DrawKind::Sprite {
                image,
                src_px,
                dst,
                tint,
                flip_x,
            } => {
                let Some(tex) = textures.get(image) else {
                    continue;
                };
                let id: TextureId = tex.id();
                if batch.as_ref().is_some_and(|m| m.texture_id != id) {
                    flush(&mut batch);
                }
                let mesh = batch.get_or_insert_with(|| Mesh::with_texture(id));
                let [w, h] = tex.size();
                let (w, h) = (w as f32, h as f32);
                let (mut u0, mut u1) = (src_px.min.x / w, src_px.max.x / w);
                if *flip_x {
                    std::mem::swap(&mut u0, &mut u1);
                }
                let uv = egui::Rect::from_min_max(
                    Pos2::new(u0, src_px.min.y / h),
                    Pos2::new(u1, src_px.max.y / h),
                );
                mesh.add_rect_with_uv(map.rect(*dst), uv, to_color32(*tint));
            }
            DrawKind::Rect {
                rect,
                color,
                stroke,
            } => {
                flush(&mut batch);
                let r = map.rect(*rect);
                match stroke {
                    None => {
                        painter.rect_filled(r, 0.0, to_color32(*color));
                    }
                    Some(w) => {
                        painter.rect_stroke(
                            r,
                            0.0,
                            Stroke::new(map.size(*w), to_color32(*color)),
                            StrokeKind::Inside,
                        );
                    }
                }
            }
            DrawKind::Circle {
                center,
                radius,
                color,
                stroke,
            } => {
                flush(&mut batch);
                let (c, r) = (map.pos(*center), map.size(*radius));
                match stroke {
                    None => {
                        painter.circle_filled(c, r, to_color32(*color));
                    }
                    Some(w) => {
                        painter.circle_stroke(c, r, Stroke::new(map.size(*w), to_color32(*color)));
                    }
                }
            }
            DrawKind::Text {
                pos,
                text,
                size,
                color,
                align,
            } => {
                flush(&mut batch);
                let anchor = match align {
                    TextAlign::Left => egui::Align2::LEFT_TOP,
                    TextAlign::Center => egui::Align2::CENTER_TOP,
                    TextAlign::Right => egui::Align2::RIGHT_TOP,
                };
                painter.text(
                    map.pos(*pos),
                    anchor,
                    text,
                    egui::FontId::proportional(map.size(*size).max(1.0)),
                    to_color32(*color),
                );
            }
        }
    }
    flush(&mut batch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::Vec2;

    #[test]
    fn mapper_applies_letterbox_and_dpi() {
        let mut viewport = Viewport::new(Vec2::new(1280.0, 720.0));
        viewport.window_size_px = Vec2::new(1920.0, 720.0);
        viewport.scale = 1.0;
        viewport.offset_px = Vec2::new(320.0, 0.0);
        let map = Mapper {
            viewport,
            pixels_per_point: 2.0,
        };
        assert_eq!(map.pos(Vec2::new(0.0, 100.0)), Pos2::new(160.0, 50.0));
        assert!((map.size(10.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn color_roundtrip_white_and_black() {
        assert_eq!(to_color32(Color::WHITE), Color32::WHITE);
        assert_eq!(to_color32(Color::BLACK), Color32::BLACK);
        assert_eq!(
            to_color32(Color::from_hex(0x80_4020)),
            Color32::from_rgb(0x80, 0x40, 0x20)
        );
    }
}
