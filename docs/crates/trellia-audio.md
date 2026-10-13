# trellia-audio 使用说明

## 发请求

```rust
use trellia_audio::AudioQueue;
fn on_shoot(world: &mut trellia_core::World, blip: trellia_assets::Handle<trellia_audio::AudioClip>) {
    world.resources.expect_mut::<AudioQueue>().play_sfx(blip);
}
```

`AudioRequest`：`PlaySfx { clip, volume }`、`PlayMusic { clip, looped }`、`StopMusic`、
`SetBusVolume { bus, volume }`。

## 测试

headless：`runner.take_audio_log()` 返回自上次调用以来的全部请求，可断言音效被请求的次数。

## 后端

egui 后端的 `RodioBackend`：音效每次一个 `Player`（同 clip ≤ 8 路），音乐单一常驻 `Player`；
实际音效音量 = 请求音量 × 音效总线音量。设备不可用时记录 `error` 并静默运行。

## 素材

`assets/demo/blip.ogg`、`hit.ogg`、`loop.ogg` 由 `scripts/gen_demo_audio.py` 合成（CC0）。
