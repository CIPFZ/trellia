//! M2 验收：精灵经动画推进后出现在 DrawList 中（UC-21、UC-22、UC-27）。

use trellia_assets::{AssetServer, Assets, AssetsPlugin, Image};
use trellia_core::{App, Transform2D, Vec2};
use trellia_gfx::{
    AnimationClip, Animator, Atlas, DrawKind, GfxPlugin, Sprite, load_atlas, load_clip,
};
use trellia_headless::HeadlessRunner;

#[test]
fn sprite_appears_in_drawlist() {
    let mut app = App::new();
    app.add_plugin(AssetsPlugin::default())
        .add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)));
    let world = app.world_mut();
    let server = world.resources.expect::<AssetServer>().clone();
    let mut images = world
        .resources
        .remove::<Assets<Image>>()
        .expect("AssetsPlugin 已插入");
    let mut atlases = world
        .resources
        .remove::<Assets<Atlas>>()
        .expect("GfxPlugin 已插入");
    let mut clips = world
        .resources
        .remove::<Assets<AnimationClip>>()
        .expect("GfxPlugin 已插入");
    let atlas =
        load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron").expect("图集");
    let walk = load_clip(
        &server,
        &atlases,
        &mut clips,
        atlas,
        "demo/walker_walk.anim.ron",
    )
    .expect("片段");
    let walk_1 =
        atlases.expect(atlas).frames[atlases.expect(atlas).frame("walk_1").expect("帧名")].src_px;
    world.resources.insert(images);
    world.resources.insert(atlases);
    world.resources.insert(clips);
    world.spawn((
        Transform2D::new(Vec2::new(640.0, 400.0)),
        Sprite::new(atlas),
        Animator::new(walk),
    ));

    let mut runner = HeadlessRunner::new(app);
    runner.frames(30);

    let sprites: Vec<_> = runner
        .last_draw_list()
        .iter()
        .filter_map(|cmd| match &cmd.kind {
            DrawKind::Sprite { src_px, .. } => Some(*src_px),
            _ => None,
        })
        .collect();
    assert_eq!(
        sprites,
        vec![walk_1],
        "30 tick 后应恰有 1 条精灵命令且为 walk_1 帧"
    );
}
