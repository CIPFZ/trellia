//! 应用骨架：App、Plugin、Stage 与帧流程（SPEC.md 3.4 / 4.1 / 4.2）。

use crate::{FrameInput, FrameOutput, GameRng, Input, Resource, Time, Transform2D, World};

/// 系统：对 World 的一次批处理。不做参数注入，系统内部按字段分别借用
/// `world.entities` / `world.resources` / `world.events`。对应 UC-03、UC-10。
pub type System = fn(&mut World);

/// 一帧内系统执行的阶段，按声明顺序执行。对应 UC-10、UC-11、UC-42、UC-62。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stage {
    /// 仅第一帧（或首次 `step_ticks`）执行一次。
    Startup,
    /// 事件翻页、输入快照之后。
    First,
    /// 资产后处理、场景 on_enter 等。
    PreUpdate,
    /// 每 tick 执行，每帧 0 ～ 5 次。
    FixedUpdate,
    /// 每帧一次：视觉补间、场景 update。
    Update,
    /// 每帧一次，之后应用延迟命令。
    PostUpdate,
    /// 生成绘制命令。
    Render,
}

const STAGE_COUNT: usize = 7;

/// 引擎子系统与游戏功能的注册单元（UC-03、UC-73）。
pub trait Plugin: 'static {
    /// 插件名；同名插件重复添加时 panic。
    fn name(&self) -> &'static str;
    /// 向 App 注册资源、事件与系统。
    fn build(&self, app: &mut App);
}

/// 资源：置 `requested = true` 请求退出程序（ADR-003）。一旦置位保持为 true。
/// 对应 UC-60、UC-62。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppExit {
    /// 是否已请求退出。
    pub requested: bool,
}

/// 单帧最多执行的 tick 数（防止“死亡螺旋”）。
const MAX_TICKS_PER_FRAME: u32 = 5;
/// 累加器与 `fixed_dt` 比较的显式阈值（秒），见 ADR-003。
const ACCUMULATOR_EPSILON: f32 = 1e-5;

