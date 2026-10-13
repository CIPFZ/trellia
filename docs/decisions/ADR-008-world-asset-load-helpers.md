# ADR-008：在 World 上直接加载资产的辅助函数（游戏 spec E-2）

- 编号：ADR-008
- 标题：`load_atlas_into` / `load_clip_into` / `load_audio_into`
- 日期：2026-10-13
- 状态：提议（来自 Rootguard 游戏 spec 第 15 章 E-2，对应引擎 v1 报告第 3.8 条与第 4 节）

## 背景

`load_atlas` / `load_clip` 需要同时可变借用多个 `Assets<T>`，而它们都存放在 `World.resources`
中。游戏层只能临时 `remove` 再 `insert`（`examples/lane_demo` 与 `examples/sprites` 各写了一遍），
音频则需要先克隆 `AssetServer` 再写入 `Assets<AudioBytes>`。

## 决策

新增三个自由函数（不改动现有 API）：

```rust
// trellia-gfx
pub fn load_atlas_into(world: &mut World, rel_ron: &str) -> Result<Handle<Atlas>, AssetError>;
pub fn load_clip_into(world: &mut World, atlas: Handle<Atlas>, rel_ron: &str)
    -> Result<Handle<AnimationClip>, AssetError>;
// trellia-audio
pub fn load_audio_into(world: &mut World, rel: &str) -> Result<Handle<AudioClip>, AssetError>;
```

- `AssetServer` 从 `World.resources` 读取（克隆）；不存在时返回 `AssetError::RootNotFound`，
  信息说明未添加 `AssetsPlugin`。
- 所需的 `Assets<T>` 不存在时以空表代替；无论成功失败，调用结束时都放回 World。
- 同一路径重复加载返回已有句柄（与 `load_atlas` / `load_clip` 一致；音频按 `handle_by_path` 去重）。
- 门面 `prelude` 导出三者；`lane_demo` 改用它们（golden 快照不变）。

## 后果

- 消除样板；依赖边不变（gfx → core / assets，audio → core / assets，ADR-002）。
- 函数名带 `_into` 后缀，与既有的 `load_atlas` / `load_clip` 区分。

## 替代方案

1. 给 `World` 加扩展 trait（`world.load_atlas(..)`）：trait 需定义在 gfx / audio 中并要求调用方导入，
   且方法名与自由函数 `load_atlas` 冲突，否决。
2. 引入 `SystemParam` 式的多资源借用：超出 v1 范围（SPEC 1.4 非目标），否决。
