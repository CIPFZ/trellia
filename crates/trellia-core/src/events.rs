//! 类型化、双缓冲的事件总线（SPEC.md 4.5）。

use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

/// 可通过 [`EventBus`] 发送的事件类型。事件应为纯数据（`Clone + Debug`）。
/// 对应 UC-23、UC-53。
pub trait Event: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Event for T {}

/// 单一类型事件的双缓冲队列。事件在发送后的“本帧剩余部分 + 下一整帧”内可读。
/// 对应 UC-23、UC-53。
pub struct Events<T: Event> {
    previous: Vec<T>,
    current: Vec<T>,
}

impl<T: Event> Default for Events<T> {
    fn default() -> Self {
        Self {
            previous: Vec::new(),
            current: Vec::new(),
        }
    }
}

impl<T: Event> Events<T> {
    /// 发送一个事件。
    pub fn send(&mut self, e: T) {
        self.current.push(e);
    }

    /// 迭代上一帧与本帧的全部事件（先旧后新）。
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.previous.iter().chain(self.current.iter())
    }

    /// 取走上一帧与本帧的全部事件（先旧后新），常用于只应被一个消费者处理的事件。
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.previous.drain(..).chain(self.current.drain(..))
    }

    /// 两个缓冲是否都为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.previous.is_empty() && self.current.is_empty()
    }

    /// `Stage::First` 调用：丢弃上一帧事件，本帧事件变为上一帧。
    pub(crate) fn swap(&mut self) {
        self.previous.clear();
        std::mem::swap(&mut self.previous, &mut self.current);
    }
}

trait AnyEvents: Send + Sync {
    fn swap(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Event> AnyEvents for Events<T> {
    fn swap(&mut self) {
        Events::swap(self);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 所有已注册事件类型的集合。对应 UC-23、UC-53。
#[derive(Default)]
pub struct EventBus {
    map: HashMap<TypeId, Box<dyn AnyEvents>>,
}

impl EventBus {
    /// 注册事件类型（通常在 `Plugin::build` 中调用）；重复注册被忽略。
    pub fn register<T: Event>(&mut self) {
        self.map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Events::<T>::default()));
    }

    /// 发送事件。
    ///
    /// # Panics
    ///
    /// 事件类型未注册时 panic，信息包含类型名。
    pub fn send<T: Event>(&mut self, e: T) {
        self.get_mut::<T>().send(e);
    }

    /// 借用某类型的事件队列。
    ///
    /// # Panics
    ///
    /// 事件类型未注册时 panic，信息包含类型名。
    #[must_use]
    pub fn get<T: Event>(&self) -> &Events<T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|b| b.as_any().downcast_ref::<Events<T>>())
            .unwrap_or_else(|| panic!("事件类型未注册: {}", type_name::<T>()))
    }

    /// 可变借用某类型的事件队列。
    ///
    /// # Panics
    ///
    /// 事件类型未注册时 panic，信息包含类型名。
    pub fn get_mut<T: Event>(&mut self) -> &mut Events<T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.as_any_mut().downcast_mut::<Events<T>>())
            .unwrap_or_else(|| panic!("事件类型未注册: {}", type_name::<T>()))
    }

    /// 对全部事件类型翻页（各类型相互独立，迭代顺序不影响结果）。
    pub(crate) fn swap_all(&mut self) {
        for events in self.map.values_mut() {
            events.swap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Ping(u32);

    #[test]
    fn iter_old_then_new_and_drain() {
        let mut bus = EventBus::default();
        bus.register::<Ping>();
        bus.register::<Ping>();
        bus.send(Ping(1));
        bus.swap_all();
        bus.send(Ping(2));
        let seen: Vec<_> = bus.get::<Ping>().iter().cloned().collect();
        assert_eq!(seen, vec![Ping(1), Ping(2)]);
        let drained: Vec<_> = bus.get_mut::<Ping>().drain().collect();
        assert_eq!(drained, vec![Ping(1), Ping(2)]);
        assert!(bus.get::<Ping>().is_empty());
    }

    #[test]
    #[should_panic(expected = "Ping")]
    fn send_unregistered_panics_with_type_name() {
        let mut bus = EventBus::default();
        bus.send(Ping(1));
    }
}
