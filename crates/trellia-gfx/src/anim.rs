//! 以 tick 推进的序列帧动画（SPEC.md 6.5 / 10.2）。

use serde::Deserialize;
use trellia_assets::{Asset, AssetError, AssetServer, Assets, Handle};
use trellia_core::{Entity, World};

use crate::{Atlas, Sprite};

/// 动画片段中的一帧。对应 UC-22、UC-23。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationFrame {
    /// 图集帧索引（加载时由帧名解析）。
    pub atlas_frame: usize,
    /// 持续 tick 数，≥ 1。
    pub duration_ticks: u32,
    /// 进入该帧时发送的事件名（UC-23）。
    pub event: Option<String>,
}

/// 动画片段，由 `*.anim.ron` 描述；时长以 tick 计，保证确定性。对应 UC-22、UC-23。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationClip {
    /// 片段名。
    pub name: String,
    /// 帧序列（非空）。
    pub frames: Vec<AnimationFrame>,
    /// 是否循环。
    pub looping: bool,
}

impl Asset for AnimationClip {}

/// 动画播放状态组件，需与 `Sprite` 同在一个实体上。对应 UC-22、UC-23。
///
/// 速度为整数分数 `speed_num / speed_den`（禁止浮点累加决定帧切换，SPEC.md 6.5）。
#[derive(Clone, Debug)]
pub struct Animator {
    /// 当前片段。
    pub clip: Handle<AnimationClip>,
    /// 当前帧在片段中的索引。
    pub frame_index: usize,
    /// 当前帧已停留的有效 tick 数。
    pub ticks_in_frame: u32,
    /// 速度分子，默认 1。
    pub speed_num: u32,
    /// 速度分母，默认 1；冰冻时可设为 1/2（UC-22）。
    pub speed_den: u32,
    /// 是否播放。
    pub playing: bool,
    /// 非循环片段播完置 true。
    pub finished: bool,
    speed_acc: u32,
    pending_entry: bool,
}

impl Animator {
    /// 从第 0 帧开始播放 `clip`，速度 1/1。
    #[must_use]
    pub fn new(clip: Handle<AnimationClip>) -> Self {
        Self {
            clip,
            frame_index: 0,
            ticks_in_frame: 0,
            speed_num: 1,
            speed_den: 1,
            playing: true,
            finished: false,
            speed_acc: 0,
            pending_entry: true,
        }
    }

    /// 切换片段并从第 0 帧开始；若 `clip` 与当前相同则不重置（避免攻击动画被重复打断）。
    pub fn play(&mut self, clip: Handle<AnimationClip>) {
        if self.clip != clip {
            self.clip = clip;
            self.restart();
        }
    }

    /// 从第 0 帧重新播放当前片段。
    pub fn restart(&mut self) {
        self.frame_index = 0;
        self.ticks_in_frame = 0;
        self.speed_acc = 0;
        self.playing = true;
        self.finished = false;
        self.pending_entry = true;
    }

    /// 设置速度 `num / den`；`den == 0` 视为 1（ADR-004）。
    pub fn set_speed(&mut self, num: u32, den: u32) {
        self.speed_num = num;
        self.speed_den = den.max(1);
        self.speed_acc = self.speed_acc.min(self.speed_den - 1);
    }
}

/// 动画事件：实体进入了带 `event` 的帧（UC-23）。由 `animation_system` 发送。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationEvent {
    /// 播放动画的实体。
    pub entity: Entity,
    /// 帧上声明的事件名。
    pub event: String,
}

