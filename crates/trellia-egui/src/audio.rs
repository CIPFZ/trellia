//! rodio 音频后端（SPEC.md 11.2，ADR-006；音乐交叉淡化 ADR-011）。

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::Source;
use trellia_assets::{Assets, Handle};
use trellia_audio::{AudioBackend, AudioBus, AudioClip, AudioRequest};
use trellia_core::Time;

/// 同一音效同时播放的最大路数。
const MAX_SAME_CLIP: usize = 8;

/// 音乐增益的更新周期（在音频线程中按样本数计时）。
const GAIN_PERIOD: Duration = Duration::from_millis(10);

/// 音量包络推进一个周期：向 `target` 线性移动 `period / fade` 个满量程，`fade` 为 0 时直接到达。
fn ramp(current: f32, target: f32, period: Duration, fade: Duration) -> f32 {
    if fade.is_zero() {
        return target;
    }
    let step = period.as_secs_f32() / fade.as_secs_f32();
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// `ticks` 个固定步长 tick 的时长。
fn ticks_to_duration(ticks: u32) -> Duration {
    Duration::from_secs_f32(ticks as f32 * Time::FIXED_DT)
}

/// 一条正在播放的背景音乐。`fade_out_ms > 0` 时音频线程把它淡出并在静音后自行停止。
struct MusicTrack {
    player: rodio::Player,
    fade_out_ms: Arc<AtomicU32>,
}

impl MusicTrack {
    fn fade_out(&self, fade: Duration) {
        let ms = u32::try_from(fade.as_millis()).unwrap_or(u32::MAX).max(1);
        self.fade_out_ms.store(ms, Ordering::Relaxed);
    }
}

/// 给音乐源加上可淡入 / 淡出的增益：从 0（`fade_in` 非零时）升到 1；收到淡出信号后降到 0 并停止。
fn with_fades<S>(source: S, fade_in: Duration, fade_out_ms: Arc<AtomicU32>) -> impl Source + Send
where
    S: Source + Send + 'static,
{
    let mut gain = if fade_in.is_zero() { 1.0 } else { 0.0 };
    source
        .amplify(gain)
        .stoppable()
        .periodic_access(GAIN_PERIOD, move |src| {
            let out = fade_out_ms.load(Ordering::Relaxed);
            if out > 0 {
                gain = ramp(
                    gain,
                    0.0,
                    GAIN_PERIOD,
                    Duration::from_millis(u64::from(out)),
                );
                if gain <= 0.0 {
                    src.stop();
                }
            } else {
                gain = ramp(gain, 1.0, GAIN_PERIOD, fade_in);
            }
            src.inner_mut().set_factor(gain);
        })
}

/// 基于 rodio 的音频后端。设备打开失败时记录 `log::error!` 并降级为静默。对应 UC-50、UC-51、UC-52。
pub struct RodioBackend {
    device: Option<rodio::MixerDeviceSink>,
    music: Option<MusicTrack>,
    /// 交叉淡化中正在淡出的旧音乐（静音后自行停止，下次处理请求时清理）。
    fading: Vec<MusicTrack>,
    sfx: Vec<(Handle<AudioClip>, f32, rodio::Player)>,
    sfx_volume: f32,
    music_volume: f32,
}

impl Default for RodioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RodioBackend {
    /// 打开默认音频输出设备；失败时返回静默后端。
    #[must_use]
    pub fn new() -> Self {
        let device = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut device) => {
                device.log_on_drop(false);
                Some(device)
            }
            Err(e) => {
                log::error!("音频设备初始化失败，降级为静默后端: {e}");
                None
            }
        };
        Self {
            device,
            music: None,
            fading: Vec::new(),
            sfx: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }

    /// 是否为静默后端（无音频设备）。
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.device.is_none()
    }
}

fn source(bytes: &Arc<[u8]>) -> Cursor<Arc<[u8]>> {
    Cursor::new(Arc::clone(bytes))
}

