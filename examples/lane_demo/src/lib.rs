//! # `lane_demo`
//!
//! M5 垂直切片（SPEC.md 13 M5）：只有一条车道的极简塔防。炮塔（绿色方块 "T"）、行者（紫色方块 "W"）、
//! 弹丸（黄色小圆）全部占位渲染（UC-27）。贯穿 core / assets / gfx / audio / scene / devtools 全部模块：
//!
//! - 模拟只在 `BattleScene::fixed_update` 中运行，结果逐 tick 确定（UC-10、UC-13、UC-14）；
//! - 行者啃食动画的 `bite_hit` 事件帧才对炮塔扣血（UC-23）；
//! - 失败 / 胜利推入半透明覆盖场景，“Restart” 重开（UC-60、UC-61）；
//! - 发射 `blip`、命中 `hit`、循环 BGM（UC-50、UC-51）；F1 调试面板与 DebugDraw（UC-70 ～ UC-72）。

#![forbid(unsafe_code)]

mod render;
pub mod rules;
mod scenes;

pub use scenes::{BattleScene, OutcomeScene, PauseScene};

use trellia::prelude::*;

use rules::{LaneAssets, LaneState, Projectile, Turret, Walker};

/// 默认随机种子（与确定性测试一致）。
pub const DEFAULT_SEED: u64 = 42;

/// 示例插件：加载全部资产（失败记为启动错误，UC-32）。对应 UC-30、UC-31、UC-32。
pub struct LanePlugin;

impl Plugin for LanePlugin {
    fn name(&self) -> &'static str {
        "LanePlugin"
    }

    fn build(&self, app: &mut App) {
        match load(app.world_mut()) {
            Ok(assets) => {
                app.insert_resource(assets);
            }
            Err(e) => app.record_startup_error(e),
        }
        app.add_system(Stage::Render, title_system);
    }
}

fn load(world: &mut World) -> Result<LaneAssets, AssetError> {
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
    let anims =
        load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron").and_then(|atlas| {
            let walk = load_clip(
                &server,
                &atlases,
                &mut clips,
                atlas,
                "demo/walker_walk.anim.ron",
            )?;
            let bite = load_clip(
                &server,
                &atlases,
                &mut clips,
                atlas,
                "demo/walker_bite.anim.ron",
            )?;
            Ok((atlas, walk, bite))
        });
    world.resources.insert(images);
    world.resources.insert(atlases);
    world.resources.insert(clips);
    let (atlas, walk, bite) = anims?;

    let mut sound = |rel: &str| -> Result<Handle<AudioBytes>, AssetError> {
        let bytes = server.load_audio(rel)?;
        Ok(world
            .resources
            .expect_mut::<Assets<AudioBytes>>()
            .add_with_path(rel, bytes))
    };
    Ok(LaneAssets {
        atlas,
        walk,
        bite,
        blip: sound("demo/blip.ogg")?,
        hit: sound("demo/hit.ogg")?,
        music: sound("demo/loop.ogg")?,
    })
}

fn title_system(world: &mut World) {
    let frame = world.resources.expect::<Time>().frame;
    if frame.is_multiple_of(30)
        && let Some(status) = world
            .resources
            .get_mut::<trellia::egui_backend::WindowStatus>()
    {
        status.title = format!("lane_demo | {:.2} ms", status.cpu_frame_ms);
    }
}

/// 组装示例 App。插件顺序：GfxPlugin 先于 ScenePlugin（动画事件先于对局逻辑），
/// DevtoolsPlugin 在 GfxPlugin 之后。
#[must_use]
pub fn build_app(seed: u64) -> App {
    let mut app = App::new();
    app.add_plugin(AssetsPlugin::default())
        .add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))
        .add_plugin(AudioPlugin)
        .add_plugin(LanePlugin)
        .add_plugin(ScenePlugin::new(BattleScene { seed }))
        .add_plugin(DevtoolsPlugin::default());
    app
}

/// 登记参与确定性快照的组件：`Transform2D`（默认）、`Turret`、`Walker`、`Projectile`、`LaneState`。
pub fn register_snapshots(runner: &mut HeadlessRunner) {
    runner
        .register_snapshot::<Turret>()
        .register_snapshot::<Walker>()
        .register_snapshot::<Projectile>()
        .register_snapshot::<LaneState>();
}

/// 第 `cell` 格中心的逻辑坐标（测试脚本点击用）。
#[must_use]
pub fn cell_click_pos(cell: u8) -> Vec2 {
    rules::cell_center(cell)
}
