//! # trellia-scene
//!
//! 职责：场景栈（`SceneStack`）、场景生命周期（`Scene`）、覆盖层语义与帧末统一切换
//! （`SceneCommands`），以及供后端调用的 egui UI 回调表（`UiHooks`，ADR-006）。
//!
//! 对应用例：UC-60、UC-61、UC-62。
//!
//! 不负责：场景图父子变换层级、场景序列化、过场动画。

#![forbid(unsafe_code)]

use std::cell::RefCell;

/// egui 核心 crate 重新导出，`Scene::ui` 的实现者无需单独依赖 egui。
pub use egui;

use trellia_core::{App, AppExit, Plugin, Stage, World};

/// 一个场景（菜单、战斗、暂停覆盖层……）。对应 UC-60、UC-61。
///
/// 需要 `Send + Sync`，因为场景存放在 Resource 中（ADR-006）。
pub trait Scene: Send + Sync + 'static {
    /// 场景名（用于 `SceneStack::top_name` 与调试面板）。
    fn name(&self) -> &'static str;
    /// 入栈时调用。
    fn on_enter(&mut self, _world: &mut World) {}
    /// 出栈时调用。
    fn on_exit(&mut self, _world: &mut World) {}
    /// 上面被 push 了新场景时调用。
    fn on_pause(&mut self, _world: &mut World) {}
    /// 上面的场景被 pop、重新成为栈顶时调用。
    fn on_resume(&mut self, _world: &mut World) {}
    /// 每 tick（FixedUpdate）。
    fn fixed_update(&mut self, _world: &mut World) {}
    /// 每帧（Update）。
    fn update(&mut self, _world: &mut World) {}
    /// 向 `DrawList` 提交绘制（Render）。
    fn draw(&mut self, _world: &mut World) {}
    /// egui UI。headless 后端也会调用（用 `egui::Context::run_ui` 驱动，不渲染）。
    fn ui(&mut self, _world: &mut World, _ctx: &egui::Context) {}
    /// true：下方场景不执行 `fixed_update` / `update`（UC-61）。
    fn blocks_update_below(&self) -> bool {
        true
    }
    /// true：下方场景不执行 `draw` / `ui`。
    fn blocks_draw_below(&self) -> bool {
        false
    }
}

/// 场景切换命令。对应 UC-60、UC-62。
pub enum SceneCommand {
    /// 压入新场景。
    Push(Box<dyn Scene>),
    /// 弹出栈顶。
    Pop,
    /// 替换栈顶（旧场景 `on_exit`，新场景 `on_enter`，ADR-006）。
    Replace(Box<dyn Scene>),
    /// 弹出所有场景。
    Clear,
}

/// 资源：任意系统可发出场景命令，`PostUpdate` 中由 `ScenePlugin` 按入队顺序执行。
/// 对应 UC-62。
#[derive(Default)]
pub struct SceneCommands(Vec<SceneCommand>);

impl SceneCommands {
    /// 压入场景。
    pub fn push(&mut self, s: impl Scene) {
        self.0.push(SceneCommand::Push(Box::new(s)));
    }

    /// 弹出栈顶。
    pub fn pop(&mut self) {
        self.0.push(SceneCommand::Pop);
    }

    /// 替换栈顶。
    pub fn replace(&mut self, s: impl Scene) {
        self.0.push(SceneCommand::Replace(Box::new(s)));
    }

    /// 清空场景栈（随后 App 请求退出）。
    pub fn clear(&mut self) {
        self.0.push(SceneCommand::Clear);
    }
}

/// 资源：场景栈的只读视图。场景方法执行期间栈被临时取出，场景内无法访问它。
/// 对应 UC-60、UC-61。
#[derive(Default)]
pub struct SceneStack {
    scenes: Vec<Box<dyn Scene>>,
}

impl SceneStack {
    /// 栈深度。
    #[must_use]
    pub fn depth(&self) -> usize {
        self.scenes.len()
    }

    /// 栈顶场景名。
    #[must_use]
    pub fn top_name(&self) -> Option<&'static str> {
        self.scenes.last().map(|s| s.name())
    }

    /// 自底向上的场景名。
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        self.scenes.iter().map(|s| s.name()).collect()
    }

    /// 绘制起点：最上方一个 `blocks_draw_below` 场景的下标（含），否则 0。
    fn first_drawn(&self) -> usize {
        self.scenes
            .iter()
            .rposition(|s| s.blocks_draw_below())
            .unwrap_or(0)
    }
}

