#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""把交付目录里的文本文件转成 **Windows 端能正确消费** 的编码。

# 为什么需要它

交付目录放在中文路径下、内容也是中文，编码错了就是「花屏」或「满屏问号」，
而且往往在客户机器上才暴露：

  · `*.bat` —— `cmd.exe` 按当前代码页（简体中文系统是 936/GBK）读取批处理文件。
    UTF-8 的中文会被按 GBK 解码成乱码，连标签跳转都可能失败。
    → 统一写成 **GBK(cp936)**。

  · `*.txt` —— 给用户直接双击看的，写成 **UTF-8 带 BOM** 最保险：
    记事本/写字板/Word 都能认出 BOM，不会当成 ANSI 花屏。

# 踩过的坑（本脚本就是为了不再踩）

PowerShell 里 `[Text.UTF8Encoding]::new($false)` 读文件时**默认不抛错**，
遇到非法 UTF-8 字节会静默替换成 U+FFFD；再写回 GBK 就变成一堆 `?`，
中文彻底丢失且**没有任何报错**。所以这里坚持：

  1. 解码按候选编码逐个尝试，UTF-8 解不出来才退 GBK；
  2. 编码目标前先检查每个字符能否编码，不能编码就**报错并拒绝写文件**，
     绝不静默替换；
  3. 出现 U+FFFD（说明源文件已经被别人转坏过）直接失败。

# 用法

    python tools/normalize_delivery_files.py            # 转换 + 校验
    python tools/normalize_delivery_files.py --check    # 只校验不写
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DIR = ROOT / "Application"

# 后缀 -> (目标编码, 人话说明)
PLAN: dict[str, tuple[str, str]] = {
    ".bat": ("cp936", "GBK(code page 936)"),
    ".cmd": ("cp936", "GBK(code page 936)"),
    ".txt": ("utf-8-sig", "UTF-8 with BOM"),
    ".nsh": ("cp936", "GBK(code page 936)"),
}

# 候选源编码：先 UTF-8（带不带 BOM 都行），再 GBK
SRC_CANDIDATES = ("utf-8-sig", "cp936")


def sniff(raw: bytes) -> tuple[str, str]:
    """猜源编码。返回 (编码, 文本)。"""
    for enc in SRC_CANDIDATES:
        try:
            return enc, raw.decode(enc)
        except UnicodeDecodeError:
            continue
    raise UnicodeDecodeError("utf-8", raw, 0, 1, "既不是 UTF-8 也不是 GBK")


def unencodable_chars(text: str, enc: str) -> list[str]:
    """列出目标编码无法表示的字符（不能靠 `errors='replace'` 蒙混过去）。"""
    bad: list[str] = []
    for ch in set(text):
        try:
            ch.encode(enc)
        except UnicodeEncodeError:
            bad.append(ch)
    return sorted(bad)


def normalize(root: Path, check_only: bool) -> int:
    # 递归扫描：交付目录现在会有子目录（例如 Application/Web版/ 里的启动脚本与说明），
    # 只扫顶层会静默漏掉它们 —— 而漏掉的后果正是本脚本要防的那种「客户机器上才花屏」。
    files = sorted(p for p in root.rglob("*") if p.is_file() and p.suffix.lower() in PLAN)
    if not files:
        print(f"[WARN] {root} 下没有需要处理的文件（{sorted(PLAN)}）")
        return 0

    failures = 0
    for path in files:
        target_enc, target_name = PLAN[path.suffix.lower()]
        raw = path.read_bytes()

        try:
            src_enc, text = sniff(raw)
        except UnicodeDecodeError as e:
            print(f"[FAIL] {path.name}: 无法解码（{e}）")
            failures += 1
            continue

        if "\ufffd" in text:
            print(
                f"[FAIL] {path.name}: 内容含 U+FFFD 替换字符 —— 这个文件已经被错误的"
                "编码转换弄坏过，中文已丢失。必须从源头重新写一份，不能就地修复。"
            )
            failures += 1
            continue

        text = text.replace("\r\n", "\n").replace("\r", "\n").replace("\n", "\r\n")

        bad = unencodable_chars(text, target_enc)
        if bad:
            print(
                f"[FAIL] {path.name}: 有 {len(bad)} 个字符无法用 {target_name} 表示："
                f"{[hex(ord(c)) for c in bad]} —— 请把源文里的这些字符换掉"
            )
            failures += 1
            continue

        payload = text.encode(target_enc)
        same = payload == raw
        status = "已是目标编码" if same else "需要转换"
        print(
            f"[OK]   {path.name}: 源编码≈{src_enc} → {target_name}"
            f" | {len(raw)} → {len(payload)} 字节 | {status}"
        )

        if check_only or same:
            continue
        path.write_bytes(payload)

        # 回读校验：写进去的字节按目标编码解回来必须与原文**逐字相同**。
        # 只比长度是不够的 —— '好' 和 '?' 都是 1 字节，静默替换就藏在这一步后面。
        back = path.read_bytes().decode(target_enc)
        if back != text:
            print(f"[FAIL] {path.name}: 回读不一致，写入过程发生了替换，已中止后续处理")
            failures += 1
            continue
        print(f"       已写入 {target_name}，回读一致（{len(payload)} 字节）")

    print()
    if failures:
        print(f"[FAILED] {failures} 个文件有问题")
        return 1
    print("[DONE] 全部符合目标编码" + ("（--check：未写文件）" if check_only else ""))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="统一交付目录文本文件的编码")
    ap.add_argument("--dir", default=str(DEFAULT_DIR), help="交付目录（默认 Application/）")
    ap.add_argument("--check", action="store_true", help="只校验，不写文件")
    args = ap.parse_args()

    root = Path(args.dir)
    if not root.is_dir():
        raise SystemExit(f"[FAIL] 目录不存在: {root}")
    print(f"目标目录: {root}")
    print("-" * 66)
    return normalize(root, args.check)


if __name__ == "__main__":
    sys.exit(main())
