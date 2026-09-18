//! SQL 构建器：`QuerySpec` → 参数化 SQL + 参数列表 + 输出列计划。
//!
//! 结构（表名/列名/函数名）全部来自白名单，用户值全部走 `?`。

use crate::error::ApiError;
use crate::query::ast::{render_cond, Cond, Leaf, Op};
use crate::query::parse::{Agg, OrderBy, QuerySpec};
use crate::schema::{Registry, Table};
use rusqlite::types::Value as SqlValue;
use rusqlite::Row;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum OutputCol {
    Base(String),
    Joined { key: String, column: String },
    Agg { alias: String },
}

#[derive(Debug, Clone)]
pub struct SelectPlan {
    pub sql: String,
    pub params: Vec<SqlValue>,
    pub outputs: Vec<OutputCol>,
}

struct Parts {
    from: String,
    where_sql: String,
    params: Vec<SqlValue>,
}

/// 列表达式：支持 `col` 与 `关联表.col`
fn expr_of(t: &Table, reg: &Registry, c: &str) -> Result<String, ApiError> {
    if let Some((rel, col)) = c.split_once('.') {
        let rt = reg.table(rel)?;
        rt.require_column(col)?;
        if t.belongs_to.iter().any(|b| b.table == rel) {
            return Ok(format!("\"{rel}\".\"{col}\""));
        }
        // JSON 路径
        if let Some(base) = t.column(rel) {
            if base.json {
                return Ok(format!(
                    "json_extract(\"{}\".\"{}\", '$.{}')",
                    t.name,
                    rel,
                    col.replace('\'', "")
                ));
            }
        }
        return Err(ApiError::unknown_column(&t.name, c, t.column_names()));
    }
    t.require_column(c)?;
    Ok(format!("\"{}\".\"{}\"", t.name, c))
}

/// 扫描条件树，收集所有需要 JOIN 的关联表
fn collect_related(cond: &Option<Cond>, out: &mut Vec<String>) {
    fn walk(c: &Cond, out: &mut Vec<String>) {
        match c {
            Cond::And(v) | Cond::Or(v) => {
                for x in v {
                    walk(x, out);
                }
            }
            Cond::Not(x) => walk(x, out),
            Cond::Leaf(l) => {
                if let Some((head, _)) = l.column.split_once('.') {
                    if !out.iter().any(|x| x == head) {
                        out.push(head.to_string());
                    }
                }
            }
        }
    }
    if let Some(c) = cond {
        walk(c, out);
    }
}

fn build_parts(t: &Table, reg: &Registry, spec: &QuerySpec) -> Result<Parts, ApiError> {
    // 1) 收集 JOIN 需求
    let mut needs: Vec<String> = Vec::new();
    for e in &spec.expand {
        if !t.expandable().iter().any(|x| x == e) {
            return Err(ApiError::param(format!(
                "表 {} 不支持 expand={}，可用: {}",
                t.name,
                e,
                t.expandable().join(", ")
            )));
        }
        if !needs.iter().any(|x| x == e) {
            needs.push(e.clone());
        }
    }
    collect_related(&spec.cond, &mut needs);
    // 过滤用的关联表必须真的是外键关系
    for n in &needs {
        if !t.belongs_to.iter().any(|b| &b.table == n) {
            // 可能是 JSON 路径（如 p3p4_flags.p1），不是关联表，跳过
            if t.column(n).map(|c| c.json).unwrap_or(false) {
                continue;
            }
            return Err(ApiError::unknown_column(
                &t.name,
                n,
                t.columns.iter().map(|c| c.name.clone()).collect(),
            ));
        }
    }

    // 2) FROM + JOIN
    let mut from = format!("\"{}\"", t.name);
    for n in &needs {
        let b = match t.belongs_to.iter().find(|b| &b.table == n) {
            Some(b) => b,
            None => continue,
        };
        if b.table == t.name {
            return Err(ApiError::param(format!(
                "表 {} 的自关联（{}）暂不支持 expand/关联筛选",
                t.name, b.column
            )));
        }
        from.push_str(&format!(
            " LEFT JOIN \"{}\" ON \"{}\".\"{}\" = \"{}\".\"{}\"",
            b.table, t.name, b.column, b.table, b.ref_column
        ));
    }

    // 3) WHERE
    let mut conds: Vec<Cond> = Vec::new();
    if let Some(c) = &spec.cond {
        conds.push(c.clone());
    }
    if let Some(q) = &spec.q {
        let mut ls: Vec<Cond> = Vec::new();
        for c in t.searchable_columns() {
            ls.push(Cond::Leaf(Leaf::new(c, Op::Like, vec![Value::String(q.clone())])));
        }
        // JSON 列按原文模糊匹配
        for c in t.columns.iter().filter(|c| c.json) {
            ls.push(Cond::Leaf(Leaf::new(
                c.name.clone(),
                Op::Like,
                vec![Value::String(q.clone())],
            )));
        }
        if let Some(c) = Cond::or(ls) {
            conds.push(c);
        }
    }

    let mut params: Vec<SqlValue> = Vec::new();
    let where_sql = match Cond::and(conds) {
        Some(c) => render_cond(t, reg, &t.name, &c, &mut params)?,
        None => "1 = 1".to_string(),
    };

    Ok(Parts {
        from,
        where_sql,
        params,
    })
}