/// UI 回调：后端在每帧 `App::frame` 之后以 egui 上下文调用（ADR-006）。
pub type UiHook = fn(&mut World, &egui::Context);

/// 资源：后端每帧依次调用的 egui UI 回调表。`ScenePlugin` 注册场景 UI，
/// `DevtoolsPlugin` 注册调试面板。对应 UC-61、UC-70。
#[derive(Default)]
pub struct UiHooks(Vec<UiHook>);

impl UiHooks {
    /// 追加回调；按添加顺序调用。
    pub fn add(&mut self, hook: UiHook) {
        self.0.push(hook);
    }
}

/// 后端调用：依次执行全部 UI 回调。对应 UC-61、UC-70。
pub fn run_ui_hooks(world: &mut World, ctx: &egui::Context) {
    let hooks = world
        .resources
        .get::<UiHooks>()
        .map(|h| h.0.clone())
        .unwrap_or_default();
    for hook in hooks {
        hook(world, ctx);
    }
}

/// 向 App 的 `UiHooks` 追加回调（不存在时先插入）。对应 UC-61、UC-70。
pub fn add_ui_hook(app: &mut App, hook: UiHook) {
    if !app.world().resources.contains::<UiHooks>() {
        app.insert_resource(UiHooks::default());
    }
    app.world_mut().resources.expect_mut::<UiHooks>().add(hook);
}

/// 场景插件：插入 `SceneStack` / `SceneCommands`，注册场景驱动系统与 UI 回调。
/// 对应 UC-60、UC-61、UC-62。
///
/// 初始场景在 `Stage::Startup` 入栈；切换命令在本插件注册的 `PostUpdate` 系统中执行，
/// 之后注册的 `PostUpdate` 系统发出的命令顺延到下一帧（ADR-006）。
pub struct ScenePlugin {
    initial: RefCell<Option<Box<dyn Scene>>>,
}

impl ScenePlugin {
    /// 以初始场景创建（ADR-006）。
    #[must_use]
    pub fn new(initial: impl Scene) -> Self {
        Self {
            initial: RefCell::new(Some(Box::new(initial))),
        }
    }
}

impl Plugin for ScenePlugin {
    fn name(&self) -> &'static str {
        "ScenePlugin"
    }

    fn build(&self, app: &mut App) {
        let mut commands = SceneCommands::default();
        if let Some(initial) = self.initial.borrow_mut().take() {
            commands.0.push(SceneCommand::Push(initial));
        }
        app.insert_resource(SceneStack::default())
            .insert_resource(commands)
            .add_system(Stage::Startup, apply_scene_commands)
            .add_system(Stage::FixedUpdate, scenes_fixed_update)
            .add_system(Stage::Update, scenes_update)
            .add_system(Stage::PostUpdate, apply_scene_commands)
            .add_system(Stage::Render, scenes_draw);
        add_ui_hook(app, scenes_ui);
    }
}

/// 临时取出栈，调用 `f`，再放回。
fn with_stack(world: &mut World, f: impl FnOnce(&mut SceneStack, &mut World)) {
    let Some(mut stack) = world.resources.remove::<SceneStack>() else {
        return;
    };
    f(&mut stack, world);
    world.resources.insert(stack);
}

fn scenes_fixed_update(world: &mut World) {
    with_stack(world, |stack, world| {
        for scene in stack.scenes.iter_mut().rev() {
            scene.fixed_update(world);
            if scene.blocks_update_below() {
                break;
            }
        }
    });
}

fn scenes_update(world: &mut World) {
    with_stack(world, |stack, world| {
        for scene in stack.scenes.iter_mut().rev() {
            scene.update(world);
            if scene.blocks_update_below() {
                break;
            }
        }
    });
}

fn scenes_draw(world: &mut World) {
    with_stack(world, |stack, world| {
        let first = stack.first_drawn();
        for scene in &mut stack.scenes[first..] {
            scene.draw(world);
        }
    });
}

fn scenes_ui(world: &mut World, ctx: &egui::Context) {
    with_stack(world, |stack, world| {
        let first = stack.first_drawn();
        for scene in &mut stack.scenes[first..] {
            scene.ui(world, ctx);
        }
    });
}

