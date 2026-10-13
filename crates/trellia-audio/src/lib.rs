//! # trellia-audio
//!
//! 职责：音频请求队列（`AudioQueue`）与音频后端 trait（`AudioBackend`）。
//! 模拟层只发出“播放意图”，不触碰音频设备；headless 下可断言“某音效被请求播放”。
//!
//! 对应用例：UC-50、UC-51、UC-52、UC-53。
//!
//! 不负责：音频设备、解码、音频总线 / 混音、空间音频。

#![forbid(unsafe_code)]

use trellia_assets::{AssetError, AssetServer, Assets, AudioBytes, Handle};
use trellia_core::{App, Plugin, Stage, World};

/// 音频片段：未解码的音频字节（re-export 自 `trellia-assets`）。对应 UC-50、UC-51。
pub type AudioClip = AudioBytes;

/// 音量总线。对应 UC-52。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioBus {
    /// 音效。
    Sfx,
    /// 背景音乐。
    Music,
}

/// 一条音频请求。对应 UC-50 ～ UC-53。
#[derive(Clone, Debug)]
pub enum AudioRequest {
    /// 播放一次性音效，同一 clip 可重叠播放（UC-50）。
    PlaySfx {
        /// 音效。
        clip: Handle<AudioClip>,
        /// 相对音量 0..1。
        volume: f32,
    },
    /// 替换当前背景音乐（UC-51）。
    PlayMusic {
        /// 音乐。
        clip: Handle<AudioClip>,
        /// 是否循环。
        looped: bool,
    },
    /// 停止背景音乐。
    StopMusic,
    /// 设置总线音量（UC-52）。
    SetBusVolume {
        /// 总线。
        bus: AudioBus,
        /// 音量 0..1。
        volume: f32,
    },
}

/// 资源：游戏层通过它发出音频请求，后端每帧 drain。对应 UC-50 ～ UC-53。
#[derive(Clone, Debug, Default)]
pub struct AudioQueue {
    requests: Vec<AudioRequest>,
    /// 上一帧 Render 结束时的请求数：后端本应在帧后取走它们。
    pending_at_frame_end: usize,
    warned_undrained: bool,
}

impl AudioQueue {
    /// 以音量 1.0 播放音效。
    pub fn play_sfx(&mut self, clip: Handle<AudioClip>) {
        self.play_sfx_with_volume(clip, 1.0);
    }

    /// 以指定相对音量播放音效。
    pub fn play_sfx_with_volume(&mut self, clip: Handle<AudioClip>, volume: f32) {
        self.requests.push(AudioRequest::PlaySfx { clip, volume });
    }

    /// 播放（替换）背景音乐。
    pub fn play_music(&mut self, clip: Handle<AudioClip>, looped: bool) {
        self.requests.push(AudioRequest::PlayMusic { clip, looped });
    }

    /// 停止背景音乐。
    pub fn stop_music(&mut self) {
        self.requests.push(AudioRequest::StopMusic);
    }

    /// 设置总线音量。
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32) {
        self.requests
            .push(AudioRequest::SetBusVolume { bus, volume });
    }

    /// 取走全部请求（后端调用）。
    pub fn drain(&mut self) -> std::vec::Drain<'_, AudioRequest> {
        self.requests.drain(..)
    }

    /// 查看尚未被取走的请求（headless 测试断言用，UC-53）。
    #[must_use]
    pub fn peek(&self) -> &[AudioRequest] {
        &self.requests
    }
}

/// 音频后端：执行请求。由后端 crate 实现。对应 UC-50 ～ UC-52。
pub trait AudioBackend {
    /// 执行一条请求；`clips` 用于解析句柄。
    fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>);
}

/// 音频插件：插入 `AudioQueue` 与（若不存在）`Assets<AudioClip>`，并在 `Stage::First`
/// 清空上一帧未被后端取走的请求（首次发生时 `log::warn!`，ADR-006）。对应 UC-50 ～ UC-53。
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn name(&self) -> &'static str {
        "AudioPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(AudioQueue::default());
        if !app.world().resources.contains::<Assets<AudioClip>>() {
            app.insert_resource(Assets::<AudioClip>::default());
        }
        app.add_system(Stage::First, clear_undrained)
            .add_system(Stage::Render, mark_frame_end);
    }
}

fn mark_frame_end(world: &mut World) {
    let queue = world.resources.expect_mut::<AudioQueue>();
    queue.pending_at_frame_end = queue.requests.len();
}

/// 只丢弃上一帧结束时已存在、却没被后端取走的请求；`Startup` 等在本帧 `First` 之前发出、
/// 后端尚未有机会处理的请求保留。
fn clear_undrained(world: &mut World) {
    let queue = world.resources.expect_mut::<AudioQueue>();
    let stale = queue.pending_at_frame_end.min(queue.requests.len());
    queue.pending_at_frame_end = 0;
    if stale == 0 {
        return;
    }
    if !queue.warned_undrained {
        queue.warned_undrained = true;
        log::warn!("AudioQueue 有 {stale} 条请求未被后端取走，已丢弃（此后不再提示）");
    }
    queue.requests.drain(..stale);
}

