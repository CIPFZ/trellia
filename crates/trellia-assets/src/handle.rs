//! 资产句柄与容器（SPEC.md 5.1）。

use std::any::type_name;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// 可存入 [`Assets`] 的资产类型。对应 UC-30、UC-31。
pub trait Asset: Send + Sync + 'static {}

/// 轻量、`Copy`、带类型的资产句柄（UC-31）。内部为 `u32` 索引，不做代际校验（v1 资产不卸载）。
///
/// 不同资产类型的句柄不可互换：
///
/// ```compile_fail
/// use trellia_assets::{Assets, AudioBytes, AudioKind, Handle, Image};
/// let mut sounds = Assets::<AudioBytes>::default();
/// let h = sounds.add(AudioBytes { bytes: Vec::new().into(), kind: AudioKind::Wav });
/// let wrong: Handle<Image> = h; // 编译错误：类型不匹配
/// ```
pub struct Handle<T> {
    id: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    fn new(id: u32) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }
}

// 手动实现，不要求 T 满足这些 trait。
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Handle<T> {}
impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({})", type_name::<T>(), self.id)
    }
}

/// 同一类型资产的容器，作为独立 `Resource` 存入 `World.resources`。对应 UC-30、UC-31。
///
/// `by_path` 只用于按路径查找、从不迭代；`iter` 按添加顺序迭代。
pub struct Assets<T: Asset> {
    items: Vec<T>,
    by_path: HashMap<String, Handle<T>>,
}

impl<T: Asset> Default for Assets<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            by_path: HashMap::new(),
        }
    }
}

impl<T: Asset> Assets<T> {
    /// 添加资产并返回新句柄。
    pub fn add(&mut self, item: T) -> Handle<T> {
        let handle = Handle::new(self.items.len() as u32);
        self.items.push(item);
        handle
    }

    /// 以路径为键添加资产；同路径重复添加时丢弃 `item` 并返回已有句柄。
    pub fn add_with_path(&mut self, path: &str, item: T) -> Handle<T> {
        if let Some(handle) = self.by_path.get(path) {
            return *handle;
        }
        let handle = self.add(item);
        self.by_path.insert(path.to_owned(), handle);
        handle
    }

    /// 借用资产。
    #[must_use]
    pub fn get(&self, h: Handle<T>) -> Option<&T> {
        self.items.get(h.id as usize)
    }

    /// 可变借用资产。
    pub fn get_mut(&mut self, h: Handle<T>) -> Option<&mut T> {
        self.items.get_mut(h.id as usize)
    }

    /// 借用一定存在的资产。
    ///
    /// # Panics
    ///
    /// 句柄不属于本容器时 panic。
    #[must_use]
    pub fn expect(&self, h: Handle<T>) -> &T {
        self.get(h).unwrap_or_else(|| panic!("资产不存在: {h:?}"))
    }

    /// 按路径查找已加载资产的句柄。
    #[must_use]
    pub fn handle_by_path(&self, path: &str) -> Option<Handle<T>> {
        self.by_path.get(path).copied()
    }

    /// 资产数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 是否为空（ADR-004）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 按添加顺序迭代全部资产。
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| (Handle::new(i as u32), item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioBytes, AudioKind, Image};

    fn image(w: u32) -> Image {
        Image {
            width: w,
            height: 1,
            rgba: vec![0; w as usize * 4],
        }
    }

    fn assert_copy<T: Copy>(_: T) {}

    #[test]
    fn handle_is_copy_and_typed() {
        // 类型不可互换由 Handle 文档中的 compile_fail 文档测试保证。
        let mut images = Assets::<Image>::default();
        let mut sounds = Assets::<AudioBytes>::default();
        let h = images.add(image(2));
        let s = sounds.add(AudioBytes {
            bytes: Vec::new().into(),
            kind: AudioKind::Ogg,
        });
        assert_copy(h);
        assert_copy(s);
        let copy = h;
        assert_eq!(images.expect(h).width, images.expect(copy).width);
        assert_eq!(sounds.get(s).map(|a| a.kind), Some(AudioKind::Ogg));
    }

    #[test]
    fn add_with_path_dedups() {
        let mut images = Assets::<Image>::default();
        let a = images.add_with_path("demo/a.png", image(1));
        let b = images.add_with_path("demo/a.png", image(2));
        assert_eq!(a, b);
        assert_eq!(images.len(), 1);
        assert_eq!(images.expect(a).width, 1);
        assert_eq!(images.handle_by_path("demo/a.png"), Some(a));
        assert_eq!(images.handle_by_path("demo/b.png"), None);
    }

    #[test]
    fn iter_in_insertion_order() {
        let mut images = Assets::<Image>::default();
        assert!(images.is_empty());
        let a = images.add(image(1));
        let b = images.add(image(2));
        let handles: Vec<_> = images.iter().map(|(h, _)| h).collect();
        assert_eq!(handles, vec![a, b]);
        images.get_mut(b).expect("存在").width = 9;
        assert_eq!(images.expect(b).width, 9);
    }
}