fn apply_scene_commands(world: &mut World) {
    let cmds = std::mem::take(&mut world.resources.expect_mut::<SceneCommands>().0);
    if cmds.is_empty() {
        return;
    }
    with_stack(world, |stack, world| {
        // 执行期间新发出的命令留在资源中，下一帧处理
        for cmd in cmds {
            match cmd {
                SceneCommand::Push(mut scene) => {
                    if let Some(top) = stack.scenes.last_mut() {
                        top.on_pause(world);
                    }
                    scene.on_enter(world);
                    stack.scenes.push(scene);
                }
                SceneCommand::Pop => {
                    if let Some(mut top) = stack.scenes.pop() {
                        top.on_exit(world);
                        if let Some(below) = stack.scenes.last_mut() {
                            below.on_resume(world);
                        }
                    }
                }
                SceneCommand::Replace(mut scene) => {
                    if let Some(mut top) = stack.scenes.pop() {
                        top.on_exit(world);
                    }
                    scene.on_enter(world);
                    stack.scenes.push(scene);
                }
                SceneCommand::Clear => {
                    while let Some(mut top) = stack.scenes.pop() {
                        top.on_exit(world);
                    }
                }
            }
        }
        if stack.scenes.is_empty() {
            world.resources.expect_mut::<AppExit>().requested = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::FrameInput;

    /// 记录回调序列：(场景名, 回调名)。
    #[derive(Default)]
    struct Log(Vec<(&'static str, &'static str)>);

    struct Probe {
        name: &'static str,
        blocks_draw: bool,
        push_on_fixed: bool,
    }

    impl Probe {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                blocks_draw: false,
                push_on_fixed: false,
            }
        }
        fn log(&self, world: &mut World, what: &'static str) {
            world
                .resources
                .expect_mut::<Log>()
                .0
                .push((self.name, what));
        }
    }

    impl Scene for Probe {
        fn name(&self) -> &'static str {
            self.name
        }
        fn on_enter(&mut self, w: &mut World) {
            self.log(w, "enter");
        }
        fn on_exit(&mut self, w: &mut World) {
            self.log(w, "exit");
        }
        fn on_pause(&mut self, w: &mut World) {
            self.log(w, "pause");
        }
        fn on_resume(&mut self, w: &mut World) {
            self.log(w, "resume");
        }
        fn fixed_update(&mut self, w: &mut World) {
            self.log(w, "fixed");
            if self.push_on_fixed {
                self.push_on_fixed = false;
                w.resources
                    .expect_mut::<SceneCommands>()
                    .push(Probe::new("pushed"));
            }
        }
        fn update(&mut self, w: &mut World) {
            assert!(
                !w.resources.contains::<SceneStack>(),
                "场景方法执行期间 SceneStack 被临时取出"
            );
            self.log(w, "update");
        }
        fn draw(&mut self, w: &mut World) {
            self.log(w, "draw");
        }
        fn blocks_draw_below(&self) -> bool {
            self.blocks_draw
        }
    }

    #[derive(Default)]
    struct DepthSeen(Vec<usize>);

    fn record_depth(world: &mut World) {
        let depth = world.resources.expect::<SceneStack>().depth();
        world.resources.expect_mut::<DepthSeen>().0.push(depth);
    }

    fn app_with(initial: Probe) -> App {
        let mut app = App::new();
        app.insert_resource(Log::default())
            .add_plugin(ScenePlugin::new(initial));
        app
    }

    fn frame(app: &mut App) -> bool {
        app.frame(FrameInput {
            real_dt: 1.0 / 60.0,
            ..FrameInput::default()
        })
        .exit_requested
    }

    fn take_log(app: &mut App) -> Vec<(&'static str, &'static str)> {
        std::mem::take(&mut app.world_mut().resources.expect_mut::<Log>().0)
    }

    fn stack(app: &App) -> &SceneStack {
        app.world().resources.expect::<SceneStack>()
    }

    #[test]
    fn push_pauses_below() {
        let mut app = app_with(Probe::new("battle"));
        frame(&mut app);
        assert_eq!(
            take_log(&mut app),
            vec![
                ("battle", "enter"),
                ("battle", "fixed"),
                ("battle", "update"),
                ("battle", "draw")
            ]
        );
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .push(Probe::new("pause"));
        frame(&mut app);
        let log = take_log(&mut app);
        assert!(log.contains(&("battle", "pause")));
        assert!(log.contains(&("pause", "enter")));
        frame(&mut app);
        let log = take_log(&mut app);
        assert!(
            !log.contains(&("battle", "fixed")),
            "下方场景不再 fixed_update"
        );
        assert!(!log.contains(&("battle", "update")));
        assert!(log.contains(&("pause", "fixed")));
        assert_eq!(stack(&app).names(), vec!["battle", "pause"]);
    }

    #[test]
    fn overlay_still_draws_below() {
        let mut app = app_with(Probe::new("battle"));
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .push(Probe::new("pause"));
        frame(&mut app);
        take_log(&mut app);
        frame(&mut app);
        let draws: Vec<_> = take_log(&mut app)
            .into_iter()
            .filter(|(_, w)| *w == "draw")
            .collect();
        assert_eq!(
            draws,
            vec![("battle", "draw"), ("pause", "draw")],
            "自底向上绘制"
        );
    }

    #[test]
    fn blocks_draw_hides_below() {
        let mut app = app_with(Probe::new("battle"));
        let mut menu = Probe::new("menu");
        menu.blocks_draw = true;
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .push(menu);
        frame(&mut app);
        take_log(&mut app);
        frame(&mut app);
        let draws: Vec<_> = take_log(&mut app)
            .into_iter()
            .filter(|(_, w)| *w == "draw")
            .collect();
        assert_eq!(draws, vec![("menu", "draw")]);
    }

    #[test]
    fn commands_applied_at_frame_end() {
        let mut battle = Probe::new("battle");
        battle.push_on_fixed = true;
        let mut app = app_with(battle);
        app.insert_resource(DepthSeen::default())
            .add_system(Stage::Update, record_depth);
        frame(&mut app);
        // fixed_update 中 push：同帧 update 时 depth 未变，battle 仍收到 update
        assert!(take_log(&mut app).contains(&("battle", "update")));
        assert_eq!(app.world().resources.expect::<DepthSeen>().0, vec![1]);
        // 下一帧已变
        frame(&mut app);
        assert_eq!(app.world().resources.expect::<DepthSeen>().0, vec![1, 2]);
        assert_eq!(stack(&app).top_name(), Some("pushed"));
        assert!(!take_log(&mut app).contains(&("battle", "update")));
    }

    #[test]
    fn empty_stack_requests_exit() {
        let mut app = app_with(Probe::new("menu"));
        assert!(!frame(&mut app));
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .pop();
        assert!(frame(&mut app));
        assert_eq!(stack(&app).depth(), 0);
        assert!(take_log(&mut app).contains(&("menu", "exit")));
    }

    #[test]
    fn replace_and_clear() {
        let mut app = app_with(Probe::new("a"));
        frame(&mut app);
        take_log(&mut app);
        {
            let cmds = app.world_mut().resources.expect_mut::<SceneCommands>();
            cmds.push(Probe::new("b"));
            cmds.replace(Probe::new("c"));
        }
        frame(&mut app);
        let log: Vec<_> = take_log(&mut app)
            .into_iter()
            .filter(|(_, w)| !matches!(*w, "fixed" | "update" | "draw"))
            .collect();
        assert_eq!(
            log,
            vec![
                ("a", "pause"),
                ("b", "enter"),
                ("b", "exit"),
                ("c", "enter")
            ]
        );
        assert_eq!(stack(&app).names(), vec!["a", "c"]);
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .clear();
        assert!(frame(&mut app));
        let exits: Vec<_> = take_log(&mut app)
            .into_iter()
            .filter(|(_, w)| *w == "exit")
            .collect();
        assert_eq!(exits, vec![("c", "exit"), ("a", "exit")]);
    }

    #[test]
    fn ui_hooks_run_scene_ui_in_draw_order() {
        struct UiProbe(&'static str);
        impl Scene for UiProbe {
            fn name(&self) -> &'static str {
                self.0
            }
            fn ui(&mut self, w: &mut World, _ctx: &egui::Context) {
                w.resources.expect_mut::<Log>().0.push((self.0, "ui"));
            }
        }
        let mut app = App::new();
        app.insert_resource(Log::default())
            .add_plugin(ScenePlugin::new(UiProbe("a")));
        app.world_mut()
            .resources
            .expect_mut::<SceneCommands>()
            .push(UiProbe("b"));
        frame(&mut app);
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            run_ui_hooks(app.world_mut(), ui.ctx());
        });
        out.textures_delta.clear();
        assert_eq!(take_log(&mut app), vec![("a", "ui"), ("b", "ui")]);
    }
}
