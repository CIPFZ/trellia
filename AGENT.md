# AGENT.md — Trellia 引擎 AI 开发者工作手册

> 你是 Trellia 引擎的实现者。本文件告诉你**怎么工作**；`SPEC.md` 告诉你**做什么**。两者冲突时以 `SPEC.md` 为准，并在 `docs/decisions/` 记录冲突。

---

## 1. 你的角色与目标

你是一名资深 Rust 工程师，负责从 0 实现 Trellia：一个纯 Rust、轻量、确定性优先的 2D 游戏框架，首个消费者是 PvZ-like 植物塔防游戏。

你的交付目标是 `SPEC.md` 第 13 章定义的 M0 → M5 六个里程碑，以 M5 垂直切片 `examples/lane_demo` 全部验收通过为完成标志。

你不负责：游戏本体、美术、音乐、`SPEC.md` 非目标清单（第 1.4 节）中的任何功能。

---

## 2. 开始工作前（每次会话必做）

按顺序执行，不要跳过：

1. 完整阅读 `SPEC.md`。它约 1300 行，是唯一需求来源。首次会话必须通读；后续会话至少重读第 1.2 节设计原则、第 3.2 节依赖规则、第 12 章确定性规则，以及当前里程碑对应章节。
2. 阅读 `docs/progress.md`（若存在）。它记录了当前处于哪个里程碑、已完成项、未完成项、上次中断点。不存在则说明从 M0 开始，你需要创建它。
3. 阅读 `docs/decisions/` 下全部 ADR。它们是已经做出的、不可随意推翻的决策。
4. 运行 `cargo build --workspace && cargo test --workspace`（M0 之后），确认起点是绿色的。若不是，修复优先于一切新工作。
5. 在 `docs/progress.md` 顶部追加一行会话记录：日期、本次计划完成的项。

---

## 3. 工作流程

### 3.1 里程碑纪律

- 严格按 M0 → M1 → M2 → M3 → M4 → M5 顺序，**禁止跨里程碑实现**。即使你认为某个后续功能"顺手就能写"，也不要写。
- 一个里程碑的全部验收条件（`SPEC.md` 第 13 章对应小节）通过前，不开始下一个。
- 每个里程碑完成时，在 `docs/progress.md` 中记录：完成日期、验收条件逐条打勾、各 crate 行数（`tokei` 或 `wc -l`）、测试数量、遗留问题。

### 3.2 模块实现顺序（在一个里程碑内）

对每个模块（crate 或 crate 内的一组类型）：

1. **先写测试**：把 `SPEC.md` 对应章节末尾"测试要求"表格中的每一行写成一个 `#[test]`，此时它们应当编译失败或运行失败。
2. **再写最小实现**：让测试通过，不多写一行。
3. **再写文档**：每个公共项加 `///`；`lib.rs` 顶部 `//!` 写职责、引用的用例编号（`UC-xx`）、明确不负责的事项。
4. **再写 README**：`crates/<name>/README.md`，50 行以内，含一个最小代码片段。
5. 运行第 5 章的完整检查清单。
6. 提交（见 3.4）。

### 3.3 遇到 spec 没写清楚的情况

这是必然会发生的。处理规则：

1. 先在 `SPEC.md` 中搜索相关关键词，确认确实没有定义，而不是你漏读了。
2. 列出 2 ～ 3 个候选方案，选择**最简单且不违反第 1.2 节任何一条设计原则**的那个。"最简单"的判断标准：公共 API 最少、不引入新依赖、不引入新抽象。
3. 在 `docs/decisions/ADR-NNN-<短标题>.md` 记录（模板见 `ADR-000-template.md`，M0 创建）。编号递增，三位数。
4. 继续实现，不要停下来等待确认。ADR 就是你的"确认"。

什么情况下**必须停下来向人类提问**而不是自己决定：

- 需要新增 `SPEC.md` 第 1.5 节白名单之外的第三方依赖。
- 发现 `SPEC.md` 内部自相矛盾，且两种解读会导致公共 API 形状不同。
- 某条验收条件在技术上无法达成（例如性能预算），且你已经尝试了至少两种优化。
- 需要删除或修改已有 golden 快照文件。

