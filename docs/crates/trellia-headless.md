# trellia-headless 使用说明

无窗口后端：`trellia::run_headless(app)` 或 `HeadlessRunner::new(app)`（启动错误时 panic）。

## 推进

- `frame()`：`real_dt = 1/60`，恰好 1 tick；`frames(n)`；`frame_with(FrameInput)` 自定义输入。
- `click(logical_pos)`：同帧按下 + 释放左键，鼠标停留在该位置；`press_key(key)` 同理。

## 观察

- `last_draw_list()`：上一帧排序后的绘制命令（占位渲染断言）。
- `take_audio_log()`：累积的音频请求。
- `snapshot()`：按登记顺序逐类型、类型内按 Entity 升序写出的字节；`register_snapshot::<T>()`
  登记组件（`T: Snapshot`），`Transform2D` 默认已登记。

```rust
use trellia_headless::{HeadlessRunner, Snapshot};
struct Hp(u16);
impl Snapshot for Hp { fn write(&self, out: &mut Vec<u8>) { out.extend(self.0.to_le_bytes()); } }
fn run() -> Vec<u8> {
    let mut r = HeadlessRunner::new(trellia_core::App::new());
    r.register_snapshot::<Hp>();
    r.frames(600);
    r.snapshot()
}
assert_eq!(run(), run());
```

## UI

每帧以 `egui::Context::run_ui` 执行全部 UI 回调（场景 `ui`、调试面板），不渲染；
`RawInput.time` 由帧号推导，不读系统时钟。