/// `Stage::FixedUpdate`：按整数分数速度推进全部 `(Animator, Sprite)`，写入 `sprite.frame`，
/// 进入带事件的帧时发送 [`AnimationEvent`]（UC-22、UC-23）。
pub(crate) fn animation_system(world: &mut World) {
    let Some(clips) = world.resources.get::<Assets<AnimationClip>>() else {
        return;
    };
    for (entity, anim, sprite) in world
        .entities
        .query_mut::<(Entity, &mut Animator, &mut Sprite)>()
    {
        if !anim.playing || anim.finished {
            continue;
        }
        let Some(clip) = clips.get(anim.clip) else {
            continue;
        };
        if clip.frames.is_empty() {
            continue;
        }
        let mut enter = |index: usize, sprite: &mut Sprite| {
            let frame = &clip.frames[index];
            sprite.frame = frame.atlas_frame;
            if let Some(event) = &frame.event {
                world.events.send(AnimationEvent {
                    entity,
                    event: event.clone(),
                });
            }
        };
        if anim.pending_entry {
            anim.pending_entry = false;
            anim.frame_index = anim.frame_index.min(clip.frames.len() - 1);
            enter(anim.frame_index, sprite);
        }

        anim.speed_acc += anim.speed_num;
        let den = anim.speed_den.max(1);
        anim.ticks_in_frame += anim.speed_acc / den;
        anim.speed_acc %= den;

        loop {
            let duration = clip.frames[anim.frame_index].duration_ticks.max(1);
            if anim.ticks_in_frame < duration {
                break;
            }
            if anim.frame_index + 1 < clip.frames.len() {
                anim.ticks_in_frame -= duration;
                anim.frame_index += 1;
            } else if clip.looping {
                anim.ticks_in_frame -= duration;
                anim.frame_index = 0;
            } else {
                anim.ticks_in_frame = duration;
                anim.finished = true;
                break;
            }
            enter(anim.frame_index, sprite);
        }
    }
}

#[derive(Deserialize)]
#[serde(rename = "AnimationClip")]
struct AnimationClipDef {
    name: String,
    looping: bool,
    frames: Vec<AnimationFrameDef>,
}

#[derive(Deserialize)]
struct AnimationFrameDef {
    frame: String,
    ticks: u32,
    #[serde(default)]
    event: Option<String>,
}