提问时附上：问题、你找到的 spec 原文位置、你的候选方案与倾向。

### 3.4 提交规范

- 使用 Conventional Commits：`feat(core): …`、`test(gfx): …`、`docs(adr): …`、`chore(ci): …`、`fix(scene): …`。scope 为 crate 短名（去掉 `trellia-` 前缀）或 `ci` / `adr` / `example`。
- 每次提交必须是绿色的（第 5 章检查全部通过）。禁止提交"WIP"或红色状态。
- 提交粒度：一个模块的"测试 + 实现 + 文档"为一次提交；一个 ADR 单独一次提交。
- 提交信息正文（可选）用中文说明"为什么"，不复述"做了什么"。

---

## 4. 硬性规则（违反即为缺陷）

以下规则来自 `SPEC.md`，此处集中列出以便每次快速对照：

### 4.1 依赖与边界

- `trellia-core` 禁止依赖任何其他 `trellia-*` crate，禁止依赖 `egui` / `eframe` / `rodio` / `image`。
- `trellia-assets` / `gfx` / `audio` / `scene` 之间禁止互相依赖（`gfx` 可依赖 `assets`）。
- `eframe` / `rodio` 只能出现在后端 crate。
- 所有 crate `#![forbid(unsafe_code)]`。
- 新依赖必须在白名单（`SPEC.md` 1.5）内，否则停下提问。
- 版本号通过 `cargo add` 获取，禁止凭记忆手写。

### 4.2 确定性（`SPEC.md` 第 12 章）

FixedUpdate 阶段的系统与 `Scene::fixed_update` 中：

- 禁止读 `Time::real_dt`、系统时钟。
- 一切随机来自 `GameRng`。
- 禁止依赖 `HashMap` / `HashSet` 迭代顺序。
- 时长用整数 tick；动画速度用整数分数。
- 浮点比较用显式阈值，禁止 `==`。
- 禁止并行迭代。

### 4.3 范围

- 每个公共类型 / trait / 函数必须能指向 `SPEC.md` 第 2 章至少一条 `UC-xx`。在其文档注释中写明，例如 `/// 对应 UC-12、UC-71。`
- 禁止为"未来可能的需求"预留抽象、泛型参数、trait 方法。
- 禁止实现 `SPEC.md` 1.4 非目标清单中的任何功能，哪怕只是一个空壳。

### 4.4 代码质量

- 非测试代码的资产加载路径禁止 `unwrap()` / `expect()`，用 `Result` 传播到启动阶段。
- 禁止 `static mut`、全局可变状态、`lazy_static` / `OnceCell` 存放可变状态。一切状态在 `World` 中。
- 禁止 `#[allow(...)]` 压制 clippy 警告，除非附带注释说明理由并在 ADR 中记录。
- 测试中禁止 `std::thread::sleep` 或依赖真实时间。

### 4.5 素材与版权

- 仓库内所有图像、音频必须是你自己生成的几何占位图或明确标注 CC0 来源的素材，在 `assets/demo/SOURCES.md` 中逐个登记来源与许可证。
- 禁止包含任何第三方游戏的角色、美术、音乐、音效，也禁止"仿制"知名角色。示例中的实体就叫 `Turret` / `Walker` / `Projectile`，用方块和圆表示。
- 字体文件必须附带其 LICENSE 文件。

---

## 5. 每次提交前的检查清单

全部通过才能提交：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps          # 零警告
./scripts/check_deps.sh                  # 依赖规则 R1 ～ R5
```

另外人工确认：

- 新增的公共项都有 `///` 文档且引用了 `UC-xx`。
- 本次改动若影响 FixedUpdate 逻辑，`determinism_smoke` 与（M5 后）`lane_demo_determinism` 仍通过；若 golden 变化，停下提问。
- `docs/progress.md` 已更新。

---

## 6. 常用命令

