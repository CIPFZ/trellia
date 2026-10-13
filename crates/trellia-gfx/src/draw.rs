//! 后端无关的绘制命令列表（SPEC.md 6.3）。

use glam::Vec2;
use trellia_assets::{Handle, Image};
use trellia_core::World;

use crate::{Color, Layer, Rect, TextAlign};

/// 一条绘制命令。对应 UC-21、UC-24、UC-25。
#[derive(Clone, Debug)]
pub struct DrawCmd {
    /// 层，小者先绘制。
    pub layer: Layer,
    /// 同层内按此排序，小者先绘制（UC-21）；非精灵命令为 0（ADR-004）。
    pub sort_y: f32,
    /// 提交顺序，前两者相同时的排序键。
    pub seq: u32,
    /// 绘制内容。
    pub kind: DrawKind,
}

/// 绘制内容。坐标均为逻辑坐标。对应 UC-24、UC-25、UC-27。
#[derive(Clone, Debug)]
pub enum DrawKind {
    /// 图像子矩形。
    Sprite {
        /// 图像。
        image: Handle<Image>,
        /// 图像中的像素矩形。
        src_px: Rect,
        /// 目标矩形。
        dst: Rect,
        /// 着色（乘法）。
        tint: Color,
        /// 水平翻转。
        flip_x: bool,
    },
    /// 矩形。
    Rect {
        /// 矩形。
        rect: Rect,
        /// 颜色。
        color: Color,
        /// 描边宽度；`None` 为实心。
        stroke: Option<f32>,
    },
    /// 圆。
    Circle {
        /// 圆心。
        center: Vec2,
        /// 半径。
        radius: f32,
        /// 颜色。
        color: Color,
        /// 描边宽度；`None` 为实心。
        stroke: Option<f32>,
    },
    /// 文字。
    Text {
        /// 锚点（含义由 `align` 决定，竖直方向为文字顶端）。
        pos: Vec2,
        /// 文本。
        text: String,
        /// 字号（逻辑像素）。
        size: f32,
        /// 颜色。
        color: Color,
        /// 水平对齐。
        align: TextAlign,
    },
}

/// 本帧绘制命令列表，作为资源存在 World 中（UC-21、UC-25、UC-27）。
///
/// `Stage::Render` 开始时由 `GfxPlugin` 清空；后端读取 `sorted()`。
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    cmds: Vec<DrawCmd>,
    next_seq: u32,
}

/// [`DrawList::sprite`] 返回的链式修饰器（ADR-004）。对应 UC-24。
pub struct DrawCmdBuilder<'a> {
    cmd: &'a mut DrawCmd,
}

impl DrawCmdBuilder<'_> {
    /// 设置着色。
    pub fn tint(&mut self, color: Color) -> &mut Self {
        if let DrawKind::Sprite { tint, .. } = &mut self.cmd.kind {
            *tint = color;
        }
        self
    }

    /// 设置水平翻转。
    pub fn flip_x(&mut self, flip: bool) -> &mut Self {
        if let DrawKind::Sprite { flip_x, .. } = &mut self.cmd.kind {
            *flip_x = flip;
        }
        self
    }
}

impl DrawList {
    /// 清空全部命令并重置提交序号。
    pub fn clear(&mut self) {
        self.cmds.clear();
        self.next_seq = 0;
    }

    fn push(&mut self, layer: Layer, sort_y: f32, kind: DrawKind) -> &mut DrawCmd {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.cmds.push(DrawCmd {
            layer,
            sort_y,
            seq,
            kind,
        });
        let last = self.cmds.len() - 1;
        &mut self.cmds[last]
    }

