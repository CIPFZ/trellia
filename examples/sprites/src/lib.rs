//! # `sprites`
//!
//! M3 验收示例：N 个行者占位精灵以不同速度播放走路动画并向左移动（UC-22、UC-26），
//! 窗口任意缩放保持 letterbox（UC-20），按 [`FLIP_KEY`] 全体水平翻转（UC-24），
//! 鼠标点击处画一个圆并持续 1 秒（UC-12、UC-40），标题栏显示帧耗时（UC-70）。

#![forbid(unsafe_code)]

use trellia::egui_backend::WindowStatus;
use trellia::prelude::*;

/// 翻转键。SPEC.md 13 M3 写作“F 键”，但 v1 `Key` 枚举没有字母键（4.9），暂用 F2（见 docs/progress.md 待确认问题）。
pub const FLIP_KEY: Key = Key::F2;

/// 点击标记存活的 tick 数（1 秒）。
pub const CLICK_MARK_TICKS: u32 = 60;

const WALKER_SPEEDS: [(u32, u32); 4] = [(1, 2), (1, 1), (3, 2), (2, 1)];

/// 行者的水平速度（逻辑像素 / tick，向左）。
#[derive(Clone, Copy, Debug)]
struct Walk(f32);

/// 点击标记：到期后销毁。
#[derive(Clone, Copy, Debug)]
pub struct ClickMark(pub Timer);

/// 当前是否翻转。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flipped(pub bool);

/// 示例插件。对应 UC-20、UC-22、UC-24、UC-40。
pub struct SpritesPlugin {
    /// 精灵数量。
    pub count: usize,
}

impl Plugin for SpritesPlugin {
    fn name(&self) -> &'static str {
        "SpritesPlugin"
    }

    fn build(&self, app: &mut App) {
        match load_walker(app.world_mut()) {
            Ok((atlas, walk)) => spawn_walkers(app.world_mut(), atlas, walk, self.count),
            Err(e) => app.record_startup_error(e),
        }
        app.insert_resource(Flipped::default())
            .add_system(Stage::Update, input_system)
            .add_system(Stage::FixedUpdate, walk_system)
            .add_system(Stage::FixedUpdate, click_mark_system)
            .add_system(Stage::Render, render_system);
    }
}

/// 加载行者图集与走路片段。三个资产容器需同时可变借用，故临时取出再放回。
fn load_walker(world: &mut World) -> Result<(Handle<Atlas>, Handle<AnimationClip>), AssetError> {
    let Some(server) = world.resources.get::<AssetServer>().cloned() else {
        return Err(AssetError::RootNotFound("assets".into()));
    };
    let mut images = world
        .resources
        .remove::<Assets<Image>>()
        .unwrap_or_default();
    let mut atlases = world
        .resources
        .remove::<Assets<Atlas>>()
        .unwrap_or_default();
    let mut clips = world
        .resources
        .remove::<Assets<AnimationClip>>()
        .unwrap_or_default();
    let result =
        load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron").and_then(|atlas| {
            let walk = load_clip(
                &server,
                &atlases,
                &mut clips,
                atlas,
                "demo/walker_walk.anim.ron",
            )?;
            Ok((atlas, walk))
        });
    world.resources.insert(images);
    world.resources.insert(atlases);
    world.resources.insert(clips);
    result
}

fn spawn_walkers(
    world: &mut World,
    atlas: Handle<Atlas>,
    walk: Handle<AnimationClip>,
    count: usize,
) {
    let cols = ((count as f32 * 16.0 / 9.0).sqrt().ceil() as usize).max(1);
    let rows = count.div_ceil(cols).max(1);
    for i in 0..count {
        let (c, r) = (i % cols, i / cols);
        let x = (c as f32 + 0.5) * 1280.0 / cols as f32;
        let y = 120.0 + (r as f32 + 1.0) * 560.0 / rows as f32;
        let mut animator = Animator::new(walk);
        let (num, den) = WALKER_SPEEDS[i % WALKER_SPEEDS.len()];
        animator.set_speed(num, den);
        world.spawn((
            Transform2D::new(Vec2::new(x, y)),
            Sprite::new(atlas),
            animator,
            Walk(0.4 + (i % 5) as f32 * 0.3),
        ));
    }
}

