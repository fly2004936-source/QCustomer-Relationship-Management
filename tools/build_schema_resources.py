#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""从 说明/ 下的源文件生成后端**编译期内嵌**的资源文件。

# 为什么需要这个脚本

`backend/src/db.rs` 用 `include_str!` 把建库 SQL 与表结构元数据在**编译期**嵌进二进制
（好处：交付物是单文件，运行时不需要外部 resources 目录）。
因此 `backend/resources/{schema.sql,schema.json}` 是**生成物**，
只要 DDL 变了就必须重新生成再重编后端 —— 手改这两个文件是没意义的，
下次重跑脚本就被覆盖。

# 唯一事实来源

  · 说明/08-客户管理系统SQLite建库语句.md   —— DDL（建库语句）
  · 说明/客户管理系统表结构元数据.json      —— 表白名单元数据（62 表 / 848 列）

# 用法

    python tools/build_schema_resources.py           # 生成 + 校验
    python tools/build_schema_resources.py --check   # 只校验不写文件（CI/构建前用）

退出码：0 = 通过；1 = 校验失败（会打印具体差异）
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DDL_MD = ROOT / "说明" / "08-客户管理系统SQLite建库语句.md"
META_JSON = ROOT / "说明" / "客户管理系统表结构元数据.json"
OUT_DIR = ROOT / "backend" / "resources"
OUT_SQL = OUT_DIR / "schema.sql"
OUT_JSON = OUT_DIR / "schema.json"

