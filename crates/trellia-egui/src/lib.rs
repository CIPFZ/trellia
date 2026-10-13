//! # trellia-egui
//!
//! 职责：基于 eframe 的窗口后端：采集输入并翻译为 `FrameInput`、驱动 `App::frame`、
//! 用 egui Painter 绘制 `DrawList`（letterbox）、调用 UI 回调（场景 UI、调试面板）、
//! 用 rodio 执行音频请求（`RodioBackend`）。
//!
//! 对应用例：UC-20、UC-24、UC-25、UC-40、UC-41、UC-42、UC-50、UC-51、UC-52、UC-70。
//!
//! 不负责：游戏逻辑、WASM / Web、手柄输入、自定义着色器。

#![forbid(unsafe_code)]

mod audio;
mod input_map;
mod paint;

pub use audio::RodioBackend;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, bail};
use trellia_assets::{Assets, Handle, Image};
use trellia_audio::{AudioBackend, AudioClip, AudioQueue};
use trellia_core::{App, Vec2};
use trellia_gfx::{DrawList, Viewport};

/// 窗口运行配置（SPEC.md 11.1，字段语义见 ADR-005）。对应 UC-20。
#[derive(Clone, Debug)]
pub struct RunConfig {
    /// 窗口标题（初始值；运行中可通过 [`WindowStatus`] 修改）。
    pub title: String,
    /// 设计分辨率；组装 App 时传给 `GfxPlugin::new`，`run` 会校验二者一致。
    pub logical_size: Vec2,
    /// 初始窗口尺寸（egui 逻辑点）。
    pub initial_window_px: (u32, u32),
    /// 是否开启垂直同步。
    pub vsync: bool,
    /// 资产根目录；组装 App 时传给 `AssetsPlugin`，后端不读取。
    pub asset_root: Option<PathBuf>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            title: "Trellia".into(),
            logical_size: Vec2::new(1280.0, 720.0),
            initial_window_px: (1280, 720),
            vsync: true,
            asset_root: None,
        }
    }
}

/// 资源（仅 egui 后端插入，ADR-005）：窗口标题与上一帧 CPU 耗时。对应 UC-70。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowStatus {
    /// 窗口标题；修改后后端在帧末更新。
    pub title: String,
    /// 最近 60 帧的平均 CPU 帧耗时（毫秒）：eframe 统计的整帧 CPU 时间
    /// （`App::frame`、DrawList 转换、UI、tessellation 与绘制提交，不含 vsync 等待）。
    pub cpu_frame_ms: f32,
}

/// 以 eframe 窗口运行 App，直到窗口关闭或 `AppExit` 被置位。对应 UC-20、UC-40、UC-42。
///
/// # Errors
///
/// App 记录了启动期错误、未添加 `GfxPlugin`、`logical_size` 不一致，或 eframe 初始化失败时返回错误。
pub fn run(mut app: App, cfg: RunConfig) -> anyhow::Result<()> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    if !app.startup_errors().is_empty() {
        bail!("启动失败:\n{}", app.startup_errors().join("\n"));
    }
    let Some(viewport) = app.world().resources.get::<Viewport>() else {
        bail!("App 未添加 GfxPlugin（需要 Viewport 与 DrawList 资源）");
    };
    if (viewport.logical_size - cfg.logical_size)
        .abs()
        .max_element()
        > 0.5
    {
        bail!(
            "RunConfig.logical_size {:?} 与 GfxPlugin 的 {:?} 不一致",
            cfg.logical_size,
            viewport.logical_size
        );
    }
    app.insert_resource(WindowStatus {
        title: cfg.title.clone(),
        cpu_frame_ms: 0.0,
    });

    let (w, h) = cfg.initial_window_px;
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(cfg.title.clone())
            .with_inner_size([w as f32, h as f32]),
        renderer: eframe::Renderer::Glow,
        ..eframe::NativeOptions::default()
    };
    options.glow_options.vsync = cfg.vsync;
    let title = cfg.title.clone();
    eframe::run_native(
        &cfg.title,
        options,
        Box::new(move |_cc| Ok(Box::new(EguiApp::new(app, title)))),
    )
    .map_err(|e| anyhow!("eframe 运行失败: {e}"))
}