fn agg_sql(t: &Table, reg: &Registry, a: &Agg) -> Result<String, ApiError> {
    match (&a.func[..], &a.column) {
        ("count", None) => Ok("count(*)".to_string()),
        ("count_distinct", Some(c)) => Ok(format!("count(DISTINCT {})", expr_of(t, reg, c)?)),
        (f, Some(c)) => Ok(format!("{}({})", f, expr_of(t, reg, c)?)),
        (f, None) => Err(ApiError::param(format!(
            "聚合函数 {f} 必须指定列（只有 count 支持 *）"
        ))),
    }
}

/// 构建 SELECT 语句
pub fn build_select(t: &Table, reg: &Registry, spec: &QuerySpec) -> Result<SelectPlan, ApiError> {
    let parts = build_parts(t, reg, spec)?;
    let mut params = parts.params;
    let mut outputs: Vec<OutputCol> = Vec::new();
    let mut sel: Vec<String> = Vec::new();

    let aggregated = !spec.aggregates.is_empty() || !spec.group_by.is_empty();

    if aggregated {
        for g in &spec.group_by {
            sel.push(expr_of(t, reg, g)?);
            outputs.push(OutputCol::Base(g.clone()));
        }
        for a in &spec.aggregates {
            sel.push(format!("{} AS \"{}\"", agg_sql(t, reg, a)?, a.alias));
            outputs.push(OutputCol::Agg {
                alias: a.alias.clone(),
            });
        }
        if sel.is_empty() {
            return Err(ApiError::param("group_by / agg 至少需要一个参数"));
        }
    } else {
        let cols: Vec<String> = if spec.select.is_empty() {
            t.columns.iter().map(|c| c.name.clone()).collect()
        } else {
            spec.select.clone()
        };
        for c in &cols {
            match c.split_once('.') {
                Some((rel, col)) if t.belongs_to.iter().any(|b| b.table == rel) => {
                    sel.push(format!("\"{rel}\".\"{col}\" AS \"{rel}__{col}\""));
                    outputs.push(OutputCol::Joined {
                        key: rel.to_string(),
                        column: col.to_string(),
                    });
                }
                _ => {
                    sel.push(expr_of(t, reg, c)?);
                    outputs.push(OutputCol::Base(c.clone()));
                }
            }
        }
        // expand 的关联表带出全部列
        for e in &spec.expand {
            let rt = reg.table(e)?;
            for c in &rt.columns {
                sel.push(format!("\"{e}\".\"{}\" AS \"{e}__{}\"", c.name, c.name));
                outputs.push(OutputCol::Joined {
                    key: e.clone(),
                    column: c.name.clone(),
                });
            }
        }
    }

    let distinct = if spec.distinct { "DISTINCT " } else { "" };
    let mut sql = format!(
        "SELECT {distinct}{} FROM {}{}",
        sel.join(", "),
        parts.from,
        if parts.where_sql == "1 = 1" {
            String::new()
        } else {
            format!(" WHERE {}", parts.where_sql)
        }
    );

    // GROUP BY
    if !spec.group_by.is_empty() {
        let mut g = Vec::new();
        for c in &spec.group_by {
            g.push(expr_of(t, reg, c)?);
        }
        sql.push_str(&format!(" GROUP BY {}", g.join(", ")));
    }

    // ORDER BY
    if !spec.order_by.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_str(&order_sql(t, reg, &spec.order_by)?);
    } else if aggregated {
        // 聚合结果默认按第一列排序，保证分页稳定
        sql.push_str(" ORDER BY 1");
    } else {
        sql.push_str(&format!(" ORDER BY \"{}\".\"id\" ASC", t.name));
    }

    // 聚合结果不分页（除非显式传了 page/page_size）
    let paging = !aggregated || spec.page.is_some() || spec.page_size.is_some() || spec.limit.is_some();
    if paging {
        sql.push_str(" LIMIT ? OFFSET ?");
        params.push(SqlValue::Integer(spec.effective_limit()));
        params.push(SqlValue::Integer(spec.effective_offset()));
    }

    Ok(SelectPlan {
        sql,
        params,
        outputs,
    })
}

