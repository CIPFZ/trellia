# trellia-headless

无窗口后端 `HeadlessRunner`：以固定 `real_dt = 1/60` 驱动 App（每帧恰好 1 tick）、
生成可逐字节比较的 World 快照，用于数值规则与流程的无窗口测试（UC-13、UC-73、UC-74）。

## 最小示例

```rust
use trellia_core::{App, Transform2D, Vec2};
use trellia_headless::{HeadlessRunner, Snapshot};

struct Hp(u16);
impl Snapshot for Hp {
    fn write(&self, out: &mut Vec<u8>) { out.extend(self.0.to_le_bytes()); }
}

let build = || {
    let mut app = App::new();
    app.world_mut().spawn((Transform2D::new(Vec2::ZERO), Hp(5)));
    let mut runner = HeadlessRunner::new(app);
    runner.register_snapshot::<Hp>(); // Transform2D 默认已登记
    runner.frames(600);
    runner.snapshot()
};
assert_eq!(build(), build());
```

快照格式：按登记顺序逐类型，类型内按 `Entity` 升序写出
`[实体数 u32][(实体 id u64, 组件字节)…]`，全部小端序。
