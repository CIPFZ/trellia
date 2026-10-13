# trellia-gfx 使用说明

## 插件顺序

`GfxPlugin` 应先于游戏插件添加：它在 FixedUpdate 首位推进动画（同 tick 的游戏系统能读到
`AnimationEvent`），在 Render 首位清空 `DrawList`。

## 坐标

逻辑坐标原点左上、y 向下。`Viewport` 维护 letterbox 缩放与偏移；后端用
`logical_to_px` 把命令换算到窗口像素，`Input.mouse_pos` 已换算为逻辑坐标。

## 加载精灵

```rust
use trellia_assets::{AssetServer, Assets, Image};
use trellia_gfx::{load_atlas, load_clip, Animator, Atlas, AnimationClip, Sprite};

fn load(server: &AssetServer, images: &mut Assets<Image>, atlases: &mut Assets<Atlas>,
        clips: &mut Assets<AnimationClip>) -> Result<(Sprite, Animator), trellia_assets::AssetError> {
    let atlas = load_atlas(server, images, atlases, "demo/walker.atlas.ron")?;
    let walk = load_clip(server, atlases, clips, atlas, "demo/walker_walk.anim.ron")?;
    Ok((Sprite::new(atlas), Animator::new(walk)))
}
```

## 动画

- 帧时长单位为 tick；速度为整数分数（冰冻 `set_speed(1, 2)`）。
- `play(clip)` 对同一片段不重置；`restart()` 强制从头播放。
- 事件：进入带 `event` 的帧时发送一次 `AnimationEvent { entity, event }`，停留期间不重复。

## 占位渲染（UC-27）

没有美术时用 `DrawList::rect` + `text` 画方块与标签；形状命令 `sort_y = 0`，同层按提交顺序。
