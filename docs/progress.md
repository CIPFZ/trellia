# Trellia 进度记录

## 会话记录（新会话追加在最上方）

- 2026-10-13：首次会话。计划：通读 SPEC.md / AGENT.md，完成 M0 工程骨架，随后按顺序推进 M1 起的里程碑。

## 当前状态

- 当前里程碑：M1 核心
- 上次中断点：M0 完成

## 环境备注

- Linux 构建 `rodio` 需要 ALSA 开发包：TencentOS / Fedora 系 `dnf install alsa-lib-devel`，Debian / Ubuntu 系 `apt-get install libasound2-dev`（CI 已配置）。
- `check_deps.sh` 需要 `python3`（解析 `cargo metadata` 的 JSON）。
- workspace `members` 在 M0 暂为 `["crates/*"]`：cargo 将无匹配的 `examples/*` glob 视为字面路径并报错；M1 新增首个示例时加入 `"examples/*"`。
- 原始图标位于 `asserts/icon/`（目录名疑似笔误），已复制到 SPEC.md 3.3 规定的 `assets/icon/`；`asserts/` 未纳入版本控制，待负责人确认后删除。

## 里程碑

### M0 工程骨架

- 状态：✅ 完成（2026-10-13）
- 交付：
  - [x] workspace `Cargo.toml`（`[workspace.dependencies]` 含 1.5 白名单全部 14 个依赖，版本由 `cargo add` 获取；`[workspace.lints]` 配置 `unsafe_code = forbid`、`missing_docs`、`clippy::must_use_candidate`、`clippy::missing_errors_doc`）
  - [x] `rust-toolchain.toml`（`channel = "stable"`）
  - [x] 九个 crate 空壳（`Cargo.toml` 依赖按 R1 ～ R5 预先声明、`lib.rs` 含 `#![forbid(unsafe_code)]` 与 `//!` 职责文档、`README.md`）
  - [x] 门面 `trellia`：`pub mod prelude {}`，features `egui` / `devtools`（均默认开启）
  - [x] `scripts/check_deps.sh`（`cargo metadata` + 硬编码允许表）
  - [x] CI：`.github/workflows/ci.yml`，矩阵 ubuntu / windows / macos
  - [x] `ADR-000-template.md`、`ADR-001-hecs-types-in-public-api.md`、`ADR-002-dependency-rules-clarification.md`
  - [x] `docs/progress.md`
- 验收：
  - [x] `cargo build --workspace` 通过
  - [x] `./scripts/check_deps.sh` 对空壳通过：`check_deps: 9 个 crate 全部符合依赖规则 R1 ～ R5`，exit 0
  - [x] 临时在 `trellia-core` 加入 `egui.workspace = true` 后失败：`trellia-core -> egui  违反 R1`，exit 1；撤销后恢复 exit 0
  - [x] fmt / clippy `-D warnings` / test / doc 零警告
- 行数：每个 crate 的 `lib.rs` 为 10 ～ 15 行文档，暂无代码
- 测试数：0
- 遗留问题：`examples/*` 尚未加入 workspace members（见环境备注）
