//! 图集与精灵（SPEC.md 6.4 / 10.1）。

use std::collections::HashMap;

use glam::Vec2;
use serde::Deserialize;
use trellia_assets::{Asset, AssetError, AssetServer, Assets, Handle, Image};
use trellia_core::{Time, Transform2D, World};

use crate::{Color, DrawList, Layer, Rect};

/// 图集中的一帧。对应 UC-22、UC-30。
#[derive(Clone, Debug, PartialEq)]
pub struct AtlasFrame {
    /// 帧名。
    pub name: String,
    /// 在图像中的像素矩形。
    pub src_px: Rect,
    /// 归一化锚点 (0..1)，(0.5, 1.0) 表示底部中心。
    pub anchor: Vec2,
}

/// 图集元数据，由 `*.atlas.ron` 描述。对应 UC-22、UC-30、UC-31。
#[derive(Clone, Debug)]
pub struct Atlas {
    /// 图集图像。
    pub image: Handle<Image>,
    /// 全部帧。
    pub frames: Vec<AtlasFrame>,
    /// 帧名 → 帧索引（只用于查找）。
    pub by_name: HashMap<String, usize>,
}

impl Asset for Atlas {}

impl Atlas {
    /// 按帧名查找帧索引。
    #[must_use]
    pub fn frame(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }
}

/// 精灵渲染组件，与 `Transform2D` 配合使用。对应 UC-21、UC-22、UC-24。
#[derive(Clone, Debug)]
pub struct Sprite {
    /// 图集。
    pub atlas: Handle<Atlas>,
    /// 当前帧索引（`Animator` 会写入）。
    pub frame: usize,
    /// 绘制层。
    pub layer: Layer,
    /// 着色，默认 `WHITE`（受伤闪白、冰冻变蓝，UC-24）。
    pub tint: Color,
    /// 水平翻转。
    pub flip_x: bool,
    /// 缩放，默认 1.0。
    pub scale: f32,
    /// 是否绘制，默认 true。
    pub visible: bool,
    /// 加到 `Transform2D.pos.y` 上作为 `sort_y`，默认 0。
    pub sort_y_offset: f32,
}

impl Sprite {
    /// 以默认值创建（ADR-004）：第 0 帧、`Layer::WORLD`、白色、不翻转、缩放 1、可见。
    #[must_use]
    pub fn new(atlas: Handle<Atlas>) -> Self {
        Self {
            atlas,
            frame: 0,
            layer: Layer::WORLD,
            tint: Color::WHITE,
            flip_x: false,
            scale: 1.0,
            visible: true,
            sort_y_offset: 0.0,
        }
    }
}

/// `Stage::Render`：把全部可见 `(Transform2D, Sprite)` 以插值位置提交到 `DrawList`（UC-21、UC-26）。
pub(crate) fn sprite_render_system(world: &mut World) {
    let Some(mut list) = world.resources.remove::<DrawList>() else {
        return;
    };
    let alpha = world.resources.expect::<Time>().alpha;
    if let Some(atlases) = world.resources.get::<Assets<Atlas>>() {
        for (transform, sprite) in world.entities.query::<(&Transform2D, &Sprite)>().iter() {
            if !sprite.visible {
                continue;
            }
            let Some(atlas) = atlases.get(sprite.atlas) else {
                continue;
            };
            let Some(frame) = atlas.frames.get(sprite.frame) else {
                continue;
            };
            let pos = transform.lerp(alpha);
            let size = frame.src_px.size() * sprite.scale;
            let dst = Rect::from_min_size(pos - frame.anchor * size, size);
            list.sprite(
                sprite.layer,
                pos.y + sprite.sort_y_offset,
                atlas.image,
                frame.src_px,
                dst,
            )
            .tint(sprite.tint)
            .flip_x(sprite.flip_x);
        }
    }
    world.resources.insert(list);
}

fn default_anchor() -> (f32, f32) {
    (0.5, 1.0)
}

#[derive(Deserialize)]
#[serde(rename = "Atlas")]
struct AtlasDef {
    image: String,
    frames: Vec<AtlasFrameDef>,
}

#[derive(Deserialize)]
struct AtlasFrameDef {
    name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    #[serde(default = "default_anchor")]
    anchor: (f32, f32),
}

