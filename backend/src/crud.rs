//! 通用 CRUD：**一套实现覆盖全部 61 张表**。
//!
//! 表名/列名都来自 `Registry` 白名单；所有值走参数化绑定。

use crate::error::{ApiError, FieldError};
use crate::query::ast::{Cond, Leaf, Op};
use crate::query::build::{self, row_to_json};
use crate::query::parse::{self, OrderBy, QuerySpec, MAX_BATCH};
use crate::schema::{Registry, Table};
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use rusqlite::types::Value as SqlValue;
use rusqlite::Connection;
use serde_json::{json, Map, Value};

pub fn ok(data: Value) -> Json<Value> {
    Json(json!({ "code": 0, "message": "ok", "data": data }))
}

fn page_count(total: i64, size: i64) -> i64 {
    if size <= 0 {
        0
    } else {
        (total + size - 1) / size
    }
}

fn table_of(st: &AppState, name: &str) -> Result<Table, ApiError> {
    Ok(st.reg.table(name)?.clone())
}

// ---------------------------------------------------------------------------
// 查询
// ---------------------------------------------------------------------------

pub fn run_list(st: &AppState, t: &Table, spec: &QuerySpec) -> Result<Value, ApiError> {
    let reg: &Registry = &st.reg;
    let plan = build::build_select(t, reg, spec)?;
    let (count_sql, count_params) = build::build_count(t, reg, spec)?;
    let limit = spec.effective_limit();
    let page = spec.page();

    st.db.with(|conn| {
        let total: i64 = {
            let mut stmt = conn.prepare(&count_sql)?;
            stmt.query_row(rusqlite::params_from_iter(count_params.iter()), |r| r.get(0))?
        };

        if spec.count_only {
            return Ok(json!({
                "items": [],
                "page": page,
                "page_size": limit,
                "total": total,
                "page_count": page_count(total, limit),
            }));
        }

        let mut stmt = conn.prepare(&plan.sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(plan.params.iter()))?;
        let mut items: Vec<Value> = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(row_to_json(row, &plan, t, reg, spec.raw_json)?);
        }

        // children：批量取子表，避免 N+1
        if !spec.children.is_empty() {
            let pk = t.primary_key.first().cloned().unwrap_or_else(|| "id".to_string());
            let ids: Vec<i64> = items
                .iter()
                .filter_map(|it| it.get(&pk).and_then(|v| v.as_i64()))
                .collect();
            if !ids.is_empty() {
                for cname in &spec.children {
                    attach_children(conn, reg, t, cname, &ids, &mut items, &pk, spec.raw_json)?;
                }
            }
        }

        Ok(json!({
            "items": items,
            "page": page,
            "page_size": limit,
            "total": total,
            "page_count": page_count(total, limit),
        }))
    })
}

fn attach_children(
    conn: &Connection,
    reg: &Registry,
    t: &Table,
    child_name: &str,
    ids: &[i64],
    items: &mut [Value],
    pk: &str,
    raw_json: bool,
) -> Result<(), ApiError> {
    let link = t
        .has_many
        .iter()
        .find(|h| h.table == child_name)
        .ok_or_else(|| {
            ApiError::param(format!(
                "表 {} 不存在子表 {}，可用: {}",
                t.name,
                child_name,
                t.childable().join(", ")
            ))
        })?
        .clone();
    let ct = reg.table(child_name)?;

    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT * FROM \"{}\" WHERE \"{}\" IN ({}) ORDER BY \"{}\" ASC",
        ct.name,
        link.column,
        placeholders,
        ct.primary_key.first().cloned().unwrap_or_else(|| "id".to_string())
    );
    let params: Vec<SqlValue> = ids.iter().map(|i| SqlValue::Integer(*i)).collect();
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
    let mut buckets: std::collections::BTreeMap<i64, Vec<Value>> =
        std::collections::BTreeMap::new();
    let mut idx_map: Vec<(String, usize)> = Vec::new();
    for c in &ct.columns {
        idx_map.push((c.name.clone(), idx_map.len()));
    }
    while let Some(row) = rows.next()? {
        let mut obj = Map::new();
        let mut parent: Option<i64> = None;
        for (cname, i) in &idx_map {
            let col = ct.require_column(cname)?;
            let v = crate::value::col_from_row(col, row, *i, raw_json)?;
            if *cname == link.column {
                parent = v.as_i64();
            }
            obj.insert(cname.clone(), v);
        }
        if let Some(p) = parent {
            buckets.entry(p).or_default().push(Value::Object(obj));
        }
    }

    for it in items.iter_mut() {
        if let Some(obj) = it.as_object_mut() {
            let id = obj.get(pk).and_then(|v| v.as_i64());
            let arr = id
                .and_then(|i| buckets.get(&i).cloned())
                .unwrap_or_default();
            obj.insert(child_name.to_string(), Value::Array(arr));
        }
    }
    Ok(())
}

