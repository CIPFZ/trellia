//! # `hello_window`
//!
//! M3 验收示例：在 letterbox 逻辑画面中显示 "Trellia" 文字与一个矩形（UC-20、UC-25）。

#![forbid(unsafe_code)]

use trellia::prelude::*;

/// 每帧提交一个矩形与一行文字。对应 UC-25。
pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn name(&self) -> &'static str {
        "HelloPlugin"
    }

    fn build(&self, app: &mut App) {
        app.add_system(Stage::Render, draw_hello);
    }
}

fn draw_hello(world: &mut World) {
    let list = world.resources.expect_mut::<DrawList>();
    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
        Color::from_hex(0x1E_272E),
    );
    list.rect(
        Layer::WORLD,
        Rect::from_center_size(Vec2::new(640.0, 420.0), Vec2::new(360.0, 180.0)),
        Color::from_hex(0x2E_86DE),
    );
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(640.0, 180.0),
        "Trellia",
        96.0,
        Color::WHITE,
        TextAlign::Center,
    );
}

/// 组装示例 App。
#[must_use]
pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))
        .add_plugin(HelloPlugin);
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_text_and_rect() {
        let mut runner = trellia::run_headless(build_app());
        runner.frame();
        let cmds = runner.last_draw_list();
        assert!(
            cmds.iter()
                .any(|c| matches!(&c.kind, DrawKind::Text { text, .. } if text == "Trellia"))
        );
        let rects = cmds
            .iter()
            .filter(|c| matches!(c.kind, DrawKind::Rect { .. }))
            .count();
        assert_eq!(rects, 2);
    }
}