/// 加载 `*.anim.ron`，把帧名按 `atlas` 解析为索引并校验（非空、帧名存在、`ticks ≥ 1`，UC-32）。
/// 同一路径重复加载返回已有句柄。
///
/// # Errors
///
/// 文件缺失、RON 解析失败或校验失败时返回 [`AssetError`]，信息包含文件路径。
pub fn load_clip(
    server: &AssetServer,
    atlases: &Assets<Atlas>,
    clips: &mut Assets<AnimationClip>,
    atlas: Handle<Atlas>,
    rel_ron: &str,
) -> Result<Handle<AnimationClip>, AssetError> {
    if let Some(handle) = clips.handle_by_path(rel_ron) {
        return Ok(handle);
    }
    let invalid = |reason: String| AssetError::Validation {
        path: server.root().join(rel_ron),
        reason,
    };
    let def: AnimationClipDef = server.load_ron(rel_ron)?;
    let atlas = atlases
        .get(atlas)
        .ok_or_else(|| invalid(format!("图集句柄无效: {atlas:?}")))?;
    if def.frames.is_empty() {
        return Err(invalid("帧列表为空".into()));
    }
    let mut frames = Vec::with_capacity(def.frames.len());
    for f in def.frames {
        if f.ticks == 0 {
            return Err(invalid(format!("帧 {} 的 ticks 必须 ≥ 1", f.frame)));
        }
        let atlas_frame = atlas
            .frame(&f.frame)
            .ok_or_else(|| invalid(format!("图集中不存在帧 {}", f.frame)))?;
        frames.push(AnimationFrame {
            atlas_frame,
            duration_ticks: f.ticks,
            event: f.event,
        });
    }
    Ok(clips.add_with_path(
        rel_ron,
        AnimationClip {
            name: def.name,
            frames,
            looping: def.looping,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtlasFrame, GfxPlugin, Rect};
    use std::collections::HashMap;
    use trellia_assets::Image;
    use trellia_core::{App, Vec2};

    /// 构造一个有 4 帧图集的 App，并以 `frames`（每项为 (时长, 事件)）创建片段。
    fn setup(frames: &[(u32, Option<&str>)], looping: bool) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)));
        let world = app.world_mut();
        let image = Assets::<Image>::default().add(Image {
            width: 4,
            height: 1,
            rgba: vec![0; 16],
        });
        let atlas = world.resources.expect_mut::<Assets<Atlas>>().add(Atlas {
            image,
            frames: (0..4)
                .map(|i| AtlasFrame {
                    name: format!("f{i}"),
                    src_px: Rect::from_min_size(Vec2::new(i as f32, 0.0), Vec2::ONE),
                    anchor: Vec2::new(0.5, 1.0),
                })
                .collect(),
            by_name: HashMap::new(),
        });
        let clip = world
            .resources
            .expect_mut::<Assets<AnimationClip>>()
            .add(AnimationClip {
                name: "test".into(),
                frames: frames
                    .iter()
                    .enumerate()
                    .map(|(i, (ticks, event))| AnimationFrame {
                        atlas_frame: i,
                        duration_ticks: *ticks,
                        event: event.map(str::to_owned),
                    })
                    .collect(),
                looping,
            });
        let e = world.spawn((Sprite::new(atlas), Animator::new(clip)));
        (app, e)
    }

    /// 逐 tick 记录 `Animator.frame_index`（第 i 项为第 i+1 tick 之后的值）。
    fn frame_trace(app: &mut App, e: Entity, ticks: u32) -> Vec<usize> {
        (0..ticks)
            .map(|_| {
                app.step_ticks(1);
                app.world()
                    .get::<Animator>(e)
                    .expect("实体存在")
                    .frame_index
            })
            .collect()
    }

    #[test]
    fn animator_advances_by_ticks() {
        let (mut app, e) = setup(&[(3, None), (2, None)], true);
        let trace = frame_trace(&mut app, e, 6);
        // 第 3 tick 切到帧 1，第 5 tick 回到帧 0
        assert_eq!(trace, vec![0, 0, 1, 1, 0, 0]);
        assert_eq!(app.world().get::<Sprite>(e).expect("实体存在").frame, 0);
    }

    #[test]
    fn animator_half_speed() {
        let (mut app, e) = setup(&[(3, None), (2, None)], true);
        app.world_mut()
            .get_mut::<Animator>(e)
            .expect("实体存在")
            .set_speed(1, 2);
        let trace = frame_trace(&mut app, e, 6);
        assert_eq!(trace, vec![0, 0, 0, 0, 0, 1], "第 6 tick 才切到帧 1");
        assert_eq!(app.world().get::<Sprite>(e).expect("实体存在").frame, 1);
    }

    #[test]
    fn animator_event_fires_once_per_entry() {
        let (mut app, e) = setup(&[(2, None), (3, Some("hit"))], true);
        let mut counts = Vec::new();
        for _ in 0..8 {
            app.step_ticks(1);
            // step_ticks 不翻页事件，故此处为累计数
            counts.push(app.world().events.get::<AnimationEvent>().iter().count());
        }
        // 第 2 tick 进入帧 1（发送），停留到第 5 tick；第 7 tick 再次进入
        assert_eq!(counts, vec![0, 1, 1, 1, 1, 1, 2, 2]);
        let first = app
            .world()
            .events
            .get::<AnimationEvent>()
            .iter()
            .next()
            .cloned();
        assert_eq!(
            first,
            Some(AnimationEvent {
                entity: e,
                event: "hit".into()
            })
        );
    }

    #[test]
    fn animator_non_loop_finishes() {
        let (mut app, e) = setup(&[(2, None), (2, None)], false);
        let trace = frame_trace(&mut app, e, 6);
        assert_eq!(trace, vec![0, 1, 1, 1, 1, 1]);
        let anim = (*app.world().get::<Animator>(e).expect("实体存在")).clone();
        assert!(anim.finished);
        assert_eq!(app.world().get::<Sprite>(e).expect("实体存在").frame, 1);
    }

    #[test]
    fn play_same_clip_does_not_reset() {
        let (mut app, e) = setup(&[(2, None), (2, None)], true);
        app.step_ticks(3);
        let mut anim = (*app.world().get::<Animator>(e).expect("实体存在")).clone();
        let before = (anim.frame_index, anim.ticks_in_frame);
        anim.play(anim.clip);
        assert_eq!((anim.frame_index, anim.ticks_in_frame), before);
        anim.restart();
        assert_eq!((anim.frame_index, anim.ticks_in_frame), (0, 0));
    }

    #[test]
    fn load_demo_clips() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
        let server = AssetServer::with_root(root);
        let mut images = Assets::default();
        let mut atlases = Assets::default();
        let mut clips = Assets::default();
        let atlas = crate::load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron")
            .expect("图集");
        let bite = load_clip(
            &server,
            &atlases,
            &mut clips,
            atlas,
            "demo/walker_bite.anim.ron",
        )
        .expect("bite 片段");
        let clip = clips.expect(bite);
        assert_eq!(clip.frames.len(), 3);
        assert_eq!(clip.frames[1].event.as_deref(), Some("bite_hit"));
        assert_eq!(clip.frames[1].atlas_frame, 3);
        load_clip(
            &server,
            &atlases,
            &mut clips,
            atlas,
            "demo/walker_walk.anim.ron",
        )
        .expect("walk 片段");
        assert_eq!(clips.len(), 2);
    }
}