pub fn fetch_one(
    st: &AppState,
    t: &Table,
    spec: &QuerySpec,
) -> Result<Option<Value>, ApiError> {
    let mut s = spec.clone();
    s.page = Some(1);
    s.page_size = Some(1);
    s.count_only = false;
    let data = run_list(st, t, &s)?;
    let items = data.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Ok(items.into_iter().next())
}

pub fn run_count(st: &AppState, t: &Table, spec: &QuerySpec) -> Result<Value, ApiError> {
    let (sql, params) = build::build_count(t, &st.reg, spec)?;
    let n: i64 = st.db.with(|conn| {
        let mut stmt = conn.prepare(&sql)?;
        Ok(stmt.query_row(rusqlite::params_from_iter(params.iter()), |r| r.get(0))?)
    })?;
    Ok(json!({ "count": n }))
}

// ---------------------------------------------------------------------------
// 写操作
// ---------------------------------------------------------------------------

/// 把请求体转成 (列名, 绑定值) 列表，并做白名单 + 校验
fn collect_writes(
    t: &Table,
    body: &Map<String, Value>,
    for_insert: bool,
) -> Result<Vec<(String, SqlValue)>, ApiError> {
    let mut out: Vec<(String, SqlValue)> = Vec::new();
    let mut errs: Vec<FieldError> = Vec::new();

    for (k, v) in body {
        let col = match t.column(k) {
            Some(c) => c,
            None => {
                return Err(ApiError::unknown_column(&t.name, k, t.column_names()));
            }
        };
        // 自增主键：忽略传入值
        if col.primary_key && col.auto_increment {
            continue;
        }
        match crate::value::json_to_sql(col, v) {
            Ok(sv) => out.push((k.clone(), sv)),
            Err(e) => {
                // 40003（JSON 列非法）等专属错误码直接透传，不压成 40001
                if e.code != 40001 {
                    return Err(e);
                }
                errs.extend(e.errors);
            }
        }
    }

    if !errs.is_empty() {
        return Err(ApiError::validation_many(errs));
    }

    if for_insert {
        let provided: Vec<String> = out.iter().map(|(c, _)| c.clone()).collect();
        for c in &t.columns {
            if c.primary_key && c.auto_increment {
                continue;
            }
            if provided.contains(&c.name) {
                continue;
            }
            if !c.not_null || c.default.is_some() {
                continue;
            }
            errs.push(FieldError::new(&c.name, "必填字段缺失"));
        }
        if !errs.is_empty() {
            return Err(ApiError::validation_many(errs));
        }
    } else if out.is_empty() {
        return Err(ApiError::param("请求体为空，没有任何字段需要更新"));
    }

    Ok(out)
}

/// 供 routes 层复用（批量更新 / 子表整体替换）
pub fn collect_writes_public(
    t: &Table,
    body: &Map<String, Value>,
    for_insert: bool,
) -> Result<Vec<(String, SqlValue)>, ApiError> {
    collect_writes(t, body, for_insert)
}

fn insert_sql(t: &Table, cols: &[String], upsert: Option<&Vec<String>>) -> String {
    let names = cols
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let ph = vec!["?"; cols.len()].join(", ");
    let mut sql = format!("INSERT INTO \"{}\" ({names}) VALUES ({ph})", t.name);
    if let Some(conflict) = upsert {
        let set = cols
            .iter()
            .filter(|c| !conflict.contains(c))
            .map(|c| format!("\"{c}\" = excluded.\"{c}\""))
            .collect::<Vec<_>>();
        let target = conflict
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        if set.is_empty() {
            sql.push_str(&format!(" ON CONFLICT ({target}) DO NOTHING"));
        } else {
            sql.push_str(&format!(
                " ON CONFLICT ({target}) DO UPDATE SET {}",
                set.join(", ")
            ));
        }
    }
    sql
}

