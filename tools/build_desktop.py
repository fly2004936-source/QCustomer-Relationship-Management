#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""一键构建 BCRM 桌面端，并把交付物汇总到 Application/。

# 流水线

    ① 校验编译期内嵌资源（说明/ 的 DDL 与元数据是否一致）
    ② 前端构建      frontend/  --vite build-->  frontend/dist/
    ③ 便携单文件 exe cargo build --release      --> launcher/target/release/bcrm-desktop.exe
    ④ NSIS 安装包    cargo tauri build           --> launcher/target/release/bundle/nsis/*.exe
    ⑤ 汇总交付       Application/（exe + 安装包 + 修复启动.bat + 使用说明.txt）
    ⑥ 统一编码       修复启动.bat -> GBK(cp936)；使用说明.txt -> UTF-8 with BOM

# 顺序为什么不能换

· ② 必须在 ③ 之前：前端 dist 是被 `include_dir`/embed 进 exe 的，
  先编 exe 再改前端等于白编（`launcher/build.rs` 里 rerun-if-changed 只是兜底提示）。
· ③ 必须在 ④ 之前：`cargo tauri build` 内部也会跑一次 release 编译，
  先手动编一次能让缓存命中，也便于把「便携版」单独交付出去。
· ① 必须最先：schema.sql / schema.json 是 `include_str!` 的编译期输入，
  缺了后端直接编译失败（os error 3）。

# 用法

    python tools/build_desktop.py                 # 全流程
    python tools/build_desktop.py --no-installer  # 只要便携 exe（快，不依赖 NSIS）
    python tools/build_desktop.py --no-frontend   # 前端没改时跳过（省时间）
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FRONTEND = ROOT / "frontend"
LAUNCHER = ROOT / "launcher"
OUT_DIR = ROOT / "Application"

PORTABLE_EXE = LAUNCHER / "target" / "release" / "bcrm-desktop.exe"
NSIS_DIR = LAUNCHER / "target" / "release" / "bundle" / "nsis"

# 交付时一起带上的两个辅助文件。
#
# ⚠️ 权威源放在 tools/delivery/，**不要只维护 Application/ 里的那份**：
# Application/ 是纯产物目录，随时可能被重建/清空；辅助文件放里面等于孤本，
# 丢一次就得重写（本仓库已经踩过这个坑 —— 两个文件连同目录一起消失过）。
DELIVERY_SRC = ROOT / "tools" / "delivery"
ASSIST_FILES = ["修复启动.bat", "使用说明.txt"]


def hr(title: str) -> None:
    print()
    print("=" * 66)
    print(f"  {title}")
    print("=" * 66)


def run(cmd: list[str], cwd: Path | None = None, label: str = "") -> None:
    shown = " ".join(str(c) for c in cmd)
    print(f"[RUN] {label or shown}")
    if cwd:
        print(f"      cwd = {cwd}")
    t0 = time.time()
    # 直接继承 stdout/stderr：编译输出很长，但用户需要看到进度与报错
    proc = subprocess.run([str(c) for c in cmd], cwd=str(cwd) if cwd else None)
    dt = time.time() - t0
    if proc.returncode != 0:
        raise SystemExit(f"[FAIL] 步骤失败（退出码 {proc.returncode}）：{label or shown}")
    print(f"[OK]  {label or shown}  （{dt:.1f}s）")


def find_node() -> str:
    """定位 node：优先 PATH，其次 WorkBuddy 托管运行时。"""
    exe = shutil.which("node")
    if exe:
        return exe
    local = os.environ.get("LOCALAPPDATA", "")
    candidates = [
        Path(local).parent / ".workbuddy" / "binaries" / "node" / "versions",
        Path.home() / ".workbuddy" / "binaries" / "node" / "versions",
    ]
    for base in candidates:
        if base.is_dir():
            found = sorted(base.glob("*/node.exe"))
            if found:
                return str(found[-1])
    raise SystemExit("[FAIL] 找不到 node，请先安装 Node.js 或把 node 加进 PATH")


