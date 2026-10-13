# ADR-001：hecs 类型允许出现在 trellia-core 公共 API 中

- 编号：ADR-001
- 标题：hecs 类型允许出现在 trellia-core 公共 API 中
- 日期：2026-10-13
- 状态：已接受

## 背景

SPEC.md 4.3 / 4.3.1 规定 `World` 以 `hecs::World` 为实体存储，并在公共 API 中直接使用
`hecs::Entity`（`pub use hecs::Entity`）、`hecs::Query`、`hecs::DynamicBundle`、`hecs::Bundle`、
`hecs::Ref` / `RefMut`、`hecs::QueryBorrow` / `QueryMut`、`hecs::NoSuchEntity`。
`World.entities` 字段本身也是公开的 `hecs::World`。

## 决策

允许 hecs 类型出现在 `trellia-core` 的公共 API 中，不做双层封装：

- `trellia-core` 以 `pub use hecs;` 重新导出 hecs，游戏层无需单独依赖 hecs 即可书写查询类型；
- `Entity` 以 `pub use hecs::Entity` 导出；
- `CommandBuffer` 是对 `hecs::CommandBuffer` 的薄封装，仅为了统一“对已销毁实体 insert / remove
  静默忽略并 `log::warn!`”的语义（SPEC.md 4.3.2）。

## 后果

- 正面：零封装成本；hecs 的查询能力（`With` / `Without` / `Option<&T>` 等）直接可用。
- 负面：hecs 升级大版本时，`trellia-core` 的公共 API 会随之破坏性变化；
  将来若替换存储（v1 非目标，SPEC.md 1.4），需要整体迁移游戏层查询代码。
- 约束：禁止在持有 `QueryBorrow` 的同时调用 `world.spawn` / `despawn`；遍历中创建 / 销毁必须
  使用 `world.commands`（UC-04）。

## 替代方案

1. 自研 `Entity` / `Query` 包装类型：需要复制 hecs 的大量泛型 API，代码量大且无用例需求，否决。
2. 只暴露 `World` 方法、隐藏 `hecs::World` 字段：系统无法在借用 `resources` 的同时查询实体
   （SPEC.md 4.1 要求按字段分别借用），否决。