# CREATE TABLE 后面可以跟 IF NOT EXISTS，也可以被引号包住
CREATE_RE = re.compile(
    r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?[\"'`\[]?([A-Za-z_][A-Za-z0-9_]*)[\"'`\]]?",
    re.IGNORECASE,
)


def extract_sql(md_path: Path) -> str:
    """取 markdown 里第一个 ```sql 代码块的内容。

    只取第一个代码块：08 文档的约定就是「一个整段 DDL」。
    取到多个会静默拼错脚本，所以这里显式校验只有一个。
    """
    text = md_path.read_text(encoding="utf-8")
    blocks = re.findall(r"^```sql\s*\n(.*?)^```", text, re.S | re.M | re.IGNORECASE)
    if not blocks:
        raise SystemExit(f"[ERROR] {md_path.name} 里找不到 ```sql 代码块")
    if len(blocks) > 1:
        raise SystemExit(
            f"[ERROR] {md_path.name} 里有 {len(blocks)} 个 ```sql 代码块，"
            "约定只保留一个整段 DDL；请合并后再跑"
        )
    sql = blocks[0].strip() + "\n"
    if "CREATE TABLE" not in sql.upper():
        raise SystemExit(f"[ERROR] {md_path.name} 的 SQL 块里没有 CREATE TABLE")
    return sql


def load_meta(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    for key in ("enums", "tables"):
        if key not in data:
            raise SystemExit(f"[ERROR] {path.name} 缺少顶层键 {key!r}")
    if not isinstance(data["tables"], list) or not data["tables"]:
        raise SystemExit(f"[ERROR] {path.name} 的 tables 不是非空数组")
    return data


def table_names_in(sql: str) -> list[str]:
    """按出现顺序列出 DDL 里 CREATE TABLE 的表名（去重保序）。"""
    seen: list[str] = []
    for m in CREATE_RE.finditer(sql):
        name = m.group(1)
        if name not in seen:
            seen.append(name)
    return seen


def verify(sql: str, meta: dict, sql_path_label: str, meta_path_label: str) -> list[str]:
    """交叉校验 DDL 与元数据，返回错误列表（空 = 通过）。"""
    errors: list[str] = []
    sql_tables = table_names_in(sql)
    meta_tables = [t.get("name") for t in meta["tables"]]

    dupes = {n for n in meta_tables if meta_tables.count(n) > 1}
    if dupes:
        errors.append(f"{meta_path_label}: 表名重复 {sorted(dupes)}")

    only_sql = [t for t in sql_tables if t not in meta_tables]
    only_meta = [t for t in meta_tables if t not in sql_tables]
    if only_sql:
        errors.append(f"{sql_path_label} 有、元数据没有的表: {only_sql}")
    if only_meta:
        errors.append(f"元数据有、{sql_path_label} 没有的表: {only_meta}")

    # 列数交叉校验：元数据里每张表的列名必须在 DDL 里出现
    for t in meta["tables"]:
        name = t.get("name")
        cols = [c.get("name") for c in t.get("columns", [])]
        if not cols:
            errors.append(f"元数据表 {name} 没有任何列定义")
            continue
        if len(cols) != len(set(cols)):
            errors.append(f"元数据表 {name} 列名重复")
        block = re.search(
            r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?[\"'`\[]?"
            + re.escape(name)
            + r"[\"'`\]]?\s*\((.*?)\n\)\s*;",
            sql,
            re.S | re.IGNORECASE,
        )
        if not block:
            continue  # 已在 only_sql 里报过
        body = block.group(1)
        for col in cols:
            if not re.search(r"[\s(\"'`\[]" + re.escape(col) + r"[\s\"'`\]]", body):
                errors.append(f"表 {name} 的列 {col} 在 DDL 里找不到定义")

    # 元数据自带的计数要跟实际一致
    declared = meta.get("meta") or {}
    if declared.get("tableCount") and declared["tableCount"] != len(meta_tables):
        errors.append(
            f"元数据 meta.tableCount={declared['tableCount']} 与实际 {len(meta_tables)} 张表不符"
        )
    total_cols = sum(len(t.get("columns", [])) for t in meta["tables"])
    if declared.get("columnCount") and declared["columnCount"] != total_cols:
        errors.append(
            f"元数据 meta.columnCount={declared['columnCount']} 与实际 {total_cols} 列不符"
        )

    return errors


def main() -> int:
    ap = argparse.ArgumentParser(description="生成/校验后端编译期内嵌资源")
    ap.add_argument("--check", action="store_true", help="只校验，不写文件")
    args = ap.parse_args()

    if not DDL_MD.is_file():
        raise SystemExit(f"[ERROR] 找不到 DDL 源文件: {DDL_MD}")
    if not META_JSON.is_file():
        raise SystemExit(f"[ERROR] 找不到元数据源文件: {META_JSON}")

    sql = extract_sql(DDL_MD)
    meta = load_meta(META_JSON)

    errors = verify(sql, meta, DDL_MD.name, META_JSON.name)
    if errors:
        print("[FAIL] DDL 与元数据不一致：")
        for e in errors:
            print("  ·", e)
        return 1

    tables = len(meta["tables"])
    cols = sum(len(t.get("columns", [])) for t in meta["tables"])
    indexes = len(re.findall(r"CREATE\s+(?:UNIQUE\s+)?INDEX", sql, re.IGNORECASE))
    print(f"[OK] 校验通过：{tables} 张表 / {cols} 个列 / {indexes} 个索引")

    if args.check:
        print("[SKIP] --check：不写文件")
        return 0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    # newline='\n' 固定 LF；后端 execute_batch 对换行不敏感，但统一了便于 diff
    OUT_SQL.write_text(sql, encoding="utf-8", newline="\n")
    # 原样搬运元数据（顶层多出的 meta / conventions 会被 serde 忽略，但保留了溯源信息）
    OUT_JSON.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[WRITE] {OUT_SQL.relative_to(ROOT)}  ({OUT_SQL.stat().st_size} 字节)")
    print(f"[WRITE] {OUT_JSON.relative_to(ROOT)}  ({OUT_JSON.stat().st_size} 字节)")
    print("提示：这两个文件是编译期内嵌资源，改完必须重编后端（cargo build --release）才生效。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