def find_cargo() -> str:
    exe = shutil.which("cargo")
    if exe:
        return exe
    fallback = Path.home() / ".cargo" / "bin" / "cargo.exe"
    if fallback.is_file():
        return str(fallback)
    raise SystemExit("[FAIL] 找不到 cargo，请先安装 Rust 工具链")


def check_schema_resources() -> None:
    run(
        [sys.executable, ROOT / "tools" / "build_schema_resources.py"],
        label="校验/生成编译期内嵌资源（schema.sql / schema.json）",
    )


def build_frontend(node: str) -> None:
    vite = FRONTEND / "node_modules" / "vite" / "bin" / "vite.js"
    if not vite.is_file():
        raise SystemExit(
            f"[FAIL] 找不到 {vite}\n       先在 frontend/ 下执行 npm install"
        )
    run([node, vite, "build"], cwd=FRONTEND, label="前端构建（vite build）")
    dist = FRONTEND / "dist" / "index.html"
    if not dist.is_file():
        raise SystemExit("[FAIL] 前端构建没有产出 dist/index.html")


def build_portable(cargo: str) -> None:
    assert_schema_embedded()
    run(
        [cargo, "build", "--release", "--manifest-path", LAUNCHER / "Cargo.toml"],
        cwd=ROOT,
        label="编译便携单文件 exe（cargo build --release）",
    )
    if not PORTABLE_EXE.is_file():
        raise SystemExit(f"[FAIL] 没有产出 {PORTABLE_EXE}")


def assert_schema_embedded() -> None:
    """编译前确认内嵌资源存在 —— 缺了会以 `os error 3` 的形式在 rustc 里报出来，
    那条报错很难一眼看出是资源目录丢了，这里提前拦一道。"""
    missing = [
        p.name
        for p in [
            ROOT / "backend" / "resources" / "schema.sql",
            ROOT / "backend" / "resources" / "schema.json",
        ]
        if not p.is_file()
    ]
    if missing:
        raise SystemExit(
            f"[FAIL] backend/resources/ 缺少 {missing}\n"
            "       这是编译期内嵌资源，请先跑 tools/build_schema_resources.py 生成"
        )


def build_installer(cargo: str) -> Path | None:
    # ⚠️ `cargo tauri build` 必须在 launcher 目录里执行，**不能带 --manifest-path**
    # （tauri-cli 会自己找当前目录下的 tauri.conf.json，带了反而找错位置）
    run([cargo, "tauri", "build"], cwd=LAUNCHER, label="打包 NSIS 安装包（cargo tauri build）")
    setups = sorted(NSIS_DIR.glob("*.exe"), key=lambda p: p.stat().st_mtime)
    if not setups:
        raise SystemExit(f"[FAIL] 没有在 {NSIS_DIR} 找到安装包")
    return setups[-1]


def collect(setup: Path | None) -> list[tuple[str, int]]:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    collected: list[tuple[str, int]] = []

    def put(src: Path) -> None:
        if not src.is_file():
            print(f"[WARN] 源文件不存在，跳过：{src}")
            return
        dst = OUT_DIR / src.name
        # ⚠️ 辅助文件（修复启动.bat / 使用说明.txt）本来就放在 Application/ 里，
        # 「源 == 目标」时 shutil.copy2 会以 WinError 32 失败（自己占用自己），
        # 所以这里必须先判等而不是无脑复制。
        if src.resolve() == dst.resolve():
            collected.append((dst.name, dst.stat().st_size))
            return
        shutil.copy2(src, dst)
        collected.append((dst.name, dst.stat().st_size))

    put(PORTABLE_EXE)
    if setup:
        put(setup)
    for name in ASSIST_FILES:
        # 优先 tools/delivery/（权威源），退回仓库根目录（历史位置，兼容旧布局）
        for src_root in (DELIVERY_SRC, ROOT):
            cand = src_root / name
            if cand.is_file():
                put(cand)
                break
        else:
            print(f"[WARN] 交付辅助文件缺失：{name}")
            print(f"       应当在 {DELIVERY_SRC} 下维护一份权威副本")

    return collected


