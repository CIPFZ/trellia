#!/usr/bin/env bash
# 依赖规则检查（SPEC.md 3.2 R1 ～ R5 + 1.5 白名单；细化见 docs/decisions/ADR-002）。
#
# 用 `cargo metadata` 解析每个 workspace crate 的直接依赖（normal + build，不含 dev），
# 与下方硬编码的允许表比对。任一违规即打印 crate 名与违规依赖并以非零状态退出。
#
# 用法：./scripts/check_deps.sh   （任意目录执行均可）
set -euo pipefail

cd "$(dirname "$0")/.."

command -v python3 >/dev/null || { echo "check_deps: 需要 python3" >&2; exit 2; }

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT
cargo metadata --format-version 1 --no-deps >"$metadata"

python3 - "$metadata" <<'PY'
import json
import sys

# 框架层通用第三方依赖（1.5 白名单中除 egui / eframe / rodio / image / env_logger / anyhow 外的部分）
COMMON = {"serde", "ron", "hecs", "glam", "rand", "rand_pcg", "log", "thiserror"}
FRAMEWORK = {"trellia-core", "trellia-assets", "trellia-gfx", "trellia-audio",
             "trellia-scene", "trellia-devtools"}
BACKENDS = {"trellia-egui", "trellia-headless"}

# crate -> (允许的依赖集合, 规则说明)
ALLOWED = {
    # R1：core 只允许 hecs / glam / rand / rand_pcg / serde / log / thiserror
    "trellia-core": ({"hecs", "glam", "rand", "rand_pcg", "serde", "log", "thiserror"}, "R1"),
    # R2 + R3：assets 只依赖 core；image 仅用于解码
    "trellia-assets": ({"trellia-core", "image"} | COMMON, "R2/R3"),
    # R2：gfx 额外允许依赖 assets
    "trellia-gfx": ({"trellia-core", "trellia-assets"} | COMMON, "R2"),
    # R2 + ADR-002：audio 的公共 API（Handle<AudioClip>）要求依赖 assets
    "trellia-audio": ({"trellia-core", "trellia-assets"} | COMMON, "R2/ADR-002"),
    # R2 + R3：scene 允许 egui（Scene::ui 钩子）
    "trellia-scene": ({"trellia-core", "egui"} | COMMON, "R2/R3"),
    # R3 + R5：devtools 允许 egui 与其他框架 crate，禁止后端
    "trellia-devtools": ((FRAMEWORK - {"trellia-devtools"}) | {"egui"} | COMMON, "R3/R5"),
    # 后端：框架 crate + 窗口 / 音频 / 解码库
    "trellia-egui": (FRAMEWORK | COMMON | {"egui", "eframe", "rodio", "image", "env_logger", "anyhow"}, "R3"),
    "trellia-headless": (FRAMEWORK | COMMON | {"egui", "image", "env_logger", "anyhow"}, "R3"),
    # 门面：位于后端之上，允许依赖全部 trellia crate（ADR-002）
    "trellia": (FRAMEWORK | BACKENDS | COMMON | {"egui", "anyhow"}, "ADR-002"),
}
# examples/* 下的示例：任意 trellia crate + 通用依赖 + egui / anyhow / env_logger（不允许 eframe / rodio / image）
EXAMPLE_ALLOWED = ({"trellia"} | FRAMEWORK | BACKENDS | COMMON | {"egui", "anyhow", "env_logger"}, "ADR-002")

with open(sys.argv[1], encoding="utf-8") as f:
    meta = json.load(f)

violations = []
checked = 0
for pkg in meta["packages"]:
    name = pkg["name"]
    manifest = pkg["manifest_path"].replace("\\", "/")
    if name in ALLOWED:
        allowed, rule = ALLOWED[name]
    elif "/examples/" in manifest:
        allowed, rule = EXAMPLE_ALLOWED
    else:
        violations.append(f"{name}: 未登记的 workspace crate（请在 check_deps.sh 中登记其规则）")
        continue
    checked += 1
    for dep in pkg["dependencies"]:
        if dep["kind"] == "dev":
            continue
        if dep["name"] not in allowed:
            violations.append(f"{name} -> {dep['name']}  违反 {rule}")

if violations:
    print("check_deps: 发现依赖规则违规：", file=sys.stderr)
    for v in violations:
        print(f"  {v}", file=sys.stderr)
    sys.exit(1)
print(f"check_deps: {checked} 个 crate 全部符合依赖规则 R1 ～ R5")
PY