impl RodioBackend {
    /// 开始播放一条背景音乐（`fade_in` 为 0 时立即满音量）。解码失败返回 `None` 并警告。
    fn start_music(
        &self,
        device: &rodio::MixerDeviceSink,
        clip: Handle<AudioClip>,
        clips: &Assets<AudioClip>,
        looped: bool,
        fade_in: Duration,
    ) -> Option<MusicTrack> {
        let Some(audio) = clips.get(clip) else {
            log::warn!("音乐句柄无效: {clip:?}");
            return None;
        };
        let fade_out_ms = Arc::new(AtomicU32::new(0));
        let player = rodio::Player::connect_new(device.mixer());
        player.set_volume(self.music_volume);
        let appended = if looped {
            rodio::Decoder::new_looped(source(&audio.bytes))
                .map(|d| player.append(with_fades(d, fade_in, Arc::clone(&fade_out_ms))))
        } else {
            rodio::Decoder::new(source(&audio.bytes))
                .map(|d| player.append(with_fades(d, fade_in, Arc::clone(&fade_out_ms))))
        };
        match appended {
            Ok(()) => Some(MusicTrack {
                player,
                fade_out_ms,
            }),
            Err(e) => {
                log::warn!("音乐解码失败 {clip:?}: {e}");
                None
            }
        }
    }

    /// 立即停止全部背景音乐（含淡出中的）。
    fn stop_all_music(&mut self) {
        for track in self.music.take().into_iter().chain(self.fading.drain(..)) {
            track.player.stop();
        }
    }
}

impl AudioBackend for RodioBackend {
    fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>) {
        let Some(device) = &self.device else {
            return;
        };
        self.sfx.retain(|(_, _, player)| !player.empty());
        self.fading.retain(|track| !track.player.empty());
        match req {
            AudioRequest::PlaySfx { clip, volume } => {
                if self.sfx.iter().filter(|(c, _, _)| *c == clip).count() >= MAX_SAME_CLIP {
                    return;
                }
                let Some(audio) = clips.get(clip) else {
                    log::warn!("音效句柄无效: {clip:?}");
                    return;
                };
                match rodio::Decoder::new(source(&audio.bytes)) {
                    Ok(decoded) => {
                        let player = rodio::Player::connect_new(device.mixer());
                        player.set_volume(volume * self.sfx_volume);
                        player.append(decoded);
                        self.sfx.push((clip, volume, player));
                    }
                    Err(e) => log::warn!("音效解码失败 {clip:?}: {e}"),
                }
            }
            AudioRequest::PlayMusic { clip, looped } => {
                let next = self.start_music(device, clip, clips, looped, Duration::ZERO);
                self.stop_all_music();
                self.music = next;
            }
            AudioRequest::CrossfadeMusic { clip, ticks } => {
                let fade = ticks_to_duration(ticks);
                let next = self.start_music(device, clip, clips, true, fade);
                if next.is_none() {
                    // 新音乐不可用：保留当前音乐，不做淡化
                    return;
                }
                if let Some(old) = self.music.take() {
                    if fade.is_zero() {
                        old.player.stop();
                    } else {
                        old.fade_out(fade);
                        self.fading.push(old);
                    }
                }
                self.music = next;
            }
            AudioRequest::StopMusic => self.stop_all_music(),
            AudioRequest::SetBusVolume { bus, volume } => match bus {
                AudioBus::Sfx => {
                    self.sfx_volume = volume;
                    for (_, base, player) in &self.sfx {
                        player.set_volume(base * volume);
                    }
                }
                AudioBus::Music => {
                    self.music_volume = volume;
                    for track in self.music.iter().chain(&self.fading) {
                        track.player.set_volume(volume);
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_is_linear_and_clamped() {
        let period = Duration::from_millis(10);
        let fade = Duration::from_millis(40);
        let mut g = 0.0;
        let mut ups = Vec::new();
        for _ in 0..5 {
            g = ramp(g, 1.0, period, fade);
            ups.push(g);
        }
        assert_eq!(ups, vec![0.25, 0.5, 0.75, 1.0, 1.0]);
        assert!((ramp(1.0, 0.0, period, fade) - 0.75).abs() < 1e-6);
        assert!((ramp(0.1, 0.0, period, fade)).abs() < 1e-6);
        assert!((ramp(0.3, 1.0, period, Duration::ZERO) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ticks_convert_with_fixed_step() {
        assert_eq!(ticks_to_duration(60), Duration::from_secs(1));
        assert_eq!(ticks_to_duration(0), Duration::ZERO);
    }

    #[test]
    fn silent_backend_accepts_crossfade() {
        // 无设备时（CI）只验证请求被接受且不 panic；真实播放见 ADR-011 的人工核对
        let mut backend = RodioBackend {
            device: None,
            music: None,
            fading: Vec::new(),
            sfx: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
        };
        let mut clips = Assets::default();
        let h = clips.add(trellia_assets::AudioBytes {
            bytes: Vec::new().into(),
            kind: trellia_assets::AudioKind::Ogg,
        });
        backend.handle(AudioRequest::CrossfadeMusic { clip: h, ticks: 90 }, &clips);
        assert!(backend.music.is_none());
    }
}