def normalize_delivery() -> None:
    """把交付目录里的文本文件转成 Windows 端能正确消费的编码。

    这一步不能省：`修复启动.bat` 里有大量中文，若以 UTF-8 落盘，
    `cmd.exe` 按 GBK 解码会变成乱码，连标签跳转都可能失败；
    而这个问题只有在中英文混排的客户机上才会暴露。
    放在流水线里跑 = 不可能忘记。
    """
    run(
        [sys.executable, ROOT / "tools" / "normalize_delivery_files.py", "--dir", OUT_DIR],
        label="统一交付文件编码（bat->GBK / txt->UTF-8 BOM）",
    )


def main() -> int:
    # 子进程（cargo / vite / makensis）是直接继承 stdout 写的，而本进程的 print 默认块缓冲，
    # 重定向到文件时会出现「步骤标题跑到上一个步骤输出后面」的错乱顺序。改成行缓冲。
    try:
        sys.stdout.reconfigure(line_buffering=True)  # type: ignore[union-attr]
    except Exception:
        pass

    ap = argparse.ArgumentParser(description="一键构建 BCRM 桌面端")
    ap.add_argument("--no-frontend", action="store_true", help="跳过前端构建")
    ap.add_argument("--no-installer", action="store_true", help="跳过 NSIS 安装包")
    ap.add_argument(
        "--collect-only",
        action="store_true",
        help="不编译，只把已有产物重新汇总到 Application/",
    )
    args = ap.parse_args()

    t0 = time.time()
    cargo = find_cargo()

    if args.collect_only:
        hr("① 校验编译期内嵌资源")
        check_schema_resources()
        hr("⑤ 汇总交付物到 Application/（--collect-only）")
        setups = sorted(NSIS_DIR.glob("*.exe"), key=lambda p: p.stat().st_mtime)
        items = collect(setups[-1] if setups else None)
        for name, size in items:
            print(f"  {name:<48} {size / 1024 / 1024:>8.2f} MB")
        hr("⑥ 统一交付文件编码")
        normalize_delivery()
        # 编码转换会改字节数，重新读一遍真实体积
        print()
        for name, _ in items:
            p = OUT_DIR / name
            if p.is_file():
                print(f"  {name:<48} {p.stat().st_size / 1024 / 1024:>8.2f} MB")
        hr(f"完成，用时 {time.time() - t0:.1f}s")
        print(f"交付目录：{OUT_DIR}")
        return 0

    hr("① 校验编译期内嵌资源")
    check_schema_resources()

    if args.no_frontend:
        hr("② 前端构建（已跳过 --no-frontend）")
        if not (FRONTEND / "dist" / "index.html").is_file():
            raise SystemExit("[FAIL] dist 不存在，不能跳过前端构建")
    else:
        hr("② 前端构建")
        build_frontend(find_node())

    hr("③ 编译便携单文件 exe")
    build_portable(cargo)

    setup: Path | None = None
    if args.no_installer:
        hr("④ NSIS 安装包（已跳过 --no-installer）")
    else:
        hr("④ 打包 NSIS 安装包")
        setup = build_installer(cargo)

    hr("⑤ 汇总交付物到 Application/")
    items = collect(setup)
    for name, size in items:
        print(f"  {name:<48} {size / 1024 / 1024:>8.2f} MB")

    hr("⑥ 统一交付文件编码")
    normalize_delivery()

    hr(f"完成，用时 {time.time() - t0:.1f}s")
    print(f"交付目录：{OUT_DIR}")
    print()
    print("交付清单（最终体积）：")
    for p in sorted(OUT_DIR.iterdir()):
        if p.is_file():
            print(f"  {p.name:<48} {p.stat().st_size / 1024 / 1024:>8.2f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