/// 应用：持有 World 与各阶段系统，由后端每帧驱动。对应 UC-10、UC-11、UC-13、UC-73、UC-74。
pub struct App {
    world: World,
    systems: [Vec<System>; STAGE_COUNT],
    plugins: Vec<&'static str>,
    started: bool,
    accumulator: f32,
    output: FrameOutput,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// 创建 App，并插入引擎保证存在的资源：`Time`、`Input`、`GameRng`（种子 0）、`AppExit`。
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(Time::default());
        world.resources.insert(Input::default());
        world.resources.insert(GameRng::from_seed(0));
        world.resources.insert(AppExit::default());
        Self {
            world,
            systems: Default::default(),
            plugins: Vec::new(),
            started: false,
            accumulator: 0.0,
            output: FrameOutput::default(),
        }
    }

    /// 添加插件并立即调用其 `build`。
    ///
    /// # Panics
    ///
    /// 同名插件重复添加时 panic，信息包含插件名。
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        let name = plugin.name();
        assert!(!self.plugins.contains(&name), "插件重复添加: {name}");
        self.plugins.push(name);
        plugin.build(self);
        self
    }

    /// 在 `stage` 末尾追加系统；同一阶段内按注册顺序执行。
    pub fn add_system(&mut self, stage: Stage, system: System) -> &mut Self {
        self.systems[stage as usize].push(system);
        self
    }

    /// 插入（或替换）资源。
    pub fn insert_resource<R: Resource>(&mut self, res: R) -> &mut Self {
        self.world.resources.insert(res);
        self
    }

    /// 借用 World。
    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// 可变借用 World。
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 后端每帧调用一次。返回本帧产出，由后端消费（SPEC.md 3.4）。
    pub fn frame(&mut self, input: FrameInput) -> &FrameOutput {
        self.startup();

        // Stage::First：事件翻页、写入时间与输入快照
        self.world.events.swap_all();
        self.world.resources.expect_mut::<Time>().real_dt = input.real_dt;
        if self
            .world
            .resources
            .expect_mut::<Input>()
            .begin_frame(&input)
        {
            self.world.resources.expect_mut::<AppExit>().requested = true;
        }
        self.run_stage(Stage::First);
        self.run_stage(Stage::PreUpdate);
        self.run_fixed_update();
        self.run_stage(Stage::Update);
        self.run_stage(Stage::PostUpdate);
        self.world.apply_commands();
        self.run_stage(Stage::Render);

        self.world.resources.expect_mut::<Time>().frame += 1;
        self.output.exit_requested = self.world.resources.expect::<AppExit>().requested;
        &self.output
    }

    /// 仅推进 `n` 个固定 tick，不经过真实时间累加、不处理输入、无视 `paused`（headless 测试用）。
    pub fn step_ticks(&mut self, n: u32) {
        self.startup();
        for _ in 0..n {
            self.run_tick();
        }
    }

    fn startup(&mut self) {
        if !self.started {
            self.started = true;
            self.run_stage(Stage::Startup);
            self.world.apply_commands();
        }
    }

    fn run_stage(&mut self, stage: Stage) {
        for system in &self.systems[stage as usize] {
            system(&mut self.world);
        }
    }

    fn run_fixed_update(&mut self) {
        let time = self.world.resources.expect_mut::<Time>();
        let fixed_dt = time.fixed_dt;
        let ticks = if time.step_once {
            time.step_once = false;
            1
        } else if time.paused {
            0
        } else {
            self.accumulator += time.real_dt.max(0.0) * time.time_scale.clamp(0.0, 4.0);
            let mut n = 0;
            while n < MAX_TICKS_PER_FRAME && self.accumulator + ACCUMULATOR_EPSILON >= fixed_dt {
                self.accumulator = (self.accumulator - fixed_dt).max(0.0);
                n += 1;
            }
            if self.accumulator + ACCUMULATOR_EPSILON >= fixed_dt {
                // 超出上限的部分丢弃
                self.accumulator = 0.0;
            }
            n
        };
        for _ in 0..ticks {
            self.run_tick();
        }
        let alpha = (self.accumulator / fixed_dt).clamp(0.0, 1.0 - f32::EPSILON);
        self.world.resources.expect_mut::<Time>().alpha = alpha;
    }

    fn run_tick(&mut self) {
        for t in self.world.entities.query_mut::<&mut Transform2D>() {
            t.prev_pos = t.pos;
        }
        self.world.resources.expect_mut::<Time>().fixed_tick += 1;
        self.run_stage(Stage::FixedUpdate);
        self.world.apply_commands();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, InputEvent, Key, MouseButton, Timer, Vec2};

    fn frame_dt(app: &mut App, dt: f32) {
        app.frame(FrameInput {
            real_dt: dt,
            ..FrameInput::default()
        });
    }

    fn time(app: &App) -> Time {
        *app.world().resources.expect::<Time>()
    }

    #[test]
    fn fixed_step_accumulates() {
        let mut app = App::new();
        frame_dt(&mut app, 1.0 / 30.0);
        assert_eq!(time(&app).fixed_tick, 2);

        let mut app = App::new();
        frame_dt(&mut app, 1.0);
        assert_eq!(time(&app).fixed_tick, 5);
        // 超出部分被丢弃：下一帧不会补跑
        frame_dt(&mut app, 0.0);
        assert_eq!(time(&app).fixed_tick, 5);

        // 长时间运行不漂移
        let mut app = App::new();
        for _ in 0..1000 {
            frame_dt(&mut app, 1.0 / 30.0);
        }
        assert_eq!(time(&app).fixed_tick, 2000);
    }

    #[test]
    fn alpha_is_fraction_of_fixed_dt() {
        let mut app = App::new();
        frame_dt(&mut app, Time::FIXED_DT * 1.5);
        let t = time(&app);
        assert_eq!(t.fixed_tick, 1);
        assert!((t.alpha - 0.5).abs() < 1e-3);
    }

    #[test]
    fn time_scale_doubles_ticks() {
        let mut app = App::new();
        app.world_mut().resources.expect_mut::<Time>().time_scale = 2.0;
        frame_dt(&mut app, 1.0 / 60.0);
        assert_eq!(time(&app).fixed_tick, 2);
    }

    #[test]
    fn paused_skips_fixed_update() {
        let mut app = App::new();
        app.world_mut().resources.expect_mut::<Time>().paused = true;
        for _ in 0..3 {
            frame_dt(&mut app, 1.0 / 60.0);
        }
        let t = time(&app);
        assert_eq!(t.fixed_tick, 0);
        assert_eq!(t.frame, 3);
    }

    #[test]
    fn step_once_runs_exactly_one_tick() {
        let mut app = App::new();
        {
            let t = app.world_mut().resources.expect_mut::<Time>();
            t.paused = true;
            t.step_once = true;
        }
        frame_dt(&mut app, 1.0 / 60.0);
        assert_eq!(time(&app).fixed_tick, 1);
        assert!(!time(&app).step_once);
        frame_dt(&mut app, 1.0 / 60.0);
        assert_eq!(time(&app).fixed_tick, 1);
    }

    #[derive(Debug, Clone)]
    struct Ping;

    #[derive(Default)]
    struct PingLog {
        send_on_frame: u64,
        visible: Vec<usize>,
    }

    fn send_ping(world: &mut World) {
        let frame = world.resources.expect::<Time>().frame;
        if frame == world.resources.expect::<PingLog>().send_on_frame {
            world.events.send(Ping);
        }
    }

    fn record_pings(world: &mut World) {
        let n = world.events.get::<Ping>().iter().count();
        world.resources.expect_mut::<PingLog>().visible.push(n);
    }

    #[test]
    fn events_live_two_frames() {
        let mut app = App::new();
        app.world_mut().events.register::<Ping>();
        app.insert_resource(PingLog {
            send_on_frame: 2,
            visible: Vec::new(),
        })
        .add_system(Stage::Update, send_ping)
        .add_system(Stage::Render, record_pings);
        for _ in 0..5 {
            frame_dt(&mut app, 1.0 / 60.0);
        }
        // 帧 0、1 无事件；第 2 帧发送，第 2、3 帧可读；第 4 帧已丢弃
        assert_eq!(
            app.world().resources.expect::<PingLog>().visible,
            vec![0, 0, 1, 1, 0]
        );
    }

    #[derive(Default)]
    struct CountLog(Vec<usize>);

    fn spawn_one(world: &mut World) {
        world.commands.spawn((1_u32,));
        let n = world.entity_count();
        world.resources.expect_mut::<CountLog>().0.push(n);
    }

    fn observe_count(world: &mut World) {
        let n = world.entity_count();
        world.resources.expect_mut::<CountLog>().0.push(n);
    }

    #[test]
    fn commands_apply_at_tick_end() {
        let mut app = App::new();
        app.insert_resource(CountLog::default())
            .add_system(Stage::FixedUpdate, spawn_one)
            .add_system(Stage::FixedUpdate, observe_count);
        app.step_ticks(1);
        assert_eq!(app.world().resources.expect::<CountLog>().0, vec![0, 0]);
        assert_eq!(app.world().entity_count(), 1);
        app.step_ticks(1);
        assert_eq!(
            app.world().resources.expect::<CountLog>().0,
            vec![0, 0, 1, 1]
        );
        assert_eq!(app.world().entity_count(), 2);
    }

    struct LastPos(Option<Vec2>);
    struct PrevOk(bool);

    fn check_prev_then_move(world: &mut World) {
        let last = world.resources.expect::<LastPos>().0;
        let mut ok = true;
        let mut new_pos = None;
        for t in world.entities.query_mut::<&mut Transform2D>() {
            if let Some(last) = last {
                ok &= t.prev_pos == last;
            }
            t.pos += Vec2::new(1.0, 2.0);
            new_pos = Some(t.pos);
        }
        world.resources.expect_mut::<LastPos>().0 = new_pos;
        world.resources.expect_mut::<PrevOk>().0 &= ok;
    }

    #[test]
    fn transform_prev_synced_each_tick() {
        let mut app = App::new();
        app.insert_resource(LastPos(None))
            .insert_resource(PrevOk(true))
            .add_system(Stage::FixedUpdate, check_prev_then_move);
        let e = app.world_mut().spawn((Transform2D::new(Vec2::ZERO),));
        for _ in 0..10 {
            frame_dt(&mut app, 1.0 / 60.0);
        }
        assert!(app.world().resources.expect::<PrevOk>().0);
        let t = *app.world().get::<Transform2D>(e).expect("实体存在");
        assert_eq!(t.pos, Vec2::new(10.0, 20.0));
        assert_eq!(t.prev_pos, Vec2::new(9.0, 18.0));
    }

    #[test]
    fn input_just_pressed_single_frame() {
        let mut app = App::new();
        app.frame(FrameInput {
            real_dt: 1.0 / 60.0,
            window_size_px: (1280, 720),
            mouse_pos_px: Some((10.0, 20.0)),
            events: vec![
                InputEvent::MousePressed(MouseButton::Left),
                InputEvent::KeyPressed(Key::Space),
            ],
        });
        {
            let input = app.world().resources.expect::<Input>();
            assert!(input.mouse_just_pressed(MouseButton::Left));
            assert!(input.mouse_down(MouseButton::Left));
            assert!(input.key_just_pressed(Key::Space));
            assert!(!input.key_just_pressed(Key::Escape));
            assert_eq!(input.mouse_pos, Some(Vec2::new(10.0, 20.0)));
        }
        frame_dt(&mut app, 1.0 / 60.0);
        {
            let input = app.world().resources.expect::<Input>();
            assert!(!input.mouse_just_pressed(MouseButton::Left));
            assert!(input.mouse_down(MouseButton::Left));
            assert!(!input.key_just_pressed(Key::Space));
            assert!(input.key_down(Key::Space));
            assert_eq!(input.mouse_pos, None);
        }
        app.frame(FrameInput {
            real_dt: 1.0 / 60.0,
            events: vec![InputEvent::MouseReleased(MouseButton::Left)],
            ..FrameInput::default()
        });
        let input = app.world().resources.expect::<Input>();
        assert!(input.mouse_just_released(MouseButton::Left));
        assert!(!input.mouse_down(MouseButton::Left));
    }

    #[test]
    fn close_requested_sets_exit() {
        let mut app = App::new();
        assert!(!app.frame(FrameInput::default()).exit_requested);
        let out = app.frame(FrameInput {
            events: vec![InputEvent::CloseRequested],
            ..FrameInput::default()
        });
        assert!(out.exit_requested);
    }

    struct Named;
    impl Plugin for Named {
        fn name(&self) -> &'static str {
            "Named"
        }
        fn build(&self, _app: &mut App) {}
    }

    #[test]
    #[should_panic(expected = "Named")]
    fn duplicate_plugin_panics_with_name() {
        let mut app = App::new();
        app.add_plugin(Named).add_plugin(Named);
    }

    // ---- determinism_smoke：RNG + Timer + commands 的小型系统集合 ----

    #[derive(Clone, Copy)]
    struct Velocity(Vec2);
    #[derive(Clone, Copy)]
    struct Life(Timer);
    struct Spawner(Timer);

    fn spawner_system(world: &mut World) {
        if !world.resources.expect_mut::<Spawner>().0.tick() {
            return;
        }
        let rng = world.resources.expect_mut::<GameRng>();
        let count = rng.gen_range(1..4);
        for _ in 0..count {
            let pos = Vec2::new(rng.gen_range(0.0..1280.0), rng.gen_range(0.0..720.0));
            let vel = Vec2::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));
            let life = Timer::once(rng.gen_range(20..120));
            world
                .commands
                .spawn((Transform2D::new(pos), Velocity(vel), Life(life)));
        }
    }

    fn movement_system(world: &mut World) {
        for (t, v) in world.entities.query_mut::<(&mut Transform2D, &Velocity)>() {
            t.pos += v.0;
        }
    }

    fn lifetime_system(world: &mut World) {
        for (e, life) in world.entities.query_mut::<(Entity, &mut Life)>() {
            if life.0.tick() {
                world.commands.despawn(e);
            }
        }
    }

    fn world_snapshot(world: &World) -> Vec<u8> {
        let mut rows: Vec<(u64, Vec<u8>)> = world
            .query::<(Entity, &Transform2D, &Life)>()
            .iter()
            .map(|(e, t, l)| {
                let mut bytes = Vec::new();
                for f in [t.pos.x, t.pos.y, t.prev_pos.x, t.prev_pos.y] {
                    bytes.extend(f.to_le_bytes());
                }
                bytes.extend(l.0.remaining.to_le_bytes());
                (e.to_bits().get(), bytes)
            })
            .collect();
        rows.sort_by_key(|(id, _)| *id);
        let mut out = Vec::new();
        for (id, bytes) in rows {
            out.extend(id.to_le_bytes());
            out.extend(bytes);
        }
        out
    }

    fn run_smoke(seed: u64) -> (Vec<u8>, usize) {
        let mut app = App::new();
        app.insert_resource(GameRng::from_seed(seed))
            .insert_resource(Spawner(Timer::repeating(7)))
            .add_system(Stage::FixedUpdate, spawner_system)
            .add_system(Stage::FixedUpdate, movement_system)
            .add_system(Stage::FixedUpdate, lifetime_system);
        app.step_ticks(600);
        (world_snapshot(app.world()), app.world().entity_count())
    }

    #[test]
    fn determinism_smoke() {
        let (a, count) = run_smoke(7);
        let (b, _) = run_smoke(7);
        assert!(count > 0, "快照应包含实体");
        assert_eq!(a, b);
        let (c, _) = run_smoke(8);
        assert_ne!(a, c);
    }
}