/// 加载 `*.atlas.ron` 及其图像并校验（帧非空、帧名唯一、帧矩形不越界，UC-32）。
/// 同一路径重复加载返回已有句柄。
///
/// # Errors
///
/// 文件缺失、RON / PNG 解析失败或校验失败时返回 [`AssetError`]，信息包含文件路径。
pub fn load_atlas(
    server: &AssetServer,
    images: &mut Assets<Image>,
    atlases: &mut Assets<Atlas>,
    rel_ron: &str,
) -> Result<Handle<Atlas>, AssetError> {
    if let Some(handle) = atlases.handle_by_path(rel_ron) {
        return Ok(handle);
    }
    let invalid = |reason: String| AssetError::Validation {
        path: server.root().join(rel_ron),
        reason,
    };
    let def: AtlasDef = server.load_ron(rel_ron)?;
    let image = match images.handle_by_path(&def.image) {
        Some(handle) => handle,
        None => {
            let loaded = server.load_image(&def.image)?;
            images.add_with_path(&def.image, loaded)
        }
    };
    let (img_w, img_h) = images
        .get(image)
        .map(|img| (img.width, img.height))
        .ok_or_else(|| invalid(format!("图像句柄无效: {}", def.image)))?;
    if def.frames.is_empty() {
        return Err(invalid("帧列表为空".into()));
    }
    let mut frames = Vec::with_capacity(def.frames.len());
    let mut by_name = HashMap::new();
    for f in def.frames {
        if f.w == 0 || f.h == 0 {
            return Err(invalid(format!("帧 {} 尺寸为 0", f.name)));
        }
        if u64::from(f.x) + u64::from(f.w) > u64::from(img_w)
            || u64::from(f.y) + u64::from(f.h) > u64::from(img_h)
        {
            return Err(invalid(format!(
                "帧 {} 的矩形 ({}, {}, {}, {}) 超出图像 {} 范围 {img_w} × {img_h}",
                f.name, f.x, f.y, f.w, f.h, def.image
            )));
        }
        if by_name.insert(f.name.clone(), frames.len()).is_some() {
            return Err(invalid(format!("帧名重复: {}", f.name)));
        }
        frames.push(AtlasFrame {
            name: f.name,
            src_px: Rect::from_min_size(
                Vec2::new(f.x as f32, f.y as f32),
                Vec2::new(f.w as f32, f.h as f32),
            ),
            anchor: Vec2::new(f.anchor.0, f.anchor.1),
        });
    }
    Ok(atlases.add_with_path(
        rel_ron,
        Atlas {
            image,
            frames,
            by_name,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::App;

    fn demo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
    }

    #[test]
    fn load_demo_walker_atlas() {
        let server = AssetServer::with_root(demo_root());
        let mut images = Assets::default();
        let mut atlases = Assets::default();
        let h = load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron")
            .expect("加载 walker 图集");
        let again = load_atlas(&server, &mut images, &mut atlases, "demo/walker.atlas.ron")
            .expect("重复加载");
        assert_eq!(h, again);
        assert_eq!((images.len(), atlases.len()), (1, 1));
        let atlas = atlases.expect(h);
        assert_eq!(atlas.frames.len(), 4);
        let walk_1 = &atlas.frames[atlas.frame("walk_1").expect("walk_1 存在")];
        assert_eq!(
            walk_1.src_px,
            Rect::from_min_size(Vec2::new(64.0, 0.0), Vec2::new(64.0, 96.0))
        );
        assert_eq!(walk_1.anchor, Vec2::new(0.5, 1.0));
    }

    #[test]
    fn atlas_validation_rejects_out_of_bounds() {
        let root = std::env::temp_dir().join(format!("trellia-gfx-atlas-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("创建临时目录");
        std::fs::copy(
            demo_root().join("demo/placeholder_8x8.png"),
            root.join("p.png"),
        )
        .expect("复制占位图");
        std::fs::write(
            root.join("bad.atlas.ron"),
            r#"Atlas(image: "p.png", frames: [(name: "a", x: 4, y: 0, w: 8, h: 8)])"#,
        )
        .expect("写入");
        let server = AssetServer::with_root(&root);
        let err = load_atlas(
            &server,
            &mut Assets::default(),
            &mut Assets::default(),
            "bad.atlas.ron",
        )
        .unwrap_err();
        assert!(matches!(err, AssetError::Validation { .. }), "{err}");
        assert!(err.to_string().contains("bad.atlas.ron"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn sprite_render_uses_anchor_and_lerp() {
        let mut app = App::new();
        app.add_plugin(crate::GfxPlugin::new(Vec2::new(1280.0, 720.0)));
        let world = app.world_mut();
        let image = Assets::<Image>::default().add(Image {
            width: 64,
            height: 96,
            rgba: vec![0; 64 * 96 * 4],
        });
        let atlas = world.resources.expect_mut::<Assets<Atlas>>().add(Atlas {
            image,
            frames: vec![AtlasFrame {
                name: "f".into(),
                src_px: Rect::from_min_size(Vec2::ZERO, Vec2::new(64.0, 96.0)),
                anchor: Vec2::new(0.5, 1.0),
            }],
            by_name: HashMap::from([("f".to_owned(), 0)]),
        });
        world.spawn((
            Transform2D {
                pos: Vec2::new(200.0, 400.0),
                prev_pos: Vec2::new(100.0, 400.0),
            },
            Sprite::new(atlas),
        ));
        let mut hidden = Sprite::new(atlas);
        hidden.visible = false;
        world.spawn((Transform2D::new(Vec2::ZERO), hidden));
        world.resources.expect_mut::<Time>().alpha = 0.5;

        sprite_render_system(world);
        let cmds = world.resources.expect_mut::<DrawList>().sorted().to_vec();
        assert_eq!(cmds.len(), 1, "不可见精灵被跳过");
        let crate::DrawKind::Sprite { dst, .. } = &cmds[0].kind else {
            panic!("应为精灵");
        };
        assert!(
            (dst.center().x - 150.0).abs() < 1e-4,
            "中心 x 为 prev 与 pos 中点"
        );
        assert!((dst.max.y - 400.0).abs() < 1e-4, "锚点在底部");
        assert!((cmds[0].sort_y - 400.0).abs() < 1e-4);
    }
}