/// 按扩展名读取 OGG / WAV 到 World 中的 `Assets<AudioClip>`；同一路径重复加载返回已有句柄
/// （引擎报告 3.8 / 4 建议，游戏 spec E-2，ADR-008）。`Assets<AudioClip>` 不存在时先插入。
///
/// # Errors
///
/// World 中没有 `AssetServer` 时返回 [`AssetError::RootNotFound`]；其余同 `AssetServer::load_audio`。
pub fn load_audio_into(world: &mut World, rel: &str) -> Result<Handle<AudioClip>, AssetError> {
    let server = world
        .resources
        .get::<AssetServer>()
        .cloned()
        .ok_or_else(|| {
            AssetError::RootNotFound("AssetServer 资源不存在（未添加 AssetsPlugin）".into())
        })?;
    if let Some(handle) = world
        .resources
        .get::<Assets<AudioClip>>()
        .and_then(|clips| clips.handle_by_path(rel))
    {
        return Ok(handle);
    }
    let bytes = server.load_audio(rel)?;
    if !world.resources.contains::<Assets<AudioClip>>() {
        world.resources.insert(Assets::<AudioClip>::default());
    }
    Ok(world
        .resources
        .expect_mut::<Assets<AudioClip>>()
        .add_with_path(rel, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_assets::AudioKind;
    use trellia_core::FrameInput;

    fn clip() -> (Assets<AudioClip>, Handle<AudioClip>) {
        let mut clips = Assets::default();
        let h = clips.add(AudioBytes {
            bytes: Vec::new().into(),
            kind: AudioKind::Ogg,
        });
        (clips, h)
    }

    #[test]
    fn queue_records_and_drains_in_order() {
        let (_, h) = clip();
        let mut q = AudioQueue::default();
        q.play_sfx(h);
        q.play_sfx_with_volume(h, 0.5);
        q.play_music(h, true);
        q.set_bus_volume(AudioBus::Music, 0.3);
        q.stop_music();
        assert_eq!(q.peek().len(), 5);
        assert!(
            matches!(q.peek()[0], AudioRequest::PlaySfx { volume, .. } if (volume - 1.0).abs() < 1e-6)
        );
        let drained: Vec<_> = q.drain().collect();
        assert!(matches!(drained[4], AudioRequest::StopMusic));
        assert!(q.peek().is_empty());
    }

    #[test]
    fn plugin_keeps_loaded_clips_and_clears_undrained() {
        let (clips, h) = clip();
        let mut app = App::new();
        app.insert_resource(clips).add_plugin(AudioPlugin);
        assert_eq!(app.world().resources.expect::<Assets<AudioClip>>().len(), 1);
        app.world_mut()
            .resources
            .expect_mut::<AudioQueue>()
            .play_sfx(h);
        // 第一帧：后端尚未有机会处理，保留；第二帧开始时仍未被取走，丢弃
        app.frame(FrameInput::default());
        app.frame(FrameInput::default());
        assert!(
            app.world()
                .resources
                .expect::<AudioQueue>()
                .peek()
                .is_empty()
        );
    }

    fn request_on_startup(world: &mut World) {
        let clip = world
            .resources
            .expect::<Assets<AudioClip>>()
            .iter()
            .next()
            .map(|(h, _)| h);
        if let Some(clip) = clip {
            world
                .resources
                .expect_mut::<AudioQueue>()
                .play_music(clip, true);
        }
    }

    #[test]
    fn startup_requests_survive_first_frame() {
        let (clips, _) = clip();
        let mut app = App::new();
        app.insert_resource(clips)
            .add_plugin(AudioPlugin)
            .add_system(Stage::Startup, request_on_startup);
        app.frame(FrameInput::default());
        assert_eq!(
            app.world().resources.expect::<AudioQueue>().peek().len(),
            1,
            "后端尚未处理过的请求不应被清空"
        );
        app.frame(FrameInput::default());
        assert!(
            app.world()
                .resources
                .expect::<AudioQueue>()
                .peek()
                .is_empty(),
            "跨帧仍未被取走的请求被丢弃"
        );
    }

    struct Recorder(Vec<String>);
    impl AudioBackend for Recorder {
        fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>) {
            if let AudioRequest::PlaySfx { clip, .. } = req {
                self.0
                    .push(format!("{:?}", clips.get(clip).map(|c| c.kind)));
            }
        }
    }

    #[test]
    fn backend_trait_receives_requests() {
        let (clips, h) = clip();
        let mut q = AudioQueue::default();
        q.play_sfx(h);
        let mut backend = Recorder(Vec::new());
        for req in q.drain() {
            backend.handle(req, &clips);
        }
        assert_eq!(backend.0, vec!["Some(Ogg)"]);
    }

    #[test]
    fn load_audio_into_world_dedupes_by_path() {
        let mut app = App::new();
        app.add_plugin(trellia_assets::AssetsPlugin {
            root: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into()),
        })
        .add_plugin(AudioPlugin);
        let world = app.world_mut();
        let a = load_audio_into(world, "demo/blip.ogg").expect("加载音效");
        let b = load_audio_into(world, "demo/blip.ogg").expect("重复加载");
        assert_eq!(a, b);
        assert_eq!(world.resources.expect::<Assets<AudioClip>>().len(), 1);
        let err = load_audio_into(world, "demo/blip.mp3").expect_err("不支持的格式");
        assert!(matches!(err, AssetError::UnsupportedAudio { .. }));
        let mut bare = App::new();
        assert!(matches!(
            load_audio_into(bare.world_mut(), "demo/blip.ogg"),
            Err(AssetError::RootNotFound(_))
        ));
    }
}
