//! 种子化随机数（SPEC.md 4.8）。

use rand::distr::uniform::{SampleRange, SampleUniform};
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg32;

/// 模拟层唯一的随机数来源（UC-13、UC-14）。由 App 以种子 `0` 插入，
/// 游戏层在进入关卡时以显式种子替换。禁止使用线程随机数或以时间做种子。
#[derive(Clone, Debug)]
pub struct GameRng {
    rng: Pcg32,
    seed: u64,
}

impl GameRng {
    /// 以给定种子创建。
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: Pcg32::seed_from_u64(seed),
            seed,
        }
    }

    /// 创建时使用的种子。
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// 在范围内均匀取值，例如 `rng.gen_range(0..30)`。
    ///
    /// # Panics
    ///
    /// 范围为空时 panic。
    pub fn gen_range<T: SampleUniform>(&mut self, range: impl SampleRange<T>) -> T {
        self.rng.random_range(range)
    }

    /// 以概率 `p` 返回 true。
    ///
    /// # Panics
    ///
    /// `p` 不在 `[0, 1]` 内时 panic。
    pub fn gen_bool(&mut self, p: f64) -> bool {
        self.rng.random_bool(p)
    }

    /// 原地打乱切片。
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        slice.shuffle(&mut self.rng);
    }

    /// 均匀选取一个元素；空切片返回 `None`。
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        slice.choose(&mut self.rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(seed: u64) -> Vec<u32> {
        let mut rng = GameRng::from_seed(seed);
        let mut out: Vec<u32> = (0..32).map(|_| rng.gen_range(0..1_000_000)).collect();
        let mut deck = [1, 2, 3, 4, 5, 6, 7, 8];
        rng.shuffle(&mut deck);
        out.extend(deck);
        out.push(*rng.choose(&[10, 20, 30]).expect("非空切片"));
        out.push(u32::from(rng.gen_bool(0.5)));
        out
    }

    #[test]
    fn rng_deterministic() {
        assert_eq!(sequence(42), sequence(42));
        assert_ne!(sequence(42), sequence(43));
        assert_eq!(GameRng::from_seed(42).seed(), 42);
    }

    #[test]
    fn choose_empty_is_none() {
        let mut rng = GameRng::from_seed(0);
        let empty: [u8; 0] = [];
        assert!(rng.choose(&empty).is_none());
    }
}
