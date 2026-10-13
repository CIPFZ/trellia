//! 同步资产加载与解码（SPEC.md 5.2）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use trellia_core::{App, Plugin};

use crate::{Asset, Assets};

/// CPU 侧 RGBA8 图像。GPU 上传由后端在首次绘制时惰性完成。对应 UC-30。
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
    /// 行优先 RGBA8 像素，长度为 `width * height * 4`。
    pub rgba: Vec<u8>,
}

impl Asset for Image {}

/// 音频文件格式。对应 UC-30、UC-50、UC-51。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioKind {
    /// Ogg Vorbis。
    Ogg,
    /// WAV。
    Wav,
}

/// 未解码的音频原始字节，交给音频后端解码。对应 UC-30、UC-50、UC-51。
#[derive(Clone)]
pub struct AudioBytes {
    /// 文件原始字节（共享，克隆廉价）。
    pub bytes: Arc<[u8]>,
    /// 文件格式。
    pub kind: AudioKind,
}

impl Asset for AudioBytes {}

/// 资产加载错误，信息均包含完整文件路径（UC-32）。
#[derive(thiserror::Error, Debug)]
pub enum AssetError {
    /// 资产根目录不存在。
    #[error("资产根目录不存在: {0}")]
    RootNotFound(PathBuf),
    /// 文件读取失败。
    #[error("读取失败 {path}: {source}")]
    Io {
        /// 完整路径。
        path: PathBuf,
        /// 底层 IO 错误。
        source: std::io::Error,
    },
    /// RON 解析失败。
    #[error("RON 解析失败 {path}: {source}")]
    Ron {
        /// 完整路径。
        path: PathBuf,
        /// 带位置的解析错误（装箱以控制 `Result` 大小，ADR-004）。
        source: Box<ron::error::SpannedError>,
    },
    /// 图像解码失败。
    #[error("图像解码失败 {path}: {source}")]
    Image {
        /// 完整路径。
        path: PathBuf,
        /// 解码错误。
        source: image::ImageError,
    },
    /// 音频扩展名不是 `.ogg` / `.wav`。
    #[error("不支持的音频格式 {path}")]
    UnsupportedAudio {
        /// 完整路径。
        path: PathBuf,
    },
    /// 加载后校验失败（帧越界、帧名不存在等）。
    #[error("校验失败 {path}: {reason}")]
    Validation {
        /// 完整路径。
        path: PathBuf,
        /// 失败原因。
        reason: String,
    },
}

/// 同步资产加载器，所有路径相对资产根目录。对应 UC-30、UC-32。
#[derive(Clone, Debug)]
pub struct AssetServer {
    root: PathBuf,
}

impl AssetServer {
    /// 解析资产根目录：环境变量 `TRELLIA_ASSETS` → 可执行文件同级 `assets/` →
    /// `CARGO_MANIFEST_DIR/assets/` → `CARGO_MANIFEST_DIR` 各级祖先下的 `assets/`（ADR-004）。
    ///
    /// # Errors
    ///
    /// `TRELLIA_ASSETS` 指向不存在的目录，或所有候选目录都不存在时返回
    /// [`AssetError::RootNotFound`]。
    pub fn new() -> Result<Self, AssetError> {
        if let Some(dir) = std::env::var_os("TRELLIA_ASSETS") {
            let root = PathBuf::from(dir);
            return if root.is_dir() {
                Ok(Self::with_root(root))
            } else {
                Err(AssetError::RootNotFound(root))
            };
        }
        let mut candidates = Vec::new();
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
        {
            candidates.push(dir.join("assets"));
        }
        if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
            candidates.extend(Path::new(&manifest).ancestors().map(|d| d.join("assets")));
        }
        match candidates.iter().find(|p| p.is_dir()) {
            Some(root) => Ok(Self::with_root(root.clone())),
            None => Err(AssetError::RootNotFound(
                candidates.pop().unwrap_or_else(|| PathBuf::from("assets")),
            )),
        }
    }

    /// 以指定根目录创建（不检查目录是否存在）。
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 资产根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 读取文件全部字节。
    ///
    /// # Errors
    ///
    /// 读取失败时返回含完整路径的 [`AssetError::Io`]。
    pub fn read_bytes(&self, rel: &str) -> Result<Vec<u8>, AssetError> {
        let path = self.root.join(rel);
        std::fs::read(&path).map_err(|source| AssetError::Io { path, source })
    }

    /// 读取 UTF-8 文本。
    ///
    /// # Errors
    ///
    /// 读取失败或非 UTF-8 时返回含完整路径的 [`AssetError::Io`]。
    pub fn read_string(&self, rel: &str) -> Result<String, AssetError> {
        let path = self.root.join(rel);
        std::fs::read_to_string(&path).map_err(|source| AssetError::Io { path, source })
    }

    /// 通用 RON 加载，启用 `IMPLICIT_SOME` 扩展（ADR-004）。
    ///
    /// # Errors
    ///
    /// 读取失败返回 [`AssetError::Io`]，解析失败返回 [`AssetError::Ron`]。
    pub fn load_ron<T: DeserializeOwned>(&self, rel: &str) -> Result<T, AssetError> {
        let text = self.read_string(rel)?;
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(&text)
            .map_err(|source| AssetError::Ron {
                path: self.root.join(rel),
                source: Box::new(source),
            })
    }

    /// PNG → CPU 侧 RGBA8 图像。
    ///
    /// # Errors
    ///
    /// 读取失败返回 [`AssetError::Io`]，解码失败返回 [`AssetError::Image`]。
    pub fn load_image(&self, rel: &str) -> Result<Image, AssetError> {
        let bytes = self.read_bytes(rel)?;
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|source| AssetError::Image {
                path: self.root.join(rel),
                source,
            })?
            .to_rgba8();
        Ok(Image {
            width: decoded.width(),
            height: decoded.height(),
            rgba: decoded.into_raw(),
        })
    }

    /// 按扩展名读取 OGG / WAV 原始字节（不解码）。
    ///
    /// # Errors
    ///
    /// 扩展名不受支持返回 [`AssetError::UnsupportedAudio`]，读取失败返回 [`AssetError::Io`]。
    pub fn load_audio(&self, rel: &str) -> Result<AudioBytes, AssetError> {
        let path = self.root.join(rel);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        let kind = match ext.as_deref() {
            Some("ogg") => AudioKind::Ogg,
            Some("wav") => AudioKind::Wav,
            _ => return Err(AssetError::UnsupportedAudio { path }),
        };
        let bytes = self.read_bytes(rel)?;
        Ok(AudioBytes {
            bytes: bytes.into(),
            kind,
        })
    }
}

