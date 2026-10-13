//! 视觉补间（SPEC.md 4.7）。按真实帧时间推进，不参与确定性；
//! 结果禁止影响任何 FixedUpdate 逻辑。

use glam::Vec2;

/// 补间曲线。对应 UC-24（视觉反馈）、UC-26。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    /// 线性。
    Linear,
    /// 二次缓出：`1 - (1 - t)^2`。
    EaseOutQuad,
}

impl Easing {
    fn apply(self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

macro_rules! tween_type {
    ($(#[$doc:meta])* $name:ident, $value:ty) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name {
            /// 起始值。
            pub from: $value,
            /// 目标值。
            pub to: $value,
            /// 总时长（秒）。
            pub duration_secs: f32,
            /// 已经过时长（秒），不超过 `duration_secs`。
            pub elapsed_secs: f32,
            /// 补间曲线。
            pub easing: Easing,
        }

        impl $name {
            /// 创建一个从 `from` 到 `to`、历时 `duration_secs` 秒的补间。
            #[must_use]
            pub fn new(from: $value, to: $value, duration_secs: f32, easing: Easing) -> Self {
                Self { from, to, duration_secs, elapsed_secs: 0.0, easing }
            }

            /// 推进 `dt` 秒并返回当前值。
            pub fn advance(&mut self, dt: f32) -> $value {
                self.elapsed_secs = (self.elapsed_secs + dt.max(0.0)).min(self.duration_secs.max(0.0));
                self.value()
            }

            /// 当前值。
            #[must_use]
            pub fn value(&self) -> $value {
                let t = if self.duration_secs > 0.0 {
                    (self.elapsed_secs / self.duration_secs).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                self.from + (self.to - self.from) * self.easing.apply(t)
            }

            /// 是否已到达终点。
            #[must_use]
            pub fn finished(&self) -> bool {
                self.elapsed_secs >= self.duration_secs
            }
        }
    };
}

tween_type!(
    /// `f32` 补间（卡牌弹入缩放、淡入淡出）。对应 UC-24、UC-26。
    TweenF32,
    f32
);
tween_type!(
    /// `Vec2` 补间（阳光飘落轨迹）。对应 UC-24、UC-26。
    TweenVec2,
    Vec2
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_and_ease_out() {
        let mut t = TweenF32::new(0.0, 10.0, 1.0, Easing::Linear);
        assert!((t.advance(0.5) - 5.0).abs() < 1e-5);
        assert!(!t.finished());
        assert!((t.advance(10.0) - 10.0).abs() < 1e-5);
        assert!(t.finished());

        let mut e = TweenF32::new(0.0, 1.0, 1.0, Easing::EaseOutQuad);
        assert!((e.advance(0.5) - 0.75).abs() < 1e-5);
    }

    #[test]
    fn vec2_tween() {
        let mut t = TweenVec2::new(Vec2::ZERO, Vec2::new(4.0, 8.0), 2.0, Easing::Linear);
        assert!((t.advance(1.0) - Vec2::new(2.0, 4.0)).length() < 1e-5);
    }

    #[test]
    fn zero_duration_is_immediately_finished() {
        let t = TweenF32::new(1.0, 2.0, 0.0, Easing::Linear);
        assert!(t.finished());
        assert!((t.value() - 2.0).abs() < 1e-5);
    }
}
