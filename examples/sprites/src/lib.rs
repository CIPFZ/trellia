//! # `sprites`
//!
//! M3 / M4 验收示例：N 个行者占位精灵以不同速度播放走路动画并向左移动（UC-22、UC-26），
//! 窗口任意缩放保持 letterbox（UC-20），按 [`FLIP_KEY`] 全体水平翻转（UC-24），
//! 鼠标点击处画一个圆并持续 1 秒、同时播放 `blip.ogg`（UC-12、UC-40、UC-50），
//! Esc 推入半透明暂停场景（UC-11、UC-61），F1 调试面板（UC-70、UC-71），标题栏显示帧耗时。

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

/// 示例音效句柄。
#[derive(Clone, Copy, Debug)]
pub struct Sounds {
    /// 点击音效。
    pub blip: Handle<AudioBytes>,
}

/// 示例插件：加载资产、生成行者、绘制背景与标题。对应 UC-20、UC-22、UC-30、UC-70。
pub struct SpritesPlugin {
    /// 精灵数量。
    pub count: usize,
}

impl Plugin for SpritesPlugin {
    fn name(&self) -> &'static str {
        "SpritesPlugin"
    }

    fn build(&self, app: &mut App) {
        match load(app.world_mut()) {
            Ok(loaded) => {
                spawn_walkers(app.world_mut(), loaded.atlas, loaded.walk, self.count);
                app.insert_resource(Sounds { blip: loaded.blip });
            }
            Err(e) => app.record_startup_error(e),
        }
        app.insert_resource(Flipped::default())
            .add_system(Stage::Render, background_system)
            .add_system(Stage::Render, title_system);
    }
}

/// 已加载的示例资产。
struct Loaded {
    atlas: Handle<Atlas>,
    walk: Handle<AnimationClip>,
    blip: Handle<AudioBytes>,
}

/// 加载行者图集、走路片段与点击音效。三个资产容器需同时可变借用，故临时取出再放回。
fn load(world: &mut World) -> Result<Loaded, AssetError> {
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
    let (atlas, walk) = result?;
    let blip = server.load_audio("demo/blip.ogg")?;
    let blip = world
        .resources
        .expect_mut::<Assets<AudioBytes>>()
        .add_with_path("demo/blip.ogg", blip);
    Ok(Loaded { atlas, walk, blip })
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

/// 主场景：行者移动、点击标记、输入。对应 UC-11、UC-24、UC-40、UC-50。
pub struct SpritesScene;

impl Scene for SpritesScene {
    fn name(&self) -> &'static str {
        "Sprites"
    }

    fn fixed_update(&mut self, world: &mut World) {
        for (t, walk) in world.entities.query_mut::<(&mut Transform2D, &Walk)>() {
            t.pos.x -= walk.0;
            if t.pos.x < -40.0 {
                t.teleport(Vec2::new(1320.0, t.pos.y));
            }
        }
        for (e, mark) in world.entities.query_mut::<(Entity, &mut ClickMark)>() {
            if mark.0.tick() {
                world.commands.despawn(e);
            }
        }
    }

    /// 每帧一次处理输入（同帧多个 tick 共享输入快照，放在 update 避免重复响应）。
    fn update(&mut self, world: &mut World) {
        let input = world.resources.expect::<Input>();
        let pause = input.key_just_pressed(Key::Escape);
        let flip = input.key_just_pressed(FLIP_KEY);
        let click = input
            .mouse_just_pressed(MouseButton::Left)
            .then_some(input.mouse_pos)
            .flatten();
        if pause {
            world
                .resources
                .expect_mut::<SceneCommands>()
                .push(PauseScene);
        }
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
            if let Some(sounds) = world.resources.get::<Sounds>().copied() {
                world
                    .resources
                    .expect_mut::<AudioQueue>()
                    .play_sfx(sounds.blip);
            }
        }
    }

    fn draw(&mut self, world: &mut World) {
        let Some(mut list) = world.resources.remove::<DrawList>() else {
            return;
        };
        for (t, _) in world.entities.query::<(&Transform2D, &ClickMark)>().iter() {
            list.circle(
                Layer::OVERLAY,
                t.pos,
                24.0,
                Color::from_hex(0xFD_CB6E).with_alpha(0.8),
            );
        }
        world.resources.insert(list);
    }
}

/// 半透明暂停覆盖层：下方画面冻结但可见，Esc 返回。对应 UC-11、UC-61。
pub struct PauseScene;

