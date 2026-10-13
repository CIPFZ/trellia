//! World、延迟命令与内置空间组件（SPEC.md 4.3）。

use glam::Vec2;
use hecs::{Bundle, ComponentError, DynamicBundle, Entity, EntityBuilder, NoSuchEntity};

use crate::{EventBus, Resources};

/// 组件：任意 `Send + Sync + 'static` 的纯数据类型。对应 UC-03。
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// 实体、资源、事件与延迟命令的聚合。字段公开，以便系统分别借用。
/// 对应 UC-02、UC-03、UC-04、UC-06。
#[derive(Default)]
pub struct World {
    /// 实体与组件存储（hecs，见 ADR-001）。
    pub entities: hecs::World,
    /// 全局单例资源。
    pub resources: Resources,
    /// 类型化事件总线。
    pub events: EventBus,
    /// 延迟 spawn / despawn / insert / remove 命令（UC-04）。
    pub commands: CommandBuffer,
}

impl World {
    /// 创建空 World。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 立即创建实体。遍历查询期间请改用 `commands.spawn`。
    pub fn spawn(&mut self, bundle: impl DynamicBundle) -> Entity {
        self.entities.spawn(bundle)
    }

    /// 立即销毁实体。
    ///
    /// # Errors
    ///
    /// 实体不存在时返回 [`NoSuchEntity`]。
    pub fn despawn(&mut self, e: Entity) -> Result<(), NoSuchEntity> {
        self.entities.despawn(e)
    }

    /// 借用实体的某个组件；实体或组件不存在时返回 `None`。
    #[must_use]
    pub fn get<T: Component>(&self, e: Entity) -> Option<hecs::Ref<'_, T>> {
        self.entities.get::<&T>(e).ok()
    }

    /// 可变借用实体的某个组件；实体或组件不存在时返回 `None`。
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<hecs::RefMut<'_, T>> {
        self.entities.get::<&mut T>(e).ok()
    }

    /// 查询（运行时借用检查）。相同操作序列下迭代顺序稳定。
    #[must_use]
    pub fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<'_, Q> {
        self.entities.query::<Q>()
    }

    /// 可变查询（编译期借用检查）。
    pub fn query_mut<Q: hecs::Query>(&mut self) -> hecs::QueryMut<'_, Q> {
        self.entities.query_mut::<Q>()
    }

    /// 实体是否存在。
    #[must_use]
    pub fn contains(&self, e: Entity) -> bool {
        self.entities.contains(e)
    }

    /// 当前实体数量。
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len() as usize
    }

    /// 按记录顺序应用延迟命令。由 App 在每个 tick 末与 PostUpdate 后自动调用。
    pub(crate) fn apply_commands(&mut self) {
        let mut cmds = std::mem::take(&mut self.commands.cmds);
        for cmd in cmds.drain(..) {
            match cmd {
                Command::Spawn(mut builder) => {
                    self.entities.spawn(builder.build());
                }
                Command::Despawn(e) => {
                    if self.entities.despawn(e).is_err() {
                        log::trace!("CommandBuffer: despawn 目标 {e:?} 已不存在，忽略");
                    }
                }
                Command::Insert(e, mut builder) => {
                    if self.entities.insert(e, builder.build()).is_err() {
                        log::warn!("CommandBuffer: insert 目标 {e:?} 已不存在，忽略");
                    }
                }
                Command::Remove(e, remove) => match remove(&mut self.entities, e) {
                    Ok(()) => {}
                    Err(ComponentError::NoSuchEntity) => {
                        log::warn!("CommandBuffer: remove 目标 {e:?} 已不存在，忽略");
                    }
                    Err(ComponentError::MissingComponent(m)) => {
                        log::trace!("CommandBuffer: remove 目标 {e:?} 缺少组件 {m}，忽略");
                    }
                },
            }
        }
        // 复用分配；应用期间不会有新命令写入。
        self.commands.cmds = cmds;
    }
}

type RemoveFn = fn(&mut hecs::World, Entity) -> Result<(), ComponentError>;

fn remove_bundle<T: Bundle + 'static>(
    world: &mut hecs::World,
    e: Entity,
) -> Result<(), ComponentError> {
    world.remove::<T>(e).map(drop)
}

