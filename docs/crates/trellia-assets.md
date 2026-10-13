# trellia-assets 使用说明

## 资源

`AssetsPlugin` 插入：`AssetServer`、`Assets<Image>`、`Assets<AudioBytes>`。
其他资产类型（`Atlas`、`AnimationClip`）的容器由各自插件插入。

## 在插件中加载

```rust
use trellia_assets::{AssetServer, Assets, AudioBytes};
use trellia_core::{App, Plugin};

struct Sounds { blip: trellia_assets::Handle<AudioBytes> }
struct SoundPlugin;
impl Plugin for SoundPlugin {
    fn name(&self) -> &'static str { "SoundPlugin" }
    fn build(&self, app: &mut App) {
        let server = app.world().resources.expect::<AssetServer>().clone();
        match server.load_audio("demo/blip.ogg") {
            Ok(bytes) => {
                let blip = app.world_mut().resources.expect_mut::<Assets<AudioBytes>>()
                    .add_with_path("demo/blip.ogg", bytes);
                app.insert_resource(Sounds { blip });
            }
            Err(e) => app.record_startup_error(e), // 启动阶段终止，信息含完整路径
        }
    }
}
```

## 错误

`AssetError` 的每个变体都带完整路径：`RootNotFound`、`Io`、`Ron`、`Image`、
`UnsupportedAudio`、`Validation`。非测试代码中禁止对加载结果 `unwrap()`。

## 根目录

`TRELLIA_ASSETS` 优先；否则依次尝试可执行文件旁 `assets/`、`CARGO_MANIFEST_DIR/assets/`
及其祖先目录（使 `cargo run --example` 能找到 workspace 根的 `assets/`）。
