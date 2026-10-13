# ADR-009：形状与文字命令可选的 `sort_y`（游戏 spec E-3）

- 编号：ADR-009
- 标题：`rect_sorted` / `rect_stroke_sorted` / `circle_sorted` / `text_sorted`
- 日期：2026-10-13
- 状态：提议（来自 Rootguard 游戏 spec 第 15 章 E-3，对应引擎 v1 报告第 3.9 条）

## 背景

ADR-004 §7 规定非精灵命令 `sort_y = 0`，同层只按提交顺序排序。占位渲染需要让不同行的
矩形 / 圆 / 文字按 y 交错遮挡（下方一行盖住上方一行），否则只能每行一个 `Layer`。

## 决策

`DrawList` 新增四个方法，与现有方法相同但多一个 `sort_y` 参数（排序键仍为 `(layer, sort_y, seq)`）：

```rust
pub fn rect_sorted(&mut self, layer: Layer, sort_y: f32, rect: Rect, color: Color);
pub fn rect_stroke_sorted(&mut self, layer: Layer, sort_y: f32, rect: Rect, color: Color, width: f32);
pub fn circle_sorted(&mut self, layer: Layer, sort_y: f32, center: Vec2, radius: f32, color: Color);
pub fn text_sorted(&mut self, layer: Layer, sort_y: f32, pos: Vec2, text: impl Into<String>, size: f32, color: Color);
```

`text_sorted` 固定居中对齐：占位标签均居中，且再加一个对齐参数会超过 clippy
`too_many_arguments` 的阈值（工程规范禁止 `#[allow]` 压制）。需要其他对齐时继续用 `text_aligned`。

## 后果

- 现有方法行为不变（`sort_y = 0`），golden 不受影响。
- 精灵与形状可在同层按 y 混排。

## 替代方案

1. 给现有方法加参数：破坏所有调用方，否决。
2. 返回构建器 `.sort_y(y)`：`rect` 等现在返回 `()`，改返回值同样破坏 API 且与精灵的构建器语义重复，否决。
