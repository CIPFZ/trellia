# ADR-011：背景音乐交叉淡化 `AudioRequest::CrossfadeMusic`（游戏 spec E-5）

- 编号：ADR-011
- 标题：`AudioRequest::CrossfadeMusic { clip, ticks }` 与 rodio 后端的淡化实现
- 日期：2026-10-14
- 状态：提议（来自 Rootguard 游戏 spec 第 15 章 E-5，对应引擎 v1 报告第 4 节“音乐淡入淡出、切换场景时的 BGM 交叉淡化”）

## 背景

UC-51 要求切换场景时切换 BGM；v1 的 `PlayMusic` 是硬切。游戏 spec E-5 要求
`AudioRequest::CrossfadeMusic { clip, ticks }`。`AudioBackend` 只在有请求时被调用，没有逐帧回调，
淡化需要在请求之间持续进行。

## 决策

1. `AudioRequest` 新增 `CrossfadeMusic { clip, ticks }`，`AudioQueue::crossfade_music(clip, ticks)`。
   字段按游戏 spec 原文；交叉淡化的新音乐**总是循环**（BGM 语义）。需要非循环时仍用 `PlayMusic`。
2. 时长以 tick 给出（与引擎其余时长一致），后端按 `Time::FIXED_DT` 换算为秒。`ticks == 0` 等价于
   循环的 `PlayMusic`；没有当前音乐时只淡入；新音乐解码失败时保留当前音乐并警告。
3. rodio 后端：每条音乐源包一层 `amplify → stoppable → periodic_access(10 ms)`，增益在音频线程中按
   样本时钟线性推进（新音乐 0 → 1；旧音乐收到原子标志后 1 → 0 并 `stop()` 自身）。淡出中的旧音乐放在
   `fading` 列表，播放器变空后在下次处理请求时清理；`PlayMusic` / `StopMusic` 立即停止全部音乐（含淡出中的），
   `SetBusVolume(Music)` 同时作用于淡出中的音乐。
4. 不新增 trait 方法、不新增依赖；headless 只记录请求（UC-53 断言方式不变）。

## 后果

- 公共 API 增加一个枚举变体与一个方法。对 `AudioRequest` 做穷尽 `match` 的外部代码需要补一个分支
  （引擎内只有 rodio 后端）。
- 淡化的听感未经人耳确认（与引擎报告第 3.3 条同样的限制）；包络函数 `ramp` 有单元测试。

## 替代方案

1. 给 `AudioBackend` 加 `update(dt)` 逐帧回调，在主线程调音量：需要改 trait 与两个后端的调用点，
   且精度受帧率影响，否决。
2. 用 rodio 的 `take_crossfade_with`：只能在构造时把两个源合成一个，无法淡出已在播放的旧音乐，否决。
3. 增加 `looped` 字段：偏离游戏 spec 字面，且当前没有非循环交叉淡化的需求，否决。