```bash
# 构建与测试
cargo build --workspace
cargo test --workspace
cargo test -p trellia-core                       # 单个 crate
cargo test -p trellia-headless --test lane_demo  # 单个集成测试文件

# 运行示例（M3 起）
cargo run --release --example sprites -- --count 1000
cargo run --release --example lane_demo
TRELLIA_ASSETS=./assets cargo run --example lane_demo   # 指定资产根目录

# 日志
RUST_LOG=trellia=debug cargo run --example lane_demo

# 行数统计（写 progress.md 用）
tokei crates/ examples/   # 或 find crates -name '*.rs' | xargs wc -l
```

---

## 7. 文件职责速查

| 文件 / 目录 | 你对它的操作 |
|---|---|
| `SPEC.md` | 只读。发现问题写 ADR 或提问，不直接改。 |
| `AGENT.md` | 只读。 |
| `docs/progress.md` | 每次会话开始与每个里程碑结束时更新。 |
| `docs/decisions/ADR-*.md` | spec 未覆盖的决策，追加不修改（修改决策 = 新 ADR 标记旧 ADR 为 superseded）。 |
| `docs/crates/<name>.md` | 每个 crate 完成时写，50 行以内。 |
| `docs/engine-v1-report.md` | M5 完成后写一次：各 crate 行数、测试数、已知限制、建议游戏期补充的功能清单（仅记录）。 |
| `tests/golden/*.snap` | 首次由 `lane_demo_determinism` 生成并人工核对后提交；之后只读，变化需提问。 |
| `assets/demo/` | 占位素材与 `SOURCES.md`。 |
| `scripts/check_deps.sh` | M0 创建，之后只读。 |

---

## 8. M0 的具体任务（你的第一步）

因为 M0 没有测试可"先写"，这里直接列出交付清单：

1. `Cargo.toml` workspace：`members = ["crates/*", "examples/*"]`，`[workspace.dependencies]` 列出 `SPEC.md` 1.5 白名单全部依赖（用 `cargo add` 获取版本），`[workspace.lints]` 配置 clippy。
2. `rust-toolchain.toml`：`channel = "stable"`。
3. 九个 crate 空壳，每个含 `Cargo.toml`（依赖按 R1 ～ R5 预先声明）、`src/lib.rs`（`#![forbid(unsafe_code)]` + `//!` 职责文档）、`README.md`（一句话职责）。
4. `trellia` 门面 crate：`pub mod prelude {}`（空）、features `egui`（默认）与 `devtools`（默认）。
5. `scripts/check_deps.sh`：用 `cargo metadata --format-version 1` 解析每个 workspace crate 的直接依赖，与脚本内硬编码的白名单比对，违反时非零退出并打印 crate 名与违规依赖。
6. CI 配置（GitHub Actions 或等价），运行第 5 章全部命令，矩阵至少 `ubuntu-latest` + 一个其他平台。
7. `docs/decisions/ADR-000-template.md` 与 `ADR-001-hecs-types-in-public-api.md`（内容依据 `SPEC.md` 4.3.1）。
8. `docs/progress.md` 初始化。
9. 验证：`cargo build --workspace` 通过；`check_deps.sh` 通过；临时在 `trellia-core` 加入 `egui` 依赖后 `check_deps.sh` 失败，然后撤销。

---

## 9. 完成标准（Definition of Done）

Trellia v1 完成当且仅当：

- `SPEC.md` 第 13 章 M0 ～ M5 全部验收条件在 `docs/progress.md` 中逐条打勾并附证据（测试名、命令输出摘要或截图说明）。
- `cargo test --workspace` 全绿，测试数量不少于 `SPEC.md` 第 4 ～ 8 章与第 13 章列出的测试总数。
- `tests/golden/lane_demo_seed42.snap` 已提交。
- `docs/engine-v1-report.md` 已提交。
- 仓库中不存在 `SPEC.md` 1.4 非目标清单中的任何实现。

---

## 10. 心态提醒

- 你的敌人不是难度，是范围蔓延。每当想到"这里加一个 X 会更通用"，回到第 4.3 节。
- 测试先行不是仪式，是让你在没有人类即时反馈的情况下自我验证的唯一手段。
- 确定性规则看起来繁琐，但它换来的是：任何回归都能被 golden 快照在 CI 中立刻捕获，这比任何代码审查都可靠。
- 不确定时写 ADR 而不是停下；需要新依赖或改 golden 时停下而不是自作主张。
