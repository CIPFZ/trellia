//! # trellia-devtools
//!
//! 职责：开发期 egui 调试面板（FPS、tick、实体数、DrawList 命令数、时间缩放、暂停、单步、
//! 场景栈）与调试形状绘制（`DebugDraw`）。发布构建可整体不链接（门面 feature `devtools`）。
//!
//! 对应用例：UC-70、UC-71、UC-72。
//!
//! 不负责：可视化编辑器、运行时反射、输入录制与回放。

#![forbid(unsafe_code)]

use std::collections::VecDeque;

use trellia_core::{App, Input, Key, Plugin, Stage, Time, Vec2, World};
use trellia_gfx::{Color, DrawKind, DrawList, Layer, Rect, TextAlign};
use trellia_scene::{SceneStack, add_ui_hook};

/// 资源：游戏层在任意阶段提交调试形状，`Stage::Render` 中被绘制到 `Layer::DEBUG` 后清空。
/// 默认关闭；关闭时提交为空操作（ADR-006）。对应 UC-72。
#[derive(Clone, Debug, Default)]
pub struct DebugDraw {
    shapes: Vec<DrawKind>,
    enabled: bool,
}

impl DebugDraw {
    /// 1 逻辑像素描边矩形（碰撞框、格子边界）。
    pub fn rect(&mut self, rect: Rect, color: Color) {
        if self.enabled {
            self.shapes.push(DrawKind::Rect {
                rect,
                color,
                stroke: Some(1.0),
            });
        }
    }

    /// 水平线（以 1 像素高的实心矩形实现）。
    pub fn line_h(&mut self, y: f32, x0: f32, x1: f32, color: Color) {
        if self.enabled {
            let rect = Rect::from_min_size(
                Vec2::new(x0.min(x1), y - 0.5),
                Vec2::new((x1 - x0).abs(), 1.0),
            );
            self.shapes.push(DrawKind::Rect {
                rect,
                color,
                stroke: None,
            });
        }
    }

    /// 14 号白色文字。
    pub fn text(&mut self, pos: Vec2, text: impl Into<String>) {
        if self.enabled {
            self.shapes.push(DrawKind::Text {
                pos,
                text: text.into(),
                size: 14.0,
                color: Color::WHITE,
                align: TextAlign::Left,
            });
        }
    }

    /// 是否开启。
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 开关（调试面板与测试使用）；关闭时丢弃已提交的形状。
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.shapes.clear();
        }
    }
}

/// 调试插件：`toggle_key`（默认 F1）开关面板。应在 `GfxPlugin` 之后添加。对应 UC-70、UC-71、UC-72。
#[derive(Clone, Copy, Debug)]
pub struct DevtoolsPlugin {
    /// 面板开关键。
    pub toggle_key: Key,
}

impl Default for DevtoolsPlugin {
    fn default() -> Self {
        Self {
            toggle_key: Key::F1,
        }
    }
}

/// 面板状态与统计（私有资源）。
struct DevtoolsState {
    open: bool,
    toggle_key: Key,
    real_dts: VecDeque<f32>,
    last_tick: u64,
    ticks_this_frame: u64,
}

impl Plugin for DevtoolsPlugin {
    fn name(&self) -> &'static str {
        "DevtoolsPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(DebugDraw::default())
            .insert_resource(DevtoolsState {
                open: false,
                toggle_key: self.toggle_key,
                real_dts: VecDeque::with_capacity(60),
                last_tick: 0,
                ticks_this_frame: 0,
            })
            .add_system(Stage::Update, toggle_system)
            .add_system(Stage::Render, debug_draw_system)
            .add_system(Stage::Render, stats_system);
        add_ui_hook(app, devtools_ui);
    }
}

fn toggle_system(world: &mut World) {
    let key = world.resources.expect::<DevtoolsState>().toggle_key;
    if world.resources.expect::<Input>().key_just_pressed(key) {
        let state = world.resources.expect_mut::<DevtoolsState>();
        state.open = !state.open;
    }
}

fn debug_draw_system(world: &mut World) {
    let Some(mut debug) = world.resources.remove::<DebugDraw>() else {
        return;
    };
    if let Some(list) = world.resources.get_mut::<DrawList>() {
        for shape in debug.shapes.drain(..) {
            match shape {
                DrawKind::Rect {
                    rect,
                    color,
                    stroke: Some(w),
                } => list.rect_stroke(Layer::DEBUG, rect, color, w),
                DrawKind::Rect { rect, color, .. } => list.rect(Layer::DEBUG, rect, color),
                DrawKind::Text {
                    pos,
                    text,
                    size,
                    color,
                    align,
                } => list.text_aligned(Layer::DEBUG, pos, text, size, color, align),
                DrawKind::Circle { .. } | DrawKind::Sprite { .. } => {}
            }
        }
    }
    debug.shapes.clear();
    world.resources.insert(debug);
}

