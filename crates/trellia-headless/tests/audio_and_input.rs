//! headless 输入注入与音频日志（UC-40、UC-41、UC-53）。

use trellia_assets::{Assets, AudioBytes, AudioKind, Handle};
use trellia_audio::{AudioPlugin, AudioQueue, AudioRequest};
use trellia_core::{App, Input, Key, MouseButton, Plugin, Stage, Vec2, World};
use trellia_gfx::GfxPlugin;
use trellia_headless::HeadlessRunner;

struct Blip(Handle<AudioBytes>);

#[derive(Default)]
struct Seen {
    clicks: Vec<Vec2>,
    keys: u32,
}

fn react(world: &mut World) {
    let input = world.resources.expect::<Input>();
    let click = input
        .mouse_just_pressed(MouseButton::Left)
        .then_some(input.mouse_pos)
        .flatten();
    let key = input.key_just_pressed(Key::Space);
    if let Some(pos) = click {
        let blip = world.resources.expect::<Blip>().0;
        world.resources.expect_mut::<AudioQueue>().play_sfx(blip);
        world.resources.expect_mut::<Seen>().clicks.push(pos);
    }
    if key {
        world.resources.expect_mut::<Seen>().keys += 1;
    }
}

struct TestPlugin;
impl Plugin for TestPlugin {
    fn name(&self) -> &'static str {
        "TestPlugin"
    }
    fn build(&self, app: &mut App) {
        let blip = app
            .world_mut()
            .resources
            .expect_mut::<Assets<AudioBytes>>()
            .add(AudioBytes {
                bytes: Vec::new().into(),
                kind: AudioKind::Ogg,
            });
        app.insert_resource(Blip(blip))
            .insert_resource(Seen::default())
            .add_system(Stage::Update, react);
    }
}

fn runner() -> HeadlessRunner {
    let mut app = App::new();
    app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))
        .add_plugin(AudioPlugin)
        .add_plugin(TestPlugin);
    HeadlessRunner::new(app)
}

#[test]
fn click_and_key_are_seen_once() {
    let mut r = runner();
    r.click(Vec2::new(300.0, 400.0));
    r.press_key(Key::Space);
    r.frames(3);
    let seen = r.app().world().resources.expect::<Seen>();
    assert_eq!(seen.clicks, vec![Vec2::new(300.0, 400.0)]);
    assert_eq!(seen.keys, 1);
}

#[test]
fn audio_log_accumulates_until_taken() {
    let mut r = runner();
    r.click(Vec2::new(10.0, 10.0));
    r.frames(2);
    r.click(Vec2::new(20.0, 10.0));
    let log = r.take_audio_log();
    assert_eq!(log.len(), 2);
    assert!(
        log.iter()
            .all(|req| matches!(req, AudioRequest::PlaySfx { .. }))
    );
    assert!(r.take_audio_log().is_empty());
}
