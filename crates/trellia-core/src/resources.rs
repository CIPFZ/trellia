//! 以类型为键的全局单例容器（SPEC.md 4.4）。

use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

/// 可存入 [`Resources`] 的全局单例类型。对应 UC-10、UC-11、UC-14、UC-42。
pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

/// 以类型为键的资源容器，每种类型最多一个实例。对应 UC-10、UC-11、UC-14、UC-42。
///
/// 内部为 `HashMap<TypeId, _>`，只按键查找、从不迭代，因此不影响确定性（SPEC.md 12.3）。
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    /// 插入资源，返回同类型的旧值（若有）。
    pub fn insert<R: Resource>(&mut self, r: R) -> Option<R> {
        self.map
            .insert(TypeId::of::<R>(), Box::new(r))
            .and_then(|old| old.downcast::<R>().ok())
            .map(|b| *b)
    }

    /// 移除并返回资源。
    pub fn remove<R: Resource>(&mut self) -> Option<R> {
        self.map
            .remove(&TypeId::of::<R>())
            .and_then(|old| old.downcast::<R>().ok())
            .map(|b| *b)
    }

    /// 借用资源。
    #[must_use]
    pub fn get<R: Resource>(&self) -> Option<&R> {
        self.map
            .get(&TypeId::of::<R>())
            .and_then(|b| b.downcast_ref::<R>())
    }

    /// 可变借用资源。
    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.map
            .get_mut(&TypeId::of::<R>())
            .and_then(|b| b.downcast_mut::<R>())
    }

    /// 借用一定存在的资源（`Time`、`Input`、`GameRng` 等）。
    ///
    /// # Panics
    ///
    /// 资源不存在时 panic，信息包含类型名。
    #[must_use]
    pub fn expect<R: Resource>(&self) -> &R {
        self.get::<R>()
            .unwrap_or_else(|| panic!("资源不存在: {}", type_name::<R>()))
    }

    /// 可变借用一定存在的资源。
    ///
    /// # Panics
    ///
    /// 资源不存在时 panic，信息包含类型名。
    pub fn expect_mut<R: Resource>(&mut self) -> &mut R {
        self.get_mut::<R>()
            .unwrap_or_else(|| panic!("资源不存在: {}", type_name::<R>()))
    }

    /// 是否存在该类型的资源。
    #[must_use]
    pub fn contains<R: Resource>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<R>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Score(u32);

    #[test]
    fn insert_get_remove() {
        let mut res = Resources::default();
        assert_eq!(res.insert(Score(1)), None);
        assert_eq!(res.insert(Score(2)), Some(Score(1)));
        res.expect_mut::<Score>().0 += 1;
        assert_eq!(res.get::<Score>(), Some(&Score(3)));
        assert!(res.contains::<Score>());
        assert_eq!(res.remove::<Score>(), Some(Score(3)));
        assert!(!res.contains::<Score>());
    }

    #[test]
    #[should_panic(expected = "Score")]
    fn expect_missing_panics_with_type_name() {
        let res = Resources::default();
        let _ = res.expect::<Score>();
    }
}