/// 插入 `AssetServer`、`Assets<Image>`、`Assets<AudioBytes>`。对应 UC-30、UC-32。
///
/// `root == None` 时按 [`AssetServer::new`] 的顺序解析根目录；解析失败记录为启动错误（ADR-004）。
#[derive(Clone, Debug, Default)]
pub struct AssetsPlugin {
    /// 显式资产根目录。
    pub root: Option<PathBuf>,
}

impl Plugin for AssetsPlugin {
    fn name(&self) -> &'static str {
        "AssetsPlugin"
    }

    fn build(&self, app: &mut App) {
        let server = match &self.root {
            Some(root) => Ok(AssetServer::with_root(root.clone())),
            None => AssetServer::new(),
        };
        match server {
            Ok(server) => {
                log::info!("资产根目录: {}", server.root().display());
                app.insert_resource(server);
            }
            Err(e) => app.record_startup_error(e),
        }
        app.insert_resource(Assets::<Image>::default())
            .insert_resource(Assets::<AudioBytes>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_server() -> AssetServer {
        AssetServer::with_root(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("trellia-assets-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    #[test]
    fn missing_file_error_contains_path() {
        let server = demo_server();
        let Err(err) = server.load_image("demo/does_not_exist.png") else {
            panic!("加载不存在的文件应失败");
        };
        let full = server.root().join("demo/does_not_exist.png");
        assert!(matches!(err, AssetError::Io { .. }));
        assert!(
            err.to_string().contains(&full.display().to_string()),
            "{err}"
        );
    }

    #[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
    struct Wave {
        name: String,
        spawns: Vec<(u32, u8)>,
        boss: Option<String>,
    }

    #[test]
    fn load_ron_roundtrip() {
        let root = temp_root("ron");
        let wave = Wave {
            name: "wave_1".into(),
            spawns: vec![(120, 0), (720, 1)],
            boss: Some("big".into()),
        };
        let text = ron::to_string(&wave).expect("序列化");
        std::fs::write(root.join("wave.ron"), text).expect("写入");
        std::fs::write(
            root.join("implicit.ron"),
            r#"(name: "w", spawns: [], boss: "b")"#,
        )
        .expect("写入");
        std::fs::write(root.join("broken.ron"), "(name: ").expect("写入");

        let server = AssetServer::with_root(&root);
        assert_eq!(server.load_ron::<Wave>("wave.ron").expect("解析"), wave);
        let implicit: Wave = server.load_ron("implicit.ron").expect("IMPLICIT_SOME");
        assert_eq!(implicit.boss.as_deref(), Some("b"));
        let err = server.load_ron::<Wave>("broken.ron").unwrap_err();
        assert!(matches!(err, AssetError::Ron { .. }));
        assert!(err.to_string().contains("broken.ron"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn load_png_dimensions() {
        let image = demo_server()
            .load_image("demo/placeholder_8x8.png")
            .expect("加载占位图");
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.rgba.len(), 256);
    }

    #[test]
    fn load_audio_by_extension() {
        let root = temp_root("audio");
        std::fs::write(root.join("a.wav"), b"RIFF").expect("写入");
        let server = AssetServer::with_root(&root);
        let audio = server.load_audio("a.wav").expect("读取");
        assert_eq!(audio.kind, AudioKind::Wav);
        assert_eq!(&*audio.bytes, b"RIFF");
        assert!(matches!(
            server.load_audio("a.mp3"),
            Err(AssetError::UnsupportedAudio { .. })
        ));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn plugin_inserts_resources() {
        let mut app = App::new();
        app.add_plugin(AssetsPlugin {
            root: Some(demo_server().root().to_path_buf()),
        });
        let res = &app.world().resources;
        assert!(res.contains::<AssetServer>());
        assert!(res.contains::<Assets<Image>>());
        assert!(res.contains::<Assets<AudioBytes>>());
        assert!(app.startup_errors().is_empty());
    }

    #[test]
    fn default_root_resolves_to_workspace_assets() {
        // cargo test 时 CARGO_MANIFEST_DIR = crates/trellia-assets，向上找到 workspace 根的 assets/
        if std::env::var_os("TRELLIA_ASSETS").is_none() {
            let server = AssetServer::new().expect("应找到 workspace assets/");
            assert!(server.root().join("demo/placeholder_8x8.png").is_file());
        }
    }
}
