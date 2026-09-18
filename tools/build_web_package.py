#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""把 Web 版打包成最小可交付源码包 `bcrm-web-3.1.zip`。

# 打包原则

  · **只带人写的文件** —— 排除 `backend/target/`、`frontend/node_modules/`、`frontend/dist/`
    这三个目录都是可再生的构建产物，带上只会让包从 1.4 MB 涨到几百 MB。
  · **前端排除调试脚本** —— `frontend/_*.mjs`、`_*.txt`、`run.mjs` 都是开发期临时产物
    （其中 `_inline_check.mjs` 一个就有 112 KB），不属于交付内容。
  · **白名单而非黑名单** —— 逐个目录显式枚举，避免将来新增的临时目录被顺手打进去。

# 用法

    python tools/build_web_package.py
    python tools/build_web_package.py --out D:\\somewhere
"""

from __future__ import annotations

import argparse
import os
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

PKG_NAME = "bcrm-web-3.1"
ZIP_NAME = PKG_NAME + ".zip"
DOC_SUBDIR = "\u6587\u6863"                      # 文档
DOC_PREFIXES = ("02-", "03-", "04-", "05-", "06-", "07-", "08-", "10-", "12-")

SCRIPTS = ("\u542f\u52a8Web\u7248.bat",           # 启动Web版.bat
           "\u542f\u52a8Web\u670d\u52a1.bat",     # 启动Web服务.bat
           "\u542f\u52a8Web\u670d\u52a1.ps1")     # 启动Web服务.ps1

SKIP_DIRS = {"target", "node_modules", "dist", ".git"}

README = """# BCRM 客户管理系统 · Web 版 v3.1

单进程 Web 应用：**一个 exe 同时提供接口和页面**，浏览器访问即可用。
目标机零依赖 —— 不需要 WebView2，也不需要额外装运行时。

---

## 30 秒上手

```bat
:: 1) 构建前端
cd frontend
npm install
npm run build

:: 2) 编译后端
cd ..
cargo build --release --manifest-path backend\\Cargo.toml

:: 3) 双击根目录的「启动Web版.bat」
```

浏览器会自动打开。**关掉那个窗口 = 停止服务。**

详细步骤与排错见 → `文档\\12-启动与环境说明.md`

---

## 它长什么样

- **工作台** —— 数字桌面：今日待办、灵感速记、专注计时、快捷入口、每日资讯
- **组织网络图** —— 客户 / 人员 / 项目 / 竞争对手的关系网，可拖拽
- **数据分析驾驶舱** —— KPI 带 + 12 个面板，图表全部手绘 SVG
- **数据管理** —— 62 张表的增删改查，支持全字段单独或联合查询
- 另有 11 套界面风格可自由切换

---

## 目录

| 路径 | 说明 |
| --- | --- |
| `启动Web版.bat` | 一键启动 |
| `启动Web服务.bat` / `.ps1` | 另一种启动方式（自动编译，开发用） |
| `backend\\` | Rust 后端源码（axum + SQLite） |
| `frontend\\` | Vue 3 + Vite 前端源码 |
| `文档\\` | 说明文档，见下表 |

## 文档导航

| 想了解 | 看这篇 |
| --- | --- |
| **怎么装、怎么启动、出问题怎么办** | `文档\\12-启动与环境说明.md` |
| **数据库说明** · 表怎么分组、关键业务口径 | `文档\\06-数据库架构解析.md` |
| **数据库说明** · 完整建库 DDL | `文档\\08-客户管理系统SQLite建库语句.md` |
| 这个系统能做什么 | `文档\\07-项目功能介绍.md` |
| 接口怎么调（62 张表增删改查） | `文档\\03-前后端接口文档.md` |
| 每层用了什么、为什么这么选 | `文档\\02-技术栈说明.md` |
| 后端设计（动态查询引擎） | `文档\\04-后端设计文档.md` |
| 前端设计（双数据通道、条件筛选器） | `文档\\05-前端设计文档.md` |
| 双击没反应 / 被安全软件拦截 | `文档\\10-终端管控拦截：机制与解决方案.md` |

---

## 技术栈

| 层 | 用了什么 |
| --- | --- |
| 后端 | Rust · axum 0.8 · rusqlite（bundled SQLite） · tower-http |
| 前端 | Vue 3.5 · Vite 5 · 无 UI 框架、无图表库（手绘 SVG） |
| 数据库 | SQLite 单文件，62 张表 / 32 个索引 |

---

## 安全须知

**后端 API 当前没有任何鉴权。** 任何能访问到该端口的人都能读写全部数据。

