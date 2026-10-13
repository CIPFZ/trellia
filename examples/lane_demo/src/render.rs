//! 占位渲染（SPEC.md 10.3，UC-25、UC-27）：所有实体用几何图形 + 文字绘制。

use trellia::devtools::DebugDraw;
use trellia::prelude::*;

use crate::rules::{
    self, CELL_COUNT, LANE_HEIGHT, LANE_Y, Placeholder, Projectile, Turret, WALKER_COUNT,
    WALKER_HALF_WIDTH, Walker, WalkerState,
};

const LOGICAL: Vec2 = Vec2::new(1280.0, 720.0);

/// 战斗画面：背景、格子、炮塔 / 行者占位、弹丸、HUD、调试形状。
pub fn draw_battle(world: &mut World) {
    let Some(mut list) = world.resources.remove::<DrawList>() else {
        return;
    };
    let alpha = world.resources.expect::<Time>().alpha;

    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, LOGICAL),
        Color::from_hex(0x1E_3A2B),
    );
    for cell in 0..CELL_COUNT {
        let shade = if cell % 2 == 0 { 0x3C_7A4E } else { 0x35_6E45 };
        list.rect(
            Layer::BACKGROUND,
            rules::cell_rect(cell),
            Color::from_hex(shade),
        );
    }

    // 炮塔先、行者后，同层内按提交顺序绘制
    for (t, p, _) in world
        .query::<(&Transform2D, &Placeholder, &Turret)>()
        .iter()
    {
        placeholder(&mut list, t.lerp(alpha), p);
    }
    for (t, p, w) in world
        .query::<(&Transform2D, &Placeholder, &Walker)>()
        .iter()
    {
        let biting = matches!(w.state, WalkerState::Biting(_));
        let p = if biting {
            Placeholder { label: "W!", ..*p }
        } else {
            *p
        };
        placeholder(&mut list, t.lerp(alpha), &p);
    }
    for (t, _) in world.query::<(&Transform2D, &Projectile)>().iter() {
        list.circle(
            Layer::PROJECTILE,
            t.lerp(alpha),
            8.0,
            Color::from_hex(0xF1_C40F),
        );
    }
    for (t, turret) in world.query::<(&Transform2D, &Turret)>().iter() {
        list.text_aligned(
            Layer::OVERLAY,
            t.pos + Vec2::new(0.0, 38.0),
            format!("hp {}", turret.hp),
            16.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }

    if let Some((_, state)) = rules::lane(world) {
        list.text(
            Layer::OVERLAY,
            Vec2::new(24.0, 16.0),
            format!(
                "Resources: {}    Walkers: {}/{} defeated    Click a cell to place a turret (50)",
                state.money, state.killed, WALKER_COUNT
            ),
            24.0,
            Color::WHITE,
        );
    }
    world.resources.insert(list);
    debug_shapes(world);
}

fn placeholder(list: &mut DrawList, pos: Vec2, p: &Placeholder) {
    list.rect(Layer::WORLD, Rect::from_center_size(pos, p.size), p.color);
    list.text_aligned(
        Layer::WORLD,
        pos - Vec2::new(0.0, 14.0),
        p.label,
        28.0,
        Color::WHITE,
        TextAlign::Center,
    );
}

/// DebugDraw 开启时：格子边界与行者碰撞区间（UC-72）。
fn debug_shapes(world: &mut World) {
    let intervals: Vec<f32> = world
        .query::<(&Transform2D, &Walker)>()
        .iter()
        .map(|(t, _)| t.pos.x)
        .collect();
    let Some(debug) = world.resources.get_mut::<DebugDraw>() else {
        return;
    };
    if !debug.enabled() {
        return;
    }
    for cell in 0..CELL_COUNT {
        debug.rect(rules::cell_rect(cell), Color::from_hex(0xEC_F0F1));
    }
    for x in intervals {
        let rect = Rect::from_min_size(
            Vec2::new(x - WALKER_HALF_WIDTH, LANE_Y - LANE_HEIGHT / 2.0),
            Vec2::new(2.0 * WALKER_HALF_WIDTH, LANE_HEIGHT),
        );
        debug.rect(rect, Color::from_hex(0xE7_4C3C));
        debug.text(rect.min - Vec2::new(0.0, 18.0), format!("x {x:.1}"));
    }
}

/// 半透明覆盖层 + 标题 + 提示。
pub fn overlay(world: &mut World, title: &str, hint: &str) {
    let list = world.resources.expect_mut::<DrawList>();
    list.rect(
        Layer::OVERLAY,
        Rect::from_min_size(Vec2::ZERO, LOGICAL),
        Color::BLACK.with_alpha(0.6),
    );
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(640.0, 250.0),
        title,
        72.0,
        Color::WHITE,
        TextAlign::Center,
    );
    list.text_aligned(
        Layer::OVERLAY,
        Vec2::new(640.0, 340.0),
        hint,
        24.0,
        Color::from_hex(0xBD_C3C7),
        TextAlign::Center,
    );
}