struct EguiApp {
    app: App,
    audio: RodioBackend,
    textures: HashMap<Handle<Image>, egui::TextureHandle>,
    last_frame: Option<Instant>,
    shown_title: String,
    frame_ms: VecDeque<f32>,
}

impl EguiApp {
    fn new(app: App, title: String) -> Self {
        Self {
            app,
            audio: RodioBackend::new(),
            textures: HashMap::new(),
            last_frame: None,
            shown_title: title,
            frame_ms: VecDeque::with_capacity(60),
        }
    }

    /// 取走本帧音频请求交给 rodio。
    fn play_audio(&mut self) {
        let world = self.app.world_mut();
        let Some(queue) = world.resources.get_mut::<AudioQueue>() else {
            return;
        };
        let requests: Vec<_> = queue.drain().collect();
        if requests.is_empty() {
            return;
        }
        let empty = Assets::<AudioClip>::default();
        let clips = world.resources.get::<Assets<AudioClip>>().unwrap_or(&empty);
        for req in requests {
            self.audio.handle(req, clips);
        }
    }

    fn paint(&mut self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        let world = self.app.world_mut();
        let viewport = *world.resources.expect::<Viewport>();
        let Some(mut list) = world.resources.remove::<DrawList>() else {
            return;
        };
        let cmds = list.sorted();
        paint::upload_textures(
            ctx,
            cmds,
            world.resources.get::<Assets<Image>>(),
            &mut self.textures,
        );
        let map = paint::Mapper {
            viewport,
            pixels_per_point: ctx.pixels_per_point(),
        };
        let painter = ui.painter();
        // letterbox 区域与逻辑区域底色均为黑色；逻辑区域外的内容被裁剪
        painter.rect_filled(ctx.content_rect(), 0.0, egui::Color32::BLACK);
        let clip = egui::Rect::from_min_max(map.pos(Vec2::ZERO), map.pos(viewport.logical_size));
        paint::paint(&painter.with_clip_rect(clip), cmds, map, &self.textures);
        world.resources.insert(list);
    }
}

impl eframe::App for EguiApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let started = Instant::now();
        let real_dt = self
            .last_frame
            .map_or(0.0, |t| started.duration_since(t).as_secs_f32());
        self.last_frame = Some(started);
        let ctx = ui.ctx().clone();

        let input = input_map::collect_frame_input(&ctx, real_dt);
        let exit = self.app.frame(input).exit_requested;
        self.paint(ui);
        trellia_scene::run_ui_hooks(self.app.world_mut(), &ctx);
        self.play_audio();

        // 优先使用 eframe 统计的上一帧完整 CPU 耗时（含 tessellation 与绘制提交，不含 vsync 等待）
        let ms = frame
            .info()
            .cpu_usage
            .unwrap_or_else(|| started.elapsed().as_secs_f32())
            * 1000.0;
        if self.frame_ms.len() == 60 {
            self.frame_ms.pop_front();
        }
        self.frame_ms.push_back(ms);
        let avg = self.frame_ms.iter().sum::<f32>() / self.frame_ms.len() as f32;
        if let Some(status) = self.app.world_mut().resources.get_mut::<WindowStatus>() {
            status.cpu_frame_ms = avg;
            if status.title != self.shown_title {
                self.shown_title.clone_from(&status.title);
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(status.title.clone()));
            }
        }
        if exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_rejects_app_without_gfx() {
        let err = run(App::new(), RunConfig::default()).unwrap_err();
        assert!(err.to_string().contains("GfxPlugin"), "{err}");
    }

    #[test]
    fn run_rejects_startup_errors() {
        let mut app = App::new();
        app.record_startup_error("读取失败 /a/b.png");
        let err = run(app, RunConfig::default()).unwrap_err();
        assert!(err.to_string().contains("/a/b.png"), "{err}");
    }

    #[test]
    fn run_rejects_logical_size_mismatch() {
        let mut app = App::new();
        app.add_plugin(trellia_gfx::GfxPlugin::new(Vec2::new(640.0, 360.0)));
        let err = run(app, RunConfig::default()).unwrap_err();
        assert!(err.to_string().contains("logical_size"), "{err}");
    }
}
