//! 在 `World` 上直接加载图集与动画片段（引擎报告 3.8 / 4 建议，游戏 spec E-2，ADR-008）。
//!
//! `load_atlas` / `load_clip` 需要同时可变借用多个 `Assets<T>`，游戏层原本要临时从 `Resources`
//! 取出再放回；这里把该样板封装一次。资源不存在时以空表代替，调用结束后一定放回。

use trellia_assets::{AssetError, AssetServer, Assets, Handle, Image};
use trellia_core::World;

use crate::{AnimationClip, Atlas, load_atlas, load_clip};

fn server(world: &World) -> Result<AssetServer, AssetError> {
    world
        .resources
        .get::<AssetServer>()
        .cloned()
        .ok_or_else(|| {
            AssetError::RootNotFound("AssetServer 资源不存在（未添加 AssetsPlugin）".into())
        })
}

/// 加载 `*.atlas.ron` 及其图像到 World 中的 `Assets<Image>` / `Assets<Atlas>`；同一路径重复加载返回已有句柄。
///
/// # Errors
///
/// World 中没有 `AssetServer` 时返回 [`AssetError::RootNotFound`]；其余同 [`load_atlas`]。
pub fn load_atlas_into(world: &mut World, rel_ron: &str) -> Result<Handle<Atlas>, AssetError> {
    let server = server(world)?;
    let mut images = world
        .resources
        .remove::<Assets<Image>>()
        .unwrap_or_default();
    let mut atlases = world
        .resources
        .remove::<Assets<Atlas>>()
        .unwrap_or_default();
    let result = load_atlas(&server, &mut images, &mut atlases, rel_ron);
    world.resources.insert(images);
    world.resources.insert(atlases);
    result
}

/// 加载 `*.anim.ron` 到 World 中的 `Assets<AnimationClip>`，帧名在 `atlas` 中解析；同一路径重复加载返回已有句柄。
///
/// # Errors
///
/// World 中没有 `AssetServer` 时返回 [`AssetError::RootNotFound`]；其余同 [`load_clip`]。
pub fn load_clip_into(
    world: &mut World,
    atlas: Handle<Atlas>,
    rel_ron: &str,
) -> Result<Handle<AnimationClip>, AssetError> {
    let server = server(world)?;
    let atlases = world
        .resources
        .remove::<Assets<Atlas>>()
        .unwrap_or_default();
    let mut clips = world
        .resources
        .remove::<Assets<AnimationClip>>()
        .unwrap_or_default();
    let result = load_clip(&server, &atlases, &mut clips, atlas, rel_ron);
    world.resources.insert(atlases);
    world.resources.insert(clips);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GfxPlugin;
    use trellia_assets::AssetsPlugin;
    use trellia_core::{App, Vec2};

    fn demo_app() -> App {
        let mut app = App::new();
        app.add_plugin(AssetsPlugin {
            root: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into()),
        })
        .add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)));
        app
    }

    #[test]
    fn load_into_world_restores_resources() {
        let mut app = demo_app();
        let world = app.world_mut();
        let atlas = load_atlas_into(world, "demo/walker.atlas.ron").expect("加载图集");
        let walk = load_clip_into(world, atlas, "demo/walker_walk.anim.ron").expect("加载片段");
        assert_eq!(
            load_atlas_into(world, "demo/walker.atlas.ron").expect("重复"),
            atlas
        );
        assert_eq!(
            load_clip_into(world, atlas, "demo/walker_walk.anim.ron").expect("重复"),
            walk
        );
        assert_eq!(world.resources.expect::<Assets<Image>>().len(), 1);
        assert_eq!(world.resources.expect::<Assets<Atlas>>().len(), 1);
        assert_eq!(world.resources.expect::<Assets<AnimationClip>>().len(), 1);
    }

    #[test]
    fn load_into_errors_keep_resources() {
        let mut app = demo_app();
        let world = app.world_mut();
        let err = load_atlas_into(world, "demo/missing.atlas.ron").expect_err("文件不存在");
        assert!(err.to_string().contains("missing.atlas.ron"), "{err}");
        assert!(world.resources.contains::<Assets<Image>>());
        assert!(world.resources.contains::<Assets<Atlas>>());

        let mut bare = App::new();
        let err =
            load_atlas_into(bare.world_mut(), "demo/walker.atlas.ron").expect_err("无 AssetServer");
        assert!(matches!(err, AssetError::RootNotFound(_)));
    }
}
