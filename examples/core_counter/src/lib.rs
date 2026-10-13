//! # `core_counter`
//!
//! M1 验收示例：只用 `trellia-core` 的 Plugin / Stage / Timer / GameRng / CommandBuffer，
//! 在无窗口后端中运行 600 tick 并给出确定的计数结果（UC-10、UC-12、UC-13、UC-14、UC-73）。

#![forbid(unsafe_code)]

use trellia::prelude::*;

/// 计数结果资源。对应 UC-12、UC-13。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    /// 已执行的 tick 数。
    pub ticks: u64,
    /// 周期计时器触发次数。
    pub pulses: u32,
    /// 每次触发时 `GameRng` 取值的累加和。
    pub rng_sum: u64,
}

/// 每 60 tick 触发一次的计时器资源。
struct Pulse(Timer);

/// 示例插件：插入资源并注册计数系统。对应 UC-12、UC-14。
pub struct CounterPlugin {
    /// 随机种子。
    pub seed: u64,
}

impl Plugin for CounterPlugin {
    fn name(&self) -> &'static str {
        "CounterPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(GameRng::from_seed(self.seed))
            .insert_resource(Counter::default())
            .insert_resource(Pulse(Timer::repeating(60)))
            .add_system(Stage::FixedUpdate, count_system);
    }
}

fn count_system(world: &mut World) {
    let fired = world.resources.expect_mut::<Pulse>().0.tick();
    let roll = if fired {
        world
            .resources
            .expect_mut::<GameRng>()
            .gen_range(1..=100_u64)
    } else {
        0
    };
    let counter = world.resources.expect_mut::<Counter>();
    counter.ticks += 1;
    if fired {
        counter.pulses += 1;
        counter.rng_sum += roll;
        let x = roll as f32;
        world.commands.spawn((Transform2D::new(Vec2::new(x, 0.0)),));
    }
}

/// 构建示例 App 并在无窗口后端中运行 `frames` 帧，返回计数结果与快照。
#[must_use]
pub fn run(seed: u64, frames: u32) -> (Counter, Vec<u8>) {
    let mut app = App::new();
    app.add_plugin(CounterPlugin { seed });
    let mut runner = trellia::run_headless(app);
    runner.frames(frames);
    let counter = *runner.app().world().resources.expect::<Counter>();
    (counter, runner.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_is_deterministic() {
        let (a, snap_a) = run(42, 600);
        let (b, snap_b) = run(42, 600);
        assert_eq!(a, b);
        assert_eq!(snap_a, snap_b);
        assert_eq!(a.ticks, 600);
        assert_eq!(a.pulses, 10);
        assert!(a.rng_sum >= 10 && a.rng_sum <= 1000);
    }
}
