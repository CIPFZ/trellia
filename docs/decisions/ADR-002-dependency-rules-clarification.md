# ADR-002：依赖规则细化（audio → assets、门面、devtools、示例）

- 编号：ADR-002
- 标题：依赖规则 R1 ～ R5 的细化与 check_deps.sh 允许表
- 日期：2026-10-13
- 状态：已接受

## 背景

SPEC.md 3.2 R2 规定 `trellia-assets` / `gfx` / `audio` / `scene` “只允许依赖 `trellia-core`
（`trellia-gfx` 额外允许依赖 `trellia-assets`）”。但 SPEC.md 7 中 `trellia-audio` 的公共 API 为：

```rust
pub type AudioClip = AudioBytes;   // re-export 自 trellia-assets
PlaySfx { clip: Handle<AudioClip>, volume: f32 }
fn handle(&mut self, req: AudioRequest, clips: &Assets<AudioClip>);
```

`Handle` / `Assets` / `AudioBytes` 定义在 `trellia-assets`（SPEC.md 5.1 / 5.2）。严格按 R2，
`trellia-audio` 无法写出第 7 章规定的任何一个公共签名。两种解读中只有“audio 可依赖 assets”
能保持 SPEC.md 第 5、7 章的公共 API 形状不变，因此这不是需要人类裁决的“API 形状分歧”。

另外 SPEC.md 未明确以下 crate 的允许依赖：`trellia-devtools`、门面 `trellia`、`examples/*`。

## 决策

`scripts/check_deps.sh` 的允许表如下（“通用”= serde / ron / hecs / glam / rand / rand_pcg / log / thiserror）：

| crate | 允许的直接依赖 |
|---|---|
| trellia-core | hecs / glam / rand / rand_pcg / serde / log / thiserror（R1 原文） |
| trellia-assets | core + image + 通用 |
| trellia-gfx | core + assets + 通用 |
| trellia-audio | core + **assets** + 通用（本 ADR 放宽 R2） |
| trellia-scene | core + egui + 通用 |
| trellia-devtools | 除自身外的全部框架 crate + egui + 通用（禁止后端，R5） |
| trellia-egui | 框架 crate + 通用 + egui / eframe / rodio / image / env_logger / anyhow |
| trellia-headless | 框架 crate + 通用 + egui / image / env_logger / anyhow |
| trellia（门面） | 全部框架 crate + 两个后端 + 通用 + egui / anyhow |
| examples/* | 任意 trellia crate + 通用 + egui / anyhow / env_logger |

要点：

1. `trellia-audio` 允许依赖 `trellia-assets`，与 `trellia-gfx` 同等地位；assets / gfx / audio / scene
   之间的其他依赖仍禁止。
2. 门面 `trellia` 必须提供 `run_egui` / `run_headless`（SPEC.md 11.1），因此位于后端之上，
   不受 R5 约束；R5 只约束 core / assets / gfx / audio / scene / devtools。
3. `trellia-devtools` 需要读取 `DrawList` 命令数与场景栈名称（SPEC.md 9），允许依赖 gfx / scene。
4. 只检查 `[dependencies]` 与 `[build-dependencies]`，不检查 `[dev-dependencies]`。

## 后果

- `trellia-audio` 的 API 与 SPEC.md 7 完全一致。
- `check_deps.sh` 在 M0 之后只读；若后续需要新的依赖边，必须新 ADR 并向人类说明。

## 替代方案

1. 把 `Handle` / `Assets` 移入 `trellia-core`：改变 SPEC.md 5.1 规定的公共 API 位置，且让 core
   承担资产职责，否决。
2. `trellia-audio` 自定义一套 `AudioHandle`：与 SPEC.md 7 的签名不符，且与 `Assets<T>` 重复，否决。