impl Scene for PauseScene {
    fn name(&self) -> &'static str {
        "Paused"
    }

    fn on_enter(&mut self, world: &mut World) {
        world.resources.expect_mut::<Time>().paused = true;
    }

    fn on_exit(&mut self, world: &mut World) {
        world.resources.expect_mut::<Time>().paused = false;
    }

    fn update(&mut self, world: &mut World) {
        if world
            .resources
            .expect::<Input>()
            .key_just_pressed(Key::Escape)
        {
            world.resources.expect_mut::<SceneCommands>().pop();
        }
    }

    fn draw(&mut self, world: &mut World) {
        let list = world.resources.expect_mut::<DrawList>();
        list.rect(
            Layer::OVERLAY,
            Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
            Color::BLACK.with_alpha(0.55),
        );
        list.text_aligned(
            Layer::OVERLAY,
            Vec2::new(640.0, 320.0),
            "PAUSED  (Esc)",
            64.0,
            Color::WHITE,
            TextAlign::Center,
        );
    }
}

fn background_system(world: &mut World) {
    world.resources.expect_mut::<DrawList>().rect(
        Layer::BACKGROUND,
        Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
        Color::from_hex(0x2D_3436),
    );
}

fn title_system(world: &mut World) {
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
        .add_plugin(AudioPlugin)
        .add_plugin(SpritesPlugin { count })
        .add_plugin(ScenePlugin::new(SpritesScene))
        .add_plugin(DevtoolsPlugin::default());
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

    fn walker_x(runner: &HeadlessRunner) -> f32 {
        runner
            .app()
            .world()
            .query::<(&Transform2D, &Walk)>()
            .iter()
            .map(|(t, _)| t.pos.x)
            .sum()
    }

    fn top(runner: &HeadlessRunner) -> Option<&'static str> {
        runner
            .app()
            .world()
            .resources
            .expect::<SceneStack>()
            .top_name()
    }

    fn time_mut(runner: &mut HeadlessRunner) -> &mut Time {
        runner.app_mut().world_mut().resources.expect_mut::<Time>()
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
        runner.click(Vec2::new(300.0, 200.0));
        assert_eq!(circles(&runner), vec![Vec2::new(300.0, 200.0)]);
        runner.frames(CLICK_MARK_TICKS - 1);
        assert_eq!(circles(&runner).len(), 1, "1 秒内仍可见");
        runner.frames(1);
        assert!(circles(&runner).is_empty(), "1 秒后消失");
    }

    #[test]
    fn flip_key_toggles_all_sprites() {
        let mut runner = trellia::run_headless(build_app(3));
        runner.press_key(FLIP_KEY);
        let flipped = runner
            .last_draw_list()
            .iter()
            .filter(|c| matches!(c.kind, DrawKind::Sprite { flip_x: true, .. }))
            .count();
        assert_eq!(flipped, 3);
    }

    #[test]
    fn audio_request_recorded() {
        let mut runner = trellia::run_headless(build_app(1));
        runner.frames(2);
        runner.click(Vec2::new(640.0, 360.0));
        runner.frames(2);
        let log = runner.take_audio_log();
        let blip = runner.app().world().resources.expect::<Sounds>().blip;
        assert_eq!(log.len(), 1);
        assert!(matches!(log[0], AudioRequest::PlaySfx { clip, .. } if clip == blip));
    }

    #[test]
    fn escape_pauses_and_resumes() {
        let mut runner = trellia::run_headless(build_app(4));
        runner.frames(3);
        runner.press_key(Key::Escape);
        runner.frame();
        assert_eq!(top(&runner), Some("Paused"));
        let frozen = walker_x(&runner);
        runner.frames(30);
        assert!((walker_x(&runner) - frozen).abs() < 1e-3, "暂停时画面冻结");
        assert_eq!(sprite_count(&runner), 4, "下方场景仍可见");
        runner.press_key(Key::Escape);
        runner.frames(2);
        assert_eq!(top(&runner), Some("Sprites"));
        assert!(walker_x(&runner) < frozen, "恢复后继续移动");
    }

    #[test]
    fn devtools_step_advances_one_tick_while_paused() {
        let mut runner = trellia::run_headless(build_app(1));
        runner.press_key(Key::F1);
        time_mut(&mut runner).paused = true;
        let tick = runner.app().world().resources.expect::<Time>().fixed_tick;
        time_mut(&mut runner).step_once = true;
        runner.frames(5);
        let time = runner.app().world().resources.expect::<Time>();
        assert_eq!(time.fixed_tick, tick + 1);
    }
}