async fn do_insert(
    st: &AppState,
    t: &Table,
    body: &Value,
) -> Result<Value, ApiError> {
    let obj = crate::value::body_to_object(body)?;
    let writes = collect_writes(t, obj, true)?;
    let cols: Vec<String> = writes.iter().map(|(c, _)| c.clone()).collect();
    let params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
    let sql = insert_sql(t, &cols, None);

    st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        stmt.execute(rusqlite::params_from_iter(params.iter()))?;
        let id = tx.last_insert_rowid();
        Ok(json!({ "id": id }))
    })
}

pub async fn create(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let t = table_of(&st, &name)?;
    let data = do_insert(&st, &t, &body).await?;
    Ok((StatusCode::CREATED, ok(data)))
}

pub async fn batch_create(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let t = table_of(&st, &name)?;
    let items = body
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::param("请求体需要 { \"items\": [ ... ] }"))?;
    if items.len() > MAX_BATCH {
        return Err(ApiError::over_limit(format!(
            "批量上限 {MAX_BATCH} 行，收到 {}",
            items.len()
        )));
    }
    if items.is_empty() {
        return Err(ApiError::param("items 为空"));
    }
    let strict = body
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|m| m != "relaxed")
        .unwrap_or(true);

    let mut inserted: i64 = 0;
    let mut ids: Vec<i64> = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    let mut first_err: Option<ApiError> = None;

    st.db.with_tx(|tx| {
        for (i, item) in items.iter().enumerate() {
            let obj = match item.as_object() {
                Some(o) => o,
                None => {
                    let e = ApiError::param(format!("items[{i}] 不是对象"));
                    if strict {
                        return Err(e);
                    }
                    failed.push(json!({ "index": i, "error": e.message }));
                    continue;
                }
            };
            let writes = match collect_writes(&t, obj, true) {
                Ok(w) => w,
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                    failed.push(json!({ "index": i, "error": e.message }));
                    continue;
                }
            };
            let cols: Vec<String> = writes.iter().map(|(c, _)| c.clone()).collect();
            let params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
            let sql = insert_sql(&t, &cols, None);
            let mut stmt = tx.prepare(&sql)?;
            match stmt.execute(rusqlite::params_from_iter(params.iter())) {
                Ok(_) => {
                    inserted += 1;
                    ids.push(tx.last_insert_rowid());
                }
                Err(e) => {
                    let api: ApiError = e.into();
                    if strict {
                        return Err(api);
                    }
                    failed.push(json!({ "index": i, "error": api.message }));
                }
            }
        }
        let _ = &mut first_err;
        Ok(())
    })?;

    Ok((
        StatusCode::CREATED,
        ok(json!({ "inserted": inserted, "ids": ids, "failed": failed })),
    ))
}

pub async fn upsert(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = table_of(&st, &name)?;
    let obj = crate::value::body_to_object(&body)?;

    let conflict: Vec<String> = match body.get("conflictBy") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => vec![s.to_string()],
        _ => Vec::new(),
    };
    if conflict.is_empty() {
        return Err(ApiError::param("upsert 需要 conflictBy（指定唯一键列）"));
    }
    if !t.unique_keys.iter().any(|k| k == &conflict) {
        return Err(ApiError::param(format!(
            "conflictBy {} 不是表 {} 已声明的唯一键，可用: {}",
            conflict.join("+"),
            t.name,
            t.unique_keys
                .iter()
                .map(|k| k.join("+"))
                .collect::<Vec<_>>()
                .join(" / ")
        )));
    }

    // conflictBy 之外的字段才是数据字段
    let mut data = obj.clone();
    data.remove("conflictBy");
    for c in &conflict {
        if !data.contains_key(c) {
            return Err(ApiError::param(format!("upsert 缺少唯一键字段 {c}")));
        }
    }
    let writes = collect_writes(&t, &data, true)?;
    let cols: Vec<String> = writes.iter().map(|(c, _)| c.clone()).collect();
    let params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
    let sql = insert_sql(&t, &cols, Some(&conflict));

    let out = st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        stmt.execute(rusqlite::params_from_iter(params.iter()))?;
        Ok(json!({ "id": tx.last_insert_rowid() }))
    })?;
    Ok(ok(out))
}

