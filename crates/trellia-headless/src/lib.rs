//! # trellia-headless
//!
//! 职责：无窗口后端 `HeadlessRunner`：以固定 `real_dt` 驱动 `App`、注入输入、
//! 记录排序后的 `DrawList` 与音频请求、生成可逐字节比较的 World 快照。
//!
//! 对应用例：UC-13、UC-53、UC-73、UC-74。
//!
//! 不负责：实际渲染与音频播放、输入录制与回放 UI。
//!
//! ```
//! use trellia_core::{App, Transform2D, Vec2};
//! use trellia_headless::HeadlessRunner;
//!
//! let mut app = App::new();
//! app.world_mut().spawn((Transform2D::new(Vec2::new(1.0, 2.0)),));
//! let mut runner = HeadlessRunner::new(app);
//! runner.frames(10);
//! assert!(!runner.snapshot().is_empty());
//! ```

#![forbid(unsafe_code)]

use std::any::TypeId;

use trellia_core::{App, Component, Entity, FrameInput, FrameOutput, Time, Transform2D, World};

/// 参与确定性快照的组件（UC-13）。游戏层通常用 serde 或手写字节实现。
///
/// 实现必须只依赖组件自身的确定性状态，且写出的字节与平台无关（例如使用 `to_le_bytes`）。
pub trait Snapshot {
    /// 把组件序列化追加到 `out`。
    fn write(&self, out: &mut Vec<u8>);
}

impl Snapshot for Transform2D {
    fn write(&self, out: &mut Vec<u8>) {
        for f in [self.pos.x, self.pos.y, self.prev_pos.x, self.prev_pos.y] {
            out.extend(f.to_le_bytes());
        }
    }
}

type SnapshotWriter = fn(&World, &mut Vec<u8>);

fn write_component<T: Component + Snapshot>(world: &World, out: &mut Vec<u8>) {
    let mut rows: Vec<(u64, Vec<u8>)> = world
        .query::<(Entity, &T)>()
        .iter()
        .map(|(e, c)| {
            let mut bytes = Vec::new();
            c.write(&mut bytes);
            (e.to_bits().get(), bytes)
        })
        .collect();
    rows.sort_by_key(|(id, _)| *id);
    let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    out.extend(count.to_le_bytes());
    for (id, bytes) in rows {
        out.extend(id.to_le_bytes());
        out.extend(bytes);
    }
}

/// 无窗口后端：以固定 `real_dt = 1/60` 驱动 App，供单元 / 集成测试使用。
/// 对应 UC-13、UC-73、UC-74。
pub struct HeadlessRunner {
    app: App,
    window_size_px: (u32, u32),
    snapshot_writers: Vec<(TypeId, SnapshotWriter)>,
}

impl HeadlessRunner {
    /// 默认虚拟窗口尺寸（与 SPEC.md 设计分辨率一致，物理像素 = 逻辑坐标）。
    pub const DEFAULT_WINDOW_PX: (u32, u32) = (1280, 720);

    /// 包装 App。`Transform2D` 默认已登记快照。
    #[must_use]
    pub fn new(app: App) -> Self {
        let mut runner = Self {
            app,
            window_size_px: Self::DEFAULT_WINDOW_PX,
            snapshot_writers: Vec::new(),
        };
        runner.register_snapshot::<Transform2D>();
        runner
    }

    /// 借用 App。
    #[must_use]
    pub fn app(&self) -> &App {
        &self.app
    }

    /// 可变借用 App。
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// 以固定 `real_dt = 1/60` 推进一帧（因此恰好一个 tick，除非暂停或改变了时间缩放）。
    pub fn frame(&mut self) -> &FrameOutput {
        let input = FrameInput {
            real_dt: Time::FIXED_DT,
            window_size_px: self.window_size_px,
            mouse_pos_px: None,
            events: Vec::new(),
        };
        self.frame_with(input)
    }

    /// 连续推进 `n` 帧。
    pub fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.frame();
        }
    }

    /// 以自定义输入推进一帧。
    pub fn frame_with(&mut self, input: FrameInput) -> &FrameOutput {
        self.app.frame(input)
    }

    /// 登记参与快照的组件类型；快照按登记顺序逐类型写出。重复登记被忽略。
    pub fn register_snapshot<T: Component + Snapshot>(&mut self) -> &mut Self {
        let id = TypeId::of::<T>();
        if !self.snapshot_writers.iter().any(|(t, _)| *t == id) {
            self.snapshot_writers.push((id, write_component::<T>));
        }
        self
    }

    /// World 的可比较快照：按登记顺序逐类型，类型内按 `Entity` 升序写出
    /// `[实体数 u32][(实体 id u64, 组件字节)…]`，全部小端序。
    #[must_use]
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (_, writer) in &self.snapshot_writers {
            writer(self.app.world(), &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellia_core::{Stage, Vec2};

    struct Hp(u16);
    impl Snapshot for Hp {
        fn write(&self, out: &mut Vec<u8>) {
            out.extend(self.0.to_le_bytes());
        }
    }

    fn drift(world: &mut World) {
        for t in world.entities.query_mut::<&mut Transform2D>() {
            t.pos.x += 0.5;
        }
    }

    fn runner() -> HeadlessRunner {
        let mut app = App::new();
        app.add_system(Stage::FixedUpdate, drift);
        app.world_mut()
            .spawn((Transform2D::new(Vec2::new(1.0, 2.0)), Hp(3)));
        app.world_mut().spawn((Hp(9),));
        let mut runner = HeadlessRunner::new(app);
        runner.register_snapshot::<Hp>().register_snapshot::<Hp>();
        runner
    }

    #[test]
    fn frame_runs_exactly_one_tick() {
        let mut r = runner();
        r.frames(3);
        let time = r.app().world().resources.expect::<Time>();
        assert_eq!((time.fixed_tick, time.frame), (3, 3));
    }

    #[test]
    fn snapshot_layout_and_determinism() {
        let mut a = runner();
        let mut b = runner();
        a.frames(10);
        b.frames(10);
        let snap = a.snapshot();
        assert_eq!(snap, b.snapshot());
        // Transform2D：1 个实体 × (8 + 16) 字节；Hp：2 个实体 × (8 + 2) 字节；重复登记被忽略
        assert_eq!(snap.len(), 4 + 24 + 4 + 2 * 10);
        b.frames(1);
        assert_ne!(snap, b.snapshot());
    }
}