fn stats_system(world: &mut World) {
    let time = *world.resources.expect::<Time>();
    let state = world.resources.expect_mut::<DevtoolsState>();
    if state.real_dts.len() == 60 {
        state.real_dts.pop_front();
    }
    state.real_dts.push_back(time.real_dt);
    state.ticks_this_frame = time.fixed_tick - state.last_tick;
    state.last_tick = time.fixed_tick;
}

fn devtools_ui(world: &mut World, ctx: &egui::Context) {
    let Some(state) = world.resources.get::<DevtoolsState>() else {
        return;
    };
    if !state.open {
        return;
    }
    let avg_dt = state.real_dts.iter().sum::<f32>() / state.real_dts.len().max(1) as f32;
    let ticks = state.ticks_this_frame;
    let mut time = *world.resources.expect::<Time>();
    let entities = world.entity_count();
    let draw_cmds = world.resources.get::<DrawList>().map_or(0, DrawList::len);
    let scenes = world
        .resources
        .get::<SceneStack>()
        .map(SceneStack::names)
        .unwrap_or_default();
    let mut debug_on = world
        .resources
        .get::<DebugDraw>()
        .is_some_and(DebugDraw::enabled);

    egui::Window::new("Trellia Devtools")
        .default_pos([8.0, 8.0])
        .resizable(false)
        .show(ctx, |ui| {
            let fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
            ui.label(format!("FPS {fps:.1}   frame {:.2} ms", avg_dt * 1000.0));
            ui.label(format!("tick {}   ticks/frame {ticks}", time.fixed_tick));
            ui.label(format!("entities {entities}   draw cmds {draw_cmds}"));
            ui.add(egui::Slider::new(&mut time.time_scale, 0.0..=4.0).text("time scale"));
            ui.horizontal(|ui| {
                if ui
                    .button(if time.paused { "Resume" } else { "Pause" })
                    .clicked()
                {
                    time.paused = !time.paused;
                }
                if ui.button("Step").clicked() {
                    time.step_once = true;
                }
            });
            ui.checkbox(&mut debug_on, "DebugDraw");
            ui.label(format!("scenes: {}", scenes.join(" > ")));
        });

    let t = world.resources.expect_mut::<Time>();
    t.time_scale = time.time_scale;
    t.paused = time.paused;
    t.step_once = time.step_once;
    if let Some(debug) = world.resources.get_mut::<DebugDraw>()
        && debug.enabled() != debug_on
    {
        debug.set_enabled(debug_on);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::{FrameInput, InputEvent};
    use trellia_gfx::GfxPlugin;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugin(GfxPlugin::new(Vec2::new(1280.0, 720.0)))
            .add_plugin(DevtoolsPlugin::default());
        app
    }

    fn push_debug(world: &mut World) {
        let debug = world.resources.expect_mut::<DebugDraw>();
        debug.rect(
            Rect::from_min_size(Vec2::ZERO, Vec2::splat(10.0)),
            Color::WHITE,
        );
        debug.line_h(5.0, 20.0, 0.0, Color::WHITE);
        debug.text(Vec2::ZERO, "dbg");
    }

    fn debug_cmds(app: &mut App) -> usize {
        app.world_mut()
            .resources
            .expect_mut::<DrawList>()
            .sorted()
            .iter()
            .filter(|c| c.layer == Layer::DEBUG)
            .count()
    }

    #[test]
    fn debug_draw_only_when_enabled() {
        let mut app = app();
        app.add_system(Stage::Update, push_debug);
        app.frame(FrameInput::default());
        assert_eq!(debug_cmds(&mut app), 0, "默认关闭");
        app.world_mut()
            .resources
            .expect_mut::<DebugDraw>()
            .set_enabled(true);
        app.frame(FrameInput::default());
        assert_eq!(debug_cmds(&mut app), 3);
        app.frame(FrameInput::default());
        assert_eq!(debug_cmds(&mut app), 3, "每帧清空后重新提交");
    }

    #[test]
    fn toggle_key_opens_panel_and_counts_ticks() {
        let mut app = app();
        app.frame(FrameInput {
            real_dt: 1.0 / 30.0,
            events: vec![InputEvent::KeyPressed(Key::F1)],
            ..FrameInput::default()
        });
        let state = app.world().resources.expect::<DevtoolsState>();
        assert!(state.open);
        assert_eq!(state.ticks_this_frame, 2);

        // 面板在 headless egui 上下文中可运行
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            trellia_scene::run_ui_hooks(app.world_mut(), ui.ctx());
        });
        out.textures_delta.clear();
    }
}