/// 每帧一次处理输入（同帧多个 tick 共享输入快照，放在 Update 避免重复响应）。
fn input_system(world: &mut World) {
    let input = world.resources.expect::<Input>();
    let flip = input.key_just_pressed(FLIP_KEY);
    let click = input
        .mouse_just_pressed(MouseButton::Left)
        .then_some(input.mouse_pos)
        .flatten();
    if flip {
        let flipped = world.resources.expect_mut::<Flipped>();
        flipped.0 = !flipped.0;
        let value = flipped.0;
        for sprite in world.entities.query_mut::<&mut Sprite>() {
            sprite.flip_x = value;
        }
    }
    if let Some(pos) = click {
        world.commands.spawn((
            Transform2D::new(pos),
            ClickMark(Timer::once(CLICK_MARK_TICKS)),
        ));
    }
}

fn walk_system(world: &mut World) {
    for (t, walk) in world.entities.query_mut::<(&mut Transform2D, &Walk)>() {
        t.pos.x -= walk.0;
        if t.pos.x < -40.0 {
            t.teleport(Vec2::new(1320.0, t.pos.y));
        }
    }
}

fn click_mark_system(world: &mut World) {
    for (e, mark) in world.entities.query_mut::<(Entity, &mut ClickMark)>() {
        if mark.0.tick() {
            world.commands.despawn(e);
        }
    }
}

fn render_system(world: &mut World) {
    let Some(mut list) = world.resources.remove::<DrawList>() else {
        return;
    };
    list.rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
        Color::from_hex(0x2D_3436),
    );
    for (t, _) in world.entities.query::<(&Transform2D, &ClickMark)>().iter() {
        list.circle(
            Layer::OVERLAY,
            t.pos,
            24.0,
            Color::from_hex(0xFD_CB6E).with_alpha(0.8),
        );
    }
    world.resources.insert(list);

    let frame = world.resources.expect::<Time>().frame;
    let count = world.entities.query::<&Walk>().iter().count();
    if frame.is_multiple_of(30)
        && let Some(status) = world.resources.get_mut::<WindowStatus>()
    {
        status.title = format!("sprites | {count} sprites | {:.2} ms", status.cpu_frame_ms);
    }
}

/// 组装示例 App。
#[must_use]
pub fn build_app(count: usize) -> App {
    let mut app = App::new();
    app.add_plugin(AssetsPlugin::default())
        .add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))
        .add_plugin(SpritesPlugin { count });
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite_count(runner: &HeadlessRunner) -> usize {
        runner
            .last_draw_list()
            .iter()
            .filter(|c| matches!(c.kind, DrawKind::Sprite { .. }))
            .count()
    }

    fn circles(runner: &HeadlessRunner) -> Vec<Vec2> {
        runner
            .last_draw_list()
            .iter()
            .filter_map(|c| match c.kind {
                DrawKind::Circle { center, .. } => Some(center),
                _ => None,
            })
            .collect()
    }

    fn input(events: Vec<InputEvent>, mouse: Option<(f32, f32)>) -> FrameInput {
        FrameInput {
            real_dt: Time::FIXED_DT,
            window_size_px: (1280, 720),
            mouse_pos_px: mouse,
            events,
        }
    }

    #[test]
    fn draws_requested_sprite_count() {
        let mut runner = trellia::run_headless(build_app(10));
        runner.frames(5);
        assert_eq!(sprite_count(&runner), 10);
    }

    #[test]
    fn click_draws_circle_for_one_second() {
        let mut runner = trellia::run_headless(build_app(1));
        runner.frame_with(input(
            vec![InputEvent::MousePressed(MouseButton::Left)],
            Some((300.0, 200.0)),
        ));
        runner.frame_with(input(
            vec![InputEvent::MouseReleased(MouseButton::Left)],
            None,
        ));
        assert_eq!(circles(&runner), vec![Vec2::new(300.0, 200.0)]);
        runner.frames(CLICK_MARK_TICKS - 2);
        assert_eq!(circles(&runner).len(), 1, "1 秒内仍可见");
        runner.frames(2);
        assert!(circles(&runner).is_empty(), "1 秒后消失");
    }

    #[test]
    fn flip_key_toggles_all_sprites() {
        let mut runner = trellia::run_headless(build_app(3));
        runner.frame_with(input(vec![InputEvent::KeyPressed(FLIP_KEY)], None));
        let flipped = runner
            .last_draw_list()
            .iter()
            .filter(|c| matches!(c.kind, DrawKind::Sprite { flip_x: true, .. }))
            .count();
        assert_eq!(flipped, 3);
    }
}