#[allow(clippy::too_many_arguments)]
pub fn run_update(
    st: &AppState,
    t: &Table,
    body: &Value,
    where_sql: &str,
    where_params: Vec<SqlValue>,
    full_replace: bool,
) -> Result<Value, ApiError> {
    let obj = crate::value::body_to_object(body)?;

    let writes = if full_replace {
        // 整体替换：未提供的可空列置 NULL
        let mut w: Vec<(String, SqlValue)> = Vec::new();
        let provided: Vec<String> = obj.keys().cloned().collect();
        for k in &provided {
            let col = t.require_column(k)?;
            if col.primary_key && col.auto_increment {
                continue;
            }
            w.push((k.clone(), crate::value::json_to_sql(col, &obj[k])?));
        }
        for c in &t.columns {
            if c.primary_key || c.auto_increment || c.not_null {
                continue;
            }
            if !provided.contains(&c.name) && c.default.is_none() {
                w.push((c.name.clone(), SqlValue::Null));
            }
        }
        w
    } else {
        collect_writes(t, obj, false)?
    };

    if writes.is_empty() {
        return Err(ApiError::param("没有可更新的字段"));
    }

    let mut sets: Vec<String> = writes
        .iter()
        .map(|(c, _)| format!("\"{c}\" = ?"))
        .collect();
    // 自动刷新 updated_at / updated_date
    for auto in ["updated_at", "updated_date"] {
        if t.column(auto).is_some() && !writes.iter().any(|(c, _)| c == auto) {
            sets.push(format!("\"{auto}\" = datetime('now','localtime')"));
        }
    }

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE {}",
        t.name,
        sets.join(", "),
        where_sql
    );
    let mut params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
    params.extend(where_params);

    st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        let n = stmt.execute(rusqlite::params_from_iter(params.iter()))?;
        Ok(json!({ "updated": n }))
    })
}

// ---------------------------------------------------------------------------
// 条件构造小工具
// ---------------------------------------------------------------------------

/// 单列等值条件（用于按 id / 按唯一键定位）
pub fn eq_cond(t: &Table, column: &str, v: Value) -> Result<Cond, ApiError> {
    t.require_column(column)?;
    Ok(Cond::Leaf(Leaf::new(column, Op::Eq, vec![v])))
}

pub fn cond_to_parts(
    t: &Table,
    reg: &Registry,
    cond: Option<Cond>,
    extra: ContextParams,
) -> Result<(String, Vec<SqlValue>), ApiError> {
    let mut params: Vec<SqlValue> = Vec::new();
    let mut sql = match cond {
        Some(c) => crate::query::ast::render_cond(t, reg, &t.name, &c, &mut params)?,
        None => "1 = 1".to_string(),
    };
    let _ = extra;
    if sql == "1 = 1" {
        sql = "1 = 1".to_string();
    }
    Ok((sql, params))
}

/// 预留：调用方注入的额外参数（当前未使用）
#[derive(Default)]
pub struct ContextParams;

/// 解析 id 路径参数
pub fn parse_id(raw: &str) -> Result<i64, ApiError> {
    raw.parse::<i64>()
        .map_err(|_| ApiError::param(format!("主键必须是整数: {raw}")))
}

/// 由查询串生成 spec（表名已知）
pub fn spec_of(
    st: &AppState,
    t: &Table,
    raw: Option<&str>,
) -> Result<QuerySpec, ApiError> {
    parse::spec_from_query(t, &st.reg, raw)
}

/// 默认排序（按主键）
pub fn default_order(t: &Table) -> Vec<OrderBy> {
    t.primary_key
        .iter()
        .map(|k| OrderBy {
            column: k.clone(),
            direction: "ASC".to_string(),
            nulls_last: false,
        })
        .collect()
}