enum Command {
    Spawn(EntityBuilder),
    Despawn(Entity),
    Insert(Entity, EntityBuilder),
    Remove(Entity, RemoveFn),
}

/// 延迟命令缓冲（UC-04、UC-06）。遍历查询时用它创建 / 销毁实体或增删组件，
/// 命令在每个 FixedUpdate tick 末与 PostUpdate 后按记录顺序应用。
///
/// 对已销毁实体执行 `insert` / `remove` 时静默忽略并记录 `log::warn!`。
#[derive(Default)]
pub struct CommandBuffer {
    cmds: Vec<Command>,
}

impl CommandBuffer {
    /// 延迟创建实体。
    pub fn spawn(&mut self, bundle: impl DynamicBundle) {
        let mut builder = EntityBuilder::new();
        builder.add_bundle(bundle);
        self.cmds.push(Command::Spawn(builder));
    }

    /// 延迟销毁实体；实体届时已不存在则忽略。
    pub fn despawn(&mut self, e: Entity) {
        self.cmds.push(Command::Despawn(e));
    }

    /// 延迟向实体添加组件（低频结构性变化，UC-06）。
    pub fn insert(&mut self, e: Entity, bundle: impl DynamicBundle) {
        let mut builder = EntityBuilder::new();
        builder.add_bundle(bundle);
        self.cmds.push(Command::Insert(e, builder));
    }

    /// 延迟从实体移除组件集合 `T`（UC-06）。
    pub fn remove<T: Bundle + 'static>(&mut self, e: Entity) {
        self.cmds.push(Command::Remove(e, remove_bundle::<T>));
    }
}

/// 与渲染无关的空间组件。逻辑坐标，像素单位，原点左上，y 向下。对应 UC-01、UC-26。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    /// 当前位置。
    pub pos: Vec2,
    /// 上一 tick 的位置，由 App 在每个 tick 开始时同步（UC-26 渲染插值）。
    pub prev_pos: Vec2,
}

impl Transform2D {
    /// 在 `pos` 处创建，`prev_pos = pos`。
    #[must_use]
    pub fn new(pos: Vec2) -> Self {
        Self { pos, prev_pos: pos }
    }

    /// 瞬移：同时设置 `pos` 与 `prev_pos`，避免插值拖影。
    pub fn teleport(&mut self, pos: Vec2) {
        self.pos = pos;
        self.prev_pos = pos;
    }

    /// 渲染插值位置：`prev_pos + (pos - prev_pos) * alpha`。
    #[must_use]
    pub fn lerp(&self, alpha: f32) -> Vec2 {
        self.prev_pos + (self.pos - self.prev_pos) * alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Hp(u32);
    #[derive(Debug, PartialEq)]
    struct Armor;

    #[test]
    fn commands_apply_in_order_and_ignore_dead_targets() {
        let mut world = World::new();
        let a = world.spawn((Hp(3), Armor));
        let dead = world.spawn((Hp(1),));
        world.despawn(dead).expect("刚创建的实体");

        world.commands.remove::<(Armor,)>(a);
        world.commands.insert(a, (Hp(5),));
        world.commands.insert(dead, (Hp(9),));
        world.commands.remove::<(Hp,)>(dead);
        world.commands.despawn(dead);
        world.commands.spawn((Hp(7),));
        assert_eq!(world.entity_count(), 1);

        world.apply_commands();
        assert_eq!(world.entity_count(), 2);
        assert!(world.get::<Armor>(a).is_none());
        assert_eq!(*world.get::<Hp>(a).expect("insert 覆盖"), Hp(5));
        assert!(!world.contains(dead));
    }

    #[test]
    fn transform_lerp_and_teleport() {
        let mut t = Transform2D::new(Vec2::ZERO);
        t.pos = Vec2::new(10.0, 20.0);
        assert!((t.lerp(0.5) - Vec2::new(5.0, 10.0)).length() < 1e-5);
        t.teleport(Vec2::new(1.0, 1.0));
        assert!((t.lerp(0.5) - Vec2::new(1.0, 1.0)).length() < 1e-5);
    }
}
