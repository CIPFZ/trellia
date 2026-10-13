//! rodio 音频后端（SPEC.md 11.2，ADR-006）。

use std::io::Cursor;
use std::sync::Arc;

use trellia_assets::{Assets, Handle};
use trellia_audio::{AudioBackend, AudioBus, AudioClip, AudioRequest};

/// 同一音效同时播放的最大路数。
const MAX_SAME_CLIP: usize = 8;

/// 基于 rodio 的音频后端。设备打开失败时记录 `log::error!` 并降级为静默。对应 UC-50、UC-51、UC-52。
pub struct RodioBackend {
    device: Option<rodio::MixerDeviceSink>,
    music: Option<rodio::Player>,
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

impl AudioBackend for RodioBackend {
    fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>) {
        let Some(device) = &self.device else {
            return;
        };
        self.sfx.retain(|(_, _, player)| !player.empty());
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
                if let Some(old) = self.music.take() {
                    old.stop();
                }
                let Some(audio) = clips.get(clip) else {
                    log::warn!("音乐句柄无效: {clip:?}");
                    return;
                };
                let player = rodio::Player::connect_new(device.mixer());
                player.set_volume(self.music_volume);
                let appended = if looped {
                    rodio::Decoder::new_looped(source(&audio.bytes)).map(|d| player.append(d))
                } else {
                    rodio::Decoder::new(source(&audio.bytes)).map(|d| player.append(d))
                };
                match appended {
                    Ok(()) => self.music = Some(player),
                    Err(e) => log::warn!("音乐解码失败 {clip:?}: {e}"),
                }
            }
            AudioRequest::StopMusic => {
                if let Some(old) = self.music.take() {
                    old.stop();
                }
            }
            AudioRequest::SetBusVolume { bus, volume } => match bus {
                AudioBus::Sfx => {
                    self.sfx_volume = volume;
                    for (_, base, player) in &self.sfx {
                        player.set_volume(base * volume);
                    }
                }
                AudioBus::Music => {
                    self.music_volume = volume;
                    if let Some(player) = &self.music {
                        player.set_volume(volume);
                    }
                }
            },
        }
    }
}