fn order_sql(t: &Table, reg: &Registry, ob: &[OrderBy]) -> Result<String, ApiError> {
    let mut parts = Vec::new();
    for o in ob {
        let e = expr_of(t, reg, &o.column)?;
        if o.nulls_last {
            parts.push(format!("({e} IS NULL) ASC"));
        }
        parts.push(format!("{e} {}", o.direction));
    }
    Ok(parts.join(", "))
}

/// 构建 COUNT 语句（条件与 SELECT 完全一致）
pub fn build_count(
    t: &Table,
    reg: &Registry,
    spec: &QuerySpec,
) -> Result<(String, Vec<SqlValue>), ApiError> {
    let parts = build_parts(t, reg, spec)?;
    let mut sql = format!("SELECT count(*) FROM {}", parts.from);
    if parts.where_sql != "1 = 1" {
        sql.push_str(&format!(" WHERE {}", parts.where_sql));
    }
    Ok((sql, parts.params))
}

/// 结果行 → JSON 对象（处理 JSON 列反序列化、expand 嵌套、聚合别名）
pub fn row_to_json(
    row: &Row<'_>,
    plan: &SelectPlan,
    t: &Table,
    reg: &Registry,
    raw_json: bool,
) -> Result<Value, ApiError> {
    let mut map = Map::new();
    let mut joined: BTreeMap<String, Map<String, Value>> = BTreeMap::new();

    for (i, out) in plan.outputs.iter().enumerate() {
        match out {
            OutputCol::Base(name) => {
                let (tbl, colname) = match name.split_once('.') {
                    Some((rel, c)) => (reg.table(rel)?, c),
                    None => (t, name.as_str()),
                };
                let col = tbl.require_column(colname)?;
                let v = crate::value::col_from_row(col, row, i, raw_json)?;
                // 关联表列（select=customer.name）归入嵌套对象
                if let Some((rel, _)) = name.split_once('.') {
                    if t.belongs_to.iter().any(|b| b.table == rel) {
                        joined
                            .entry(rel.to_string())
                            .or_default()
                            .insert(colname.to_string(), v);
                        continue;
                    }
                }
                map.insert(name.clone(), v);
            }
            OutputCol::Agg { alias } => {
                let v: SqlValue = row.get(i)?;
                map.insert(alias.clone(), plain_json(v));
            }
            OutputCol::Joined { key, column } => {
                let rt = reg.table(key)?;
                let col = rt.require_column(column)?;
                let v = crate::value::col_from_row(col, row, i, raw_json)?;
                joined
                    .entry(key.clone())
                    .or_default()
                    .insert(column.clone(), v);
            }
        }
    }

    for (k, m) in joined {
        let all_null = m.values().all(|v| v.is_null());
        map.insert(k, if all_null { Value::Null } else { Value::Object(m) });
    }

    Ok(Value::Object(map))
}

fn plain_json(v: SqlValue) -> Value {
    match v {
        SqlValue::Null => Value::Null,
        SqlValue::Integer(i) => Value::from(i),
        SqlValue::Real(f) => Value::from(f),
        SqlValue::Text(s) => Value::String(s),
        SqlValue::Blob(_) => Value::Null,
    }
}

/// 多态表必须带 `person_type + person_id` / `entity_type + entity_id` 的校验
pub fn require_polymorphic_keys(t: &Table, conds: &Map<String, Value>) -> Result<(), ApiError> {
    let pair: Option<(&str, &str)> = if t.columns.iter().any(|c| c.name == "person_type") {
        Some(("person_type", "person_id"))
    } else if t.columns.iter().any(|c| c.name == "entity_type") {
        Some(("entity_type", "entity_id"))
    } else {
        None
    };
    if let Some((k1, k2)) = pair {
        let has1 = conds.get(k1).map(|v| !v.is_null()).unwrap_or(false);
        let has2 = conds.get(k2).map(|v| !v.is_null()).unwrap_or(false);
        if !(has1 && has2) {
            return Err(ApiError::param(format!(
                "表 {} 是多态表，必须同时提供 {k1} 与 {k2}",
                t.name
            )));
        }
    }
    Ok(())
}
