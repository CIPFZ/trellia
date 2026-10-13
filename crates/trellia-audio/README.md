# trellia-audio

音频请求队列 `AudioQueue` 与 `AudioBackend` trait。模拟层只发出播放意图、不触碰设备，
headless 下可断言“某音效被请求播放”（UC-50 ～ UC-53）。

## 最小示例

```rust
use trellia_assets::{Assets, AudioBytes, AudioKind};
use trellia_audio::{AudioBus, AudioQueue, AudioRequest};

let mut clips = Assets::<AudioBytes>::default();
let blip = clips.add(AudioBytes { bytes: Vec::new().into(), kind: AudioKind::Ogg });

let mut queue = AudioQueue::default();
queue.play_sfx(blip);                       // 同一音效可重叠播放
queue.set_bus_volume(AudioBus::Music, 0.5);
assert!(matches!(queue.peek()[0], AudioRequest::PlaySfx { .. }));
```

## 要点

- `AudioPlugin` 插入 `AudioQueue`；`Assets<AudioClip>` 已存在（`AssetsPlugin`）时不覆盖。
- 后端每帧 `drain`；未被取走的请求在下一帧 `Stage::First` 丢弃（首次 `warn`）。
- egui 后端用 `RodioBackend` 播放（同一 clip 最多 8 路）；无设备时降级为静默。
- `load_audio_into(world, rel)`：读取 OGG / WAV 到 World 中的 `Assets<AudioClip>`，按路径去重（ADR-008）。
