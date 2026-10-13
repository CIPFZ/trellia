# trellia-assets

资产句柄 `Handle<T>`、容器 `Assets<T>` 与同步加载器 `AssetServer`（RON / PNG / 音频字节），
加载失败在启动阶段报错并给出完整路径（UC-30 ～ UC-32）。

## 最小示例

```rust
use trellia_assets::{AssetServer, Assets, Image};

fn load(server: &AssetServer, images: &mut Assets<Image>) -> Result<(), trellia_assets::AssetError> {
    let img = server.load_image("demo/placeholder_8x8.png")?;      // 错误信息含完整路径
    let h = images.add_with_path("demo/placeholder_8x8.png", img); // 同路径去重
    assert_eq!(images.expect(h).width, 8);
    Ok(())
}
```

## 要点

- 根目录：`TRELLIA_ASSETS` → 可执行文件旁 `assets/` → `CARGO_MANIFEST_DIR` 及其祖先下的 `assets/`（ADR-004）。
- `AssetsPlugin` 插入 `AssetServer`、`Assets<Image>`、`Assets<AudioBytes>`；根目录解析失败记为启动错误。
- 在自己的 `Plugin::build` 中加载资产，失败时 `app.record_startup_error(e)`，后端在第一帧前终止。
- `load_ron` 启用 `IMPLICIT_SOME`：`Option` 字段可直接写值。
- 句柄不做代际校验，v1 资产不卸载。