默认只监听 `127.0.0.1`（仅本机）。开放到局域网之前，请先读
`文档\\12-启动与环境说明.md` 第六节。
"""


def collect() -> tuple[list[tuple[Path, str]], list[str]]:
    """返回 (待打包项, 跳过说明)。每项 = (源文件绝对路径, 包内相对路径)。"""
    items: list[tuple[Path, str]] = []
    notes: list[str] = []

    be = ROOT / "backend"
    for rel in ("Cargo.toml", "Cargo.lock"):
        items.append((be / rel, "backend/" + rel))
    for rel in ("resources/schema.sql", "resources/schema.json"):
        items.append((be / rel, "backend/" + rel))
    for dirpath, dirnames, filenames in os.walk(be / "src"):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for fn in sorted(filenames):
            if fn.endswith(".rs"):
                fp = Path(dirpath) / fn
                items.append((fp, "backend/" + fp.relative_to(be).as_posix()))

    fe = ROOT / "frontend"
    for rel in ("package.json", "package-lock.json", "index.html", "vite.config.js"):
        items.append((fe / rel, "frontend/" + rel))
    for dirpath, dirnames, filenames in os.walk(fe / "src"):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for fn in sorted(filenames):
            fp = Path(dirpath) / fn
            items.append((fp, "frontend/" + fp.relative_to(fe).as_posix()))

    # 明确记录被排除的前端调试文件，避免"是不是漏了"的疑问
    for fn in sorted(os.listdir(fe)):
        fp = fe / fn
        if fp.is_file() and (fn.startswith("_") or fn == "run.mjs"):
            notes.append("SKIP(debug): frontend/" + fn)

    for name in SCRIPTS:
        items.append((ROOT / name, name))

    docs_dir = ROOT / "\u8bf4\u660e"
    for fn in sorted(os.listdir(docs_dir)):
        if fn.endswith(".md") and fn.startswith(DOC_PREFIXES):
            items.append((docs_dir / fn, DOC_SUBDIR + "/" + fn))

    return items, notes


def main() -> int:
    ap = argparse.ArgumentParser(description="打包 Web 版最小可交付源码")
    ap.add_argument("--out", default=str(ROOT / "Application"), help="输出目录")
    args = ap.parse_args()

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    zip_path = out_dir / ZIP_NAME

    items, notes = collect()

    missing = [str(src) for src, _ in items if not src.is_file()]
    if missing:
        print("[FAIL] 下列文件不存在，拒绝产出不完整的包：")
        for m in missing:
            print("   " + m)
        return 1

    if zip_path.exists():
        zip_path.unlink()

    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for src, rel in items:
            z.write(src, PKG_NAME + "/" + rel)
        z.writestr(PKG_NAME + "/README.md", README.encode("utf-8"))

    # ---- 校验 ----
    bad = []
    with zipfile.ZipFile(zip_path) as z:
        names = z.namelist()
        raw_bytes = {n: z.read(n) for n in names}

    for n in names:
        low = n.lower()
        for skip in ("/target/", "/node_modules/", "/dist/", "/.git/"):
            if skip in low:
                bad.append("含有应排除的路径: " + n)
        if n.endswith(".mjs"):
            bad.append("含有前端调试脚本: " + n)

    # 启动 bat 必须是 GBK（cmd 按当前代码页读）且不含坏字符
    for name in SCRIPTS:
        if not name.lower().endswith(".bat"):
            continue
        key = PKG_NAME + "/" + name
        if key in raw_bytes:
            data = raw_bytes[key]
            try:
                data.decode("gbk")
                enc = "gbk"
            except UnicodeDecodeError:
                enc = "NOT-GBK"
                bad.append("bat 不是 GBK: " + name)
            qm = data.count(b"?")
            if qm:
                bad.append("bat 含 %d 个半角 ? （可能有权重字符被静默替换）: %s" % (qm, name))
            print("bat  %s: enc=%s, %d bytes, qmarks=%d" % (name, enc, len(data), qm))

    print("-" * 66)
    print("包        : %s" % zip_path)
    print("大小      : %d bytes (%.2f MB)" % (zip_path.stat().st_size,
                                              zip_path.stat().st_size / 1024 / 1024))
    print("条目数    : %d" % len(names))
    print("源文件合计: %d bytes" % sum(p.stat().st_size for p, _ in items))
    print()

    groups = {}
    for n in names:
        parts = n.split("/")
        key = parts[1] if len(parts) > 2 else "(root)"
        groups[key] = groups.get(key, 0) + 1
    print("分组:")
    for k in sorted(groups):
        print("   %-14s %d 个" % (k, groups[k]))

    print()
    print("顶层文件:")
    for n in sorted(names):
        if len(n.split("/")) == 2:
            print("   " + n.split("/")[1])

    if notes:
        print()
        print("跳过的前端调试文件:")
        for x in notes:
            print("   " + x)

    print()
    if bad:
        print("[FAIL] 校验未通过:")
        for b in bad:
            print("   " + b)
        return 1
    print("[OK] 校验通过（无构建产物、无调试脚本、bat 编码正确）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