    /// 提交精灵（默认白色着色、不翻转），返回修饰器。
    pub fn sprite(
        &mut self,
        layer: Layer,
        sort_y: f32,
        image: Handle<Image>,
        src_px: Rect,
        dst: Rect,
    ) -> DrawCmdBuilder<'_> {
        let kind = DrawKind::Sprite {
            image,
            src_px,
            dst,
            tint: Color::WHITE,
            flip_x: false,
        };
        DrawCmdBuilder {
            cmd: self.push(layer, sort_y, kind),
        }
    }

    /// 提交实心矩形。
    pub fn rect(&mut self, layer: Layer, rect: Rect, color: Color) {
        self.push(
            layer,
            0.0,
            DrawKind::Rect {
                rect,
                color,
                stroke: None,
            },
        );
    }

    /// 提交矩形描边。
    pub fn rect_stroke(&mut self, layer: Layer, rect: Rect, color: Color, width: f32) {
        self.push(
            layer,
            0.0,
            DrawKind::Rect {
                rect,
                color,
                stroke: Some(width),
            },
        );
    }

    /// 提交实心圆。
    pub fn circle(&mut self, layer: Layer, center: Vec2, radius: f32, color: Color) {
        self.push(
            layer,
            0.0,
            DrawKind::Circle {
                center,
                radius,
                color,
                stroke: None,
            },
        );
    }

    /// 提交左对齐文字。
    pub fn text(
        &mut self,
        layer: Layer,
        pos: Vec2,
        text: impl Into<String>,
        size: f32,
        color: Color,
    ) {
        self.text_aligned(layer, pos, text, size, color, TextAlign::Left);
    }

    /// 提交指定对齐方式的文字。
    pub fn text_aligned(
        &mut self,
        layer: Layer,
        pos: Vec2,
        text: impl Into<String>,
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        self.push(
            layer,
            0.0,
            DrawKind::Text {
                pos,
                text: text.into(),
                size,
                color,
                align,
            },
        );
    }

    /// 按 `(layer, sort_y, seq)` 排序后返回，供后端消费。
    pub fn sorted(&mut self) -> &[DrawCmd] {
        self.cmds.sort_by(|a, b| {
            a.layer
                .cmp(&b.layer)
                .then(a.sort_y.total_cmp(&b.sort_y))
                .then(a.seq.cmp(&b.seq))
        });
        &self.cmds
    }

    /// 命令数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    /// 是否没有命令（ADR-004）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }
}

/// `Stage::Render` 首个系统：清空上一帧命令。
pub(crate) fn clear_draw_list(world: &mut World) {
    if let Some(list) = world.resources.get_mut::<DrawList>() {
        list.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_assets::Assets;

    #[test]
    fn drawlist_sort_order() {
        let mut images = Assets::<Image>::default();
        let img = images.add(Image {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        });
        let r = Rect::from_min_size(Vec2::ZERO, Vec2::ONE);
        let mut list = DrawList::default();
        list.rect(Layer::OVERLAY, r, Color::BLACK); // seq 0
        list.sprite(Layer::WORLD, 300.0, img, r, r); // seq 1
        list.sprite(Layer::WORLD, 100.0, img, r, r) // seq 2
            .tint(Color::from_hex(0x00_00FF))
            .flip_x(true);
        list.rect(Layer::BACKGROUND, r, Color::WHITE); // seq 3
        list.sprite(Layer::WORLD, 100.0, img, r, r); // seq 4
        list.text(Layer::DEBUG, Vec2::ZERO, "t", 12.0, Color::WHITE); // seq 5
        assert_eq!(list.len(), 6);

        let order: Vec<u32> = list.sorted().iter().map(|c| c.seq).collect();
        assert_eq!(order, vec![3, 2, 4, 1, 0, 5]);
        let DrawKind::Sprite { flip_x, tint, .. } = &list.sorted()[1].kind else {
            panic!("应为精灵");
        };
        assert!(*flip_x && tint.b > 0.99);

        list.clear();
        assert!(list.is_empty());
        list.circle(Layer::FX, Vec2::ZERO, 1.0, Color::WHITE);
        assert_eq!(list.sorted()[0].seq, 0);
    }
}
