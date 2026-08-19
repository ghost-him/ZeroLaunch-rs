#!/usr/bin/env python3
"""ZeroLaunch 插件打包脚本：构建 Rust 插件并打包为宿主可直接安装的 zip。

用法:
    python package.py                 # cargo build --release 后打包
    python package.py --no-build      # 复用现有构建产物，直接打包
    python package.py --target <triple>   # 交叉编译（产物在 target/<triple>/release/）
    python package.py --out <目录>    # 指定输出目录（默认 ./dist）

无系统 Python 时可用 uv 运行（uv 自动下载托管 Python，无需手动安装）:
    uv run --python 3.12 python package.py

产物: <输出目录>/<plugin-id>-<version>.zip

zip 根目录结构（与宿主安装器约定一致，manifest.toml 必须位于 zip 根）:
    manifest.toml
    bin/<可执行文件>        # 文件名取自 manifest [runtime].command
    ui/...                  # 若存在
    i18n/...                # 若存在
    <icon 文件>             # 若 manifest [icon] path 声明

安装方式: 设置 → 插件管理 → 安装本地插件，选择该 zip；
或手动解压到 %APPDATA%/ZeroLaunch/plugins/<plugin-id>/。
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import zipfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    sys.exit("需要 Python 3.11+（tomllib 为标准库），请升级 Python 后重试。")

ROOT = Path(__file__).resolve().parent

# 宿主 manifest schema 字段名（serde rename 后的 camelCase 键）。
# toml 中写成 snake_case 不会导致解析报错，但字段会被 serde 静默丢弃
# （如 [ui] panel_entry → 前端拿不到 panelEntry → 面板不加载），
# 因此对已知易错字段做显式预检。
SNAKE_TO_CAMEL = {
    "panel_entry": "panelEntry",
    "settings_entry": "settingsEntry",
    "result_item_entry": "resultItemEntry",
}


def check_manifest_field_names(manifest: dict) -> None:
    """预检 manifest 可选段字段名，发现 snake_case 键时给出显式警告。

    宿主 schema 使用 camelCase（panelEntry/settingsEntry/resultItemEntry）；
    snake_case 键会被 serde 静默忽略，属无声失效，必须提前暴露。
    """
    ui = manifest.get("ui") or {}
    for snake, camel in SNAKE_TO_CAMEL.items():
        if snake in ui and camel not in ui:
            print(
                f"警告: manifest [ui] 使用了 {snake!r}，"
                f"宿主 schema 要求 {camel!r}，该字段会被忽略。"
            )


def load_toml(path: Path) -> dict:
    """读取 toml 文件并返回解析结果；文件缺失或格式错误时直接报错退出。"""
    with open(path, "rb") as f:
        return tomllib.load(f)


def build_release(target: str | None) -> None:
    """在插件目录执行 cargo build --release；--target 指定时做交叉编译。"""
    cmd = ["cargo", "build", "--release"]
    if target:
        cmd += ["--target", target]
    print(f"$ {' '.join(cmd)}")
    subprocess.run(cmd, cwd=ROOT, check=True)


def locate_binary(package_name: str, target: str | None) -> Path:
    """返回构建产物可执行文件路径；不存在时给出排查提示并退出。

    本机构建产物在 target/release/，交叉编译在 target/<triple>/release/；
    Windows 目标（本机 Windows 或 cross 目标含 windows）带 .exe 后缀。
    """
    exe_suffix = ".exe" if (os.name == "nt" or (target and "windows" in target)) else ""
    base = (
        ROOT / "target" / "release"
        if target is None
        else ROOT / "target" / target / "release"
    )
    cand = base / (package_name + exe_suffix)
    if cand.is_file():
        return cand
    sys.exit(
        f"未找到构建产物 {cand}，请先执行 `cargo build --release`"
        "（或确认 --target 与实际构建产物一致）。"
    )


def collect_entries(manifest: dict, binary: Path) -> list[tuple[Path, str]]:
    """收集打包条目 (磁盘路径, zip 内相对路径)，全部位于 zip 根，避免公共前缀歧义。

    打包内容：manifest.toml（必需）、bin/<command 文件名>、ui/、i18n/、
    以及 manifest [icon] path 声明的图标文件（若存在）。
    """
    command = manifest.get("runtime", {}).get("command")
    if not command:
        sys.exit("manifest.toml 缺少 [runtime].command 字段，无法确定可执行文件位置。")
    entries: list[tuple[Path, str]] = [(ROOT / "manifest.toml", "manifest.toml")]
    entries.append((binary, f"bin/{Path(command).name}"))
    for sub in ("ui", "i18n"):
        src = ROOT / sub
        if src.is_dir():
            for p in sorted(src.rglob("*")):
                if p.is_file():
                    entries.append((p, f"{sub}/{p.relative_to(src).as_posix()}"))
    icon = (manifest.get("icon") or {}).get("path")
    if icon:
        icon_rel = Path(icon)
        if icon_rel.is_absolute() or icon_rel.name in ("", "..") or icon.startswith(".."):
            print(f"警告: manifest [icon].path 必须是插件目录内的相对路径（当前: {icon!r}），已跳过。")
        elif (ROOT / icon).is_file():
            entries.append((ROOT / icon, icon_rel.as_posix()))
        else:
            print(f"警告: manifest 声明了 icon {icon!r} 但文件不存在，已跳过。")
    return entries


def write_zip(entries: list[tuple[Path, str]], out_path: Path) -> None:
    """将条目写入 zip，统一使用正斜杠路径（zip 规范要求）。"""
    with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for disk_path, arcname in entries:
            zf.write(disk_path, arcname)


def main() -> int:
    """解析命令行参数并执行 构建 → 定位产物 → 打包 → 输出安装提示。"""
    parser = argparse.ArgumentParser(description="ZeroLaunch 插件打包脚本")
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="跳过 cargo build --release，直接打包现有产物",
    )
    parser.add_argument(
        "--target",
        default=None,
        help="cargo 交叉编译目标 triple（如 x86_64-pc-windows-gnu）",
    )
    parser.add_argument("--out", default="dist", help="输出目录（默认 dist/）")
    args = parser.parse_args()

    cargo_toml = load_toml(ROOT / "Cargo.toml")
    manifest = load_toml(ROOT / "manifest.toml")
    check_manifest_field_names(manifest)
    package_name = cargo_toml["package"]["name"]
    plugin_id = manifest["plugin"]["id"]
    plugin_version = manifest["plugin"]["version"]

    if not args.no_build:
        build_release(args.target)

    binary = locate_binary(package_name, args.target)

    command_name = Path(manifest["runtime"]["command"]).name
    if command_name != binary.name:
        print(
            f"警告: manifest [runtime].command 文件名为 {command_name!r}，"
            f"实际构建产物为 {binary.name!r}，将按 command 名打包。"
        )

    entries = collect_entries(manifest, binary)
    out_dir = ROOT / args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{plugin_id}-{plugin_version}.zip"
    write_zip(entries, out_path)
    print(f"打包完成: {out_path}（共 {len(entries)} 个文件）")
    print("安装: 设置 → 插件管理 → 安装本地插件，选择该 zip；")
    print("      或手动解压到 %APPDATA%/ZeroLaunch/plugins/<plugin-id>/。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
