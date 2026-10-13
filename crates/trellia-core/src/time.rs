//! 时间资源与以 tick 计数的计时器（SPEC.md 4.6 / 4.7）。

/// 时间资源，由 App 维护。对应 UC-10、UC-11、UC-26、UC-71。
///
/// 游戏逻辑中的时长一律以 tick 为单位；FixedUpdate 系统禁止读取 `real_dt`（SPEC.md 12.1）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    /// 固定步长秒数，恒为 `1.0 / 60.0`。只读。
    pub fixed_dt: f32,
    /// 自 Startup 起已执行的 tick 总数。
    pub fixed_tick: u64,
    /// 已渲染帧数（每次 `App::frame` 结束时 +1）。
    pub frame: u64,
    /// 本帧真实秒数（未缩放，未钳制）。
    pub real_dt: f32,
    /// 时间缩放，默认 1.0；App 使用时钳制到 `[0.0, 4.0]`。
    pub time_scale: f32,
    /// 为 true 时 FixedUpdate 不执行，Update / Render 照常。
    pub paused: bool,
    /// 本帧渲染插值因子，范围 `[0, 1)`。
    pub alpha: f32,
    /// 为 true 时无视 `paused` 执行恰好 1 个 tick 后复位。
    pub step_once: bool,
}

impl Time {
    /// 固定步长：每秒 60 tick（UC-10）。
    pub const FIXED_DT: f32 = 1.0 / 60.0;
}

impl Default for Time {
    fn default() -> Self {
        Self {
            fixed_dt: Self::FIXED_DT,
            fixed_tick: 0,
            frame: 0,
            real_dt: 0.0,
            time_scale: 1.0,
            paused: false,
            alpha: 0.0,
            step_once: false,
        }
    }
}

/// 以 tick 计数的倒计时（UC-12）。纯数据，由持有者每 tick 调用 [`Timer::tick`]。
///
/// `duration == 0` 的计时器永不触发。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timer {
    /// 距下次触发剩余的 tick 数。
    pub remaining: u32,
    /// 周期（tick）。
    pub duration: u32,
    /// 是否在触发后自动重置。
    pub repeating: bool,
}

impl Timer {
    /// 单次计时器：`ticks` 个 tick 后触发一次。
    #[must_use]
    pub fn once(ticks: u32) -> Self {
        Self {
            remaining: ticks,
            duration: ticks,
            repeating: false,
        }
    }

    /// 周期计时器：每 `ticks` 个 tick 触发一次。
    #[must_use]
    pub fn repeating(ticks: u32) -> Self {
        Self {
            remaining: ticks,
            duration: ticks,
            repeating: true,
        }
    }

    /// 推进一个 tick。返回 true 表示本 tick 触发；周期计时器触发后自动重置。
    pub fn tick(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        if self.remaining > 0 {
            return false;
        }
        if self.repeating {
            self.remaining = self.duration;
        }
        true
    }

    /// 重新开始计时。
    pub fn reset(&mut self) {
        self.remaining = self.duration;
    }

    /// 单次计时器是否已触发。周期计时器永不结束。
    #[must_use]
    pub fn finished(&self) -> bool {
        !self.repeating && self.remaining == 0
    }

    /// 当前周期的进度，0.0 → 1.0。
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.duration == 0 {
            return 1.0;
        }
        1.0 - self.remaining as f32 / self.duration as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_repeating_resets() {
        let mut timer = Timer::repeating(3);
        let fired: Vec<u32> = (1..=9).filter(|_| timer.tick()).collect();
        assert_eq!(fired, vec![3, 6, 9]);
        assert!(!timer.finished());
    }

    #[test]
    fn timer_once_fires_once_then_finished() {
        let mut timer = Timer::once(2);
        assert!(!timer.tick());
        assert!(timer.tick());
        assert!(timer.finished());
        assert!(!timer.tick());
        assert!((timer.progress() - 1.0).abs() < 1e-6);
        timer.reset();
        assert!(!timer.finished());
        assert!(timer.progress().abs() < 1e-6);
    }

    #[test]
    fn zero_duration_never_fires() {
        let mut timer = Timer::repeating(0);
        assert!(!(0..10).any(|_| timer.tick()));
    }
}
