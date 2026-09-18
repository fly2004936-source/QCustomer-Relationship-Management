//! HTTP 路由与 handler。
//!
//! 路由设计（axum 0.8 的 `{}` 参数语法）：
//! - 静态段优先于参数段，所以 `/{table}/count`、`/{table}/by` 等不会被 `/{table}/{id}` 吃掉
//! - 子表接口用 `/{table}/{id}/children/{child}`，避免与 `/{table}/{column}/{value}` 同形冲突

use crate::crud;
use crate::error::ApiError;
use crate::query::ast::{render_cond, Cond};
use crate::query::build;
use crate::query::parse;
use crate::schema::Table;
use crate::AppState;
use axum::extract::{Path, RawQuery, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::types::Value as SqlValue;
use serde_json::{json, Value};
use std::time::Duration;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

// ---------------------------------------------------------------------------
// 元数据
// ---------------------------------------------------------------------------

async fn meta_tables(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(crud::ok(crate::schema::tables_summary(&st.reg)))
}

async fn meta_table(
    State(st): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?;
    Ok(crud::ok(json!({
        "name": t.name,
        "group": t.group,
        "primary_key": t.primary_key,
        "unique_keys": t.unique_keys,
        "foreign_keys": t.foreign_keys,
        "indexes": t.indexes,
        "belongs_to": t.belongs_to,
        "has_many": t.has_many,
        "columns": t.columns,
    })))
}

async fn meta_enums(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(crud::ok(json!(st.reg.enums())))
}

async fn meta_schema() -> Result<Response, ApiError> {
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        crate::db::SCHEMA_JSON,
    )
        .into_response())
}

async fn meta_health(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let counts = st.db.table_row_counts(200)?;
    let total_rows: i64 = counts.iter().map(|(_, c)| *c).sum();
    Ok(crud::ok(json!({
        "status": "ok",
        "db_path": st.db.path().to_string_lossy(),
        "table_count": st.reg.table_count(),
        "total_rows": total_rows,
        "tables": counts.into_iter().map(|(n, c)| json!({"table": n, "rows": c})).collect::<Vec<_>>(),
    })))
}

// ---------------------------------------------------------------------------
// 通用 CRUD
// ---------------------------------------------------------------------------

async fn list(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    Ok(crud::ok(crud::run_list(&st, &t, &spec)?))
}

async fn count(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    Ok(crud::ok(crud::run_count(&st, &t, &spec)?))
}

async fn exists(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    let data = crud::run_count(&st, &t, &spec)?;
    let n = data.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
    Ok(crud::ok(json!({ "exists": n > 0 })))
}

async fn get_by_id(
    State(st): State<AppState>,
    Path((name, id)): Path<(String, String)>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let pk = t
        .primary_key
        .first()
        .cloned()
        .ok_or_else(|| ApiError::param(format!("表 {} 没有单一主键", t.name)))?;
    let mut spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    let v = crud::parse_id(&id)?;
    let cond = crud::eq_cond(&t, &pk, Value::from(v))?;
    spec.cond = Some(match spec.cond.take() {
        Some(c) => Cond::And(vec![c, cond]),
        None => cond,
    });
    match crud::fetch_one(&st, &t, &spec)? {
        Some(v) => Ok(crud::ok(v)),
        None => Err(ApiError::not_found(format!(
            "{} 中不存在 {pk}={id}",
            t.name
        ))),
    }
}

/// `/{table}/{column}/{value}`：唯一列返回单对象，普通列返回列表
async fn get_by_column(
    State(st): State<AppState>,
    Path((name, column, value)): Path<(String, String, String)>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let col = t.require_column(&column)?.clone();
    let mut spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    let json_v = coerce_path_value(&col, &value);
    let cond = crud::eq_cond(&t, &column, json_v)?;
    spec.cond = Some(match spec.cond.take() {
        Some(c) => Cond::And(vec![c, cond]),
        None => cond,
    });

    let is_single = t.primary_key.iter().any(|k| k == &column)
        || t.unique_keys
            .iter()
            .any(|k| k.len() == 1 && k[0] == column);
    if is_single {
        match crud::fetch_one(&st, &t, &spec)? {
            Some(v) => Ok(crud::ok(v)),
            None => Err(ApiError::not_found(format!(
                "{} 中不存在 {column}={value}",
                t.name
            ))),
        }
    } else {
        Ok(crud::ok(crud::run_list(&st, &t, &spec)?))
    }
}

/// `/{table}/by?k1=v1&k2=v2`：命中完整唯一键 → 单对象，否则降级为列表
async fn get_by_keys(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    if t.unique_keys.is_empty() && t.primary_key.len() == 1 {
        // 允许用主键
    }
    let provided = eq_columns_of_query(raw.as_deref());
    let matched = t
        .unique_keys
        .iter()
        .find(|k| k.iter().all(|c| provided.contains(c)));
    let pk_matched = t.primary_key.len() == 1 && provided.contains(&t.primary_key[0]);

    if let Some(_k) = matched.or(if pk_matched { Some(&t.primary_key) } else { None }) {
        match crud::fetch_one(&st, &t, &spec)? {
            Some(v) => Ok(crud::ok(v)),
            None => Err(ApiError::not_found(format!(
                "{} 中不存在符合条件的记录",
                t.name
            ))),
        }
    } else {
        Ok(crud::ok(crud::run_list(&st, &t, &spec)?))
    }
}

async fn query(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_body(&t, &st.reg, &body)?;
    Ok(crud::ok(crud::run_list(&st, &t, &spec)?))
}

async fn put_by_id(
    State(st): State<AppState>,
    Path((name, id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let (where_sql, params) = pk_where(&t, &id)?;
    Ok(crud::ok(crud::run_update(&st, &t, &body, &where_sql, params, true)?))
}

async fn patch_by_id(
    State(st): State<AppState>,
    Path((name, id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let (where_sql, params) = pk_where(&t, &id)?;
    let out = crud::run_update(&st, &t, &body, &where_sql, params, false)?;
    if out.get("updated").and_then(|v| v.as_i64()).unwrap_or(0) == 0 {
        return Err(ApiError::not_found(format!("{} 中不存在 id={id}", t.name)));
    }
    Ok(crud::ok(out))
}

async fn batch_patch(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let pk = t
        .primary_key
        .first()
        .cloned()
        .ok_or_else(|| ApiError::param("该表没有单一主键"))?;

    let pairs: Vec<(i64, Value)> = if let Some(items) = body.get("items").and_then(|v| v.as_array())
    {
        items
            .iter()
            .map(|it| {
                let id = it
                    .get("id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| ApiError::param("items 项缺少 id"))?;
                Ok((id, it.clone()))
            })
            .collect::<Result<Vec<_>, ApiError>>()?
    } else if let (Some(ids), Some(patch)) = (
        body.get("ids").and_then(|v| v.as_array()),
        body.get("patch").and_then(|v| v.as_object()),
    ) {
        let pv = Value::Object(patch.clone());
        ids.iter()
            .map(|v| {
                let id = v
                    .as_i64()
                    .ok_or_else(|| ApiError::param("ids 必须是整数数组"))?;
                Ok((id, pv.clone()))
            })
            .collect::<Result<Vec<_>, ApiError>>()?
    } else {
        return Err(ApiError::param(
            "请求体需要 { ids: [...], patch: {...} } 或 { items: [...] }",
        ));
    };

    if pairs.is_empty() {
        return Err(ApiError::param("没有要更新的记录"));
    }
    if pairs.len() > parse::MAX_BATCH {
        return Err(ApiError::over_limit(format!(
            "批量上限 {} 行",
            parse::MAX_BATCH
        )));
    }

    let mut updated: i64 = 0;
    st.db.with_tx(|tx| {
        for (id, body) in &pairs {
            let mut b = body.clone();
            if let Some(o) = b.as_object_mut() {
                o.remove("id");
            }
            let obj = crate::value::body_to_object(&b)?;
            let writes = collect_writes_public(&t, obj, false)?;
            if writes.is_empty() {
                continue;
            }
            let mut sets: Vec<String> = writes
                .iter()
                .map(|(c, _)| format!("\"{c}\" = ?"))
                .collect();
            for auto in ["updated_at", "updated_date"] {
                if t.column(auto).is_some() && !writes.iter().any(|(c, _)| c == auto) {
                    sets.push(format!("\"{auto}\" = datetime('now','localtime')"));
                }
            }
            let sql = format!(
                "UPDATE \"{}\" SET {} WHERE \"{}\" = ?",
                t.name,
                sets.join(", "),
                pk
            );
            let mut params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
            params.push(SqlValue::Integer(*id));
            let mut stmt = tx.prepare(&sql)?;
            updated += stmt.execute(rusqlite::params_from_iter(params.iter()))? as i64;
        }
        Ok(())
    })?;

    Ok(crud::ok(json!({ "updated": updated })))
}

async fn batch_delete(
    State(st): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let pk = t
        .primary_key
        .first()
        .cloned()
        .ok_or_else(|| ApiError::param("该表没有单一主键"))?;
    let ids = body
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::param("请求体需要 { ids: [...] }"))?;
    if ids.is_empty() {
        return Err(ApiError::param("ids 为空"));
    }
    if ids.len() > parse::MAX_BATCH {
        return Err(ApiError::over_limit(format!(
            "批量上限 {} 行",
            parse::MAX_BATCH
        )));
    }
    let vals: Vec<i64> = ids
        .iter()
        .map(|v| {
            v.as_i64()
                .ok_or_else(|| ApiError::param("ids 必须是整数数组"))
        })
        .collect::<Result<_, _>>()?;
    let ph = vec!["?"; vals.len()].join(", ");
    let sql = format!("DELETE FROM \"{}\" WHERE \"{}\" IN ({ph})", t.name, pk);
    let params: Vec<SqlValue> = vals.iter().map(|i| SqlValue::Integer(*i)).collect();
    let n = st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(params.iter()))?)
    })?;
    Ok(crud::ok(json!({ "deleted": n })))
}

async fn delete_by_id(
    State(st): State<AppState>,
    Path((name, id)): Path<(String, String)>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let (where_sql, params) = pk_where(&t, &id)?;

    if raw.as_deref().map(|r| r.contains("dry_run")).unwrap_or(false) {
        return Ok(crud::ok(dry_run(&st, &t, &where_sql, params)?));
    }

    let sql = format!("DELETE FROM \"{}\" WHERE {where_sql}", t.name);
    let n = st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(params.iter()))?)
    })?;
    if n == 0 {
        return Err(ApiError::not_found(format!("{} 中不存在 id={id}", t.name)));
    }
    Ok(crud::ok(json!({ "deleted": n })))
}

async fn delete_by_filter(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    if spec.cond.is_none() {
        return Err(ApiError::dangerous(
            "按条件删除必须至少提供一个 filter 或 where 条件",
        ));
    }
    let cond = spec.cond.clone().unwrap();
    let mut params: Vec<SqlValue> = Vec::new();
    let where_sql = render_cond(&t, &st.reg, &t.name, &cond, &mut params)?;

    if raw.as_deref().map(|r| r.contains("dry_run")).unwrap_or(false) {
        return Ok(crud::ok(dry_run(&st, &t, &where_sql, params)?));
    }

    let sql = format!("DELETE FROM \"{}\" WHERE {where_sql}", t.name);
    let n = st.db.with_tx(|tx| {
        let mut stmt = tx.prepare(&sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(params.iter()))?)
    })?;
    Ok(crud::ok(json!({ "deleted": n })))
}

/// 预演：返回本级 + 各子表将被连带删除的行数
fn dry_run(
    st: &AppState,
    t: &Table,
    where_sql: &str,
    params: Vec<SqlValue>,
) -> Result<Value, ApiError> {
    let pk = t.primary_key.first().cloned().unwrap_or_else(|| "id".into());
    st.db.with(|conn| {
        let mut out = serde_json::Map::new();
        let base_sql = format!("SELECT count(*) FROM \"{}\" WHERE {where_sql}", t.name);
        let n: i64 = conn.query_row(
            &base_sql,
            rusqlite::params_from_iter(params.iter()),
            |r| r.get(0),
        )?;
        out.insert(t.name.clone(), json!(n));

        let mut seen: Vec<String> = Vec::new();
        for h in &t.has_many {
            if h.on_delete.as_deref() != Some("CASCADE") || seen.contains(&h.table) {
                continue;
            }
            seen.push(h.table.clone());
            let sql = format!(
                "SELECT count(*) FROM \"{}\" WHERE \"{}\" IN (SELECT \"{}\" FROM \"{}\" WHERE {})",
                h.table, h.column, pk, t.name, where_sql
            );
            let c: i64 = conn.query_row(&sql, rusqlite::params_from_iter(params.iter()), |r| r.get(0))?;
            out.insert(h.table.clone(), json!(c));
        }
        Ok(json!(out))
    })
}

async fn get_children(
    State(st): State<AppState>,
    Path((name, id, child)): Path<(String, String, String)>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let link = child_link(&t, &child)?;
    let ct = st.reg.table(&child)?.clone();
    let idv = crud::parse_id(&id)?;

    let mut spec = parse::spec_from_query(&ct, &st.reg, raw.as_deref())?;
    let cond = crud::eq_cond(&ct, &link, Value::from(idv))?;
    spec.cond = Some(match spec.cond.take() {
        Some(c) => Cond::And(vec![c, cond]),
        None => cond,
    });
    Ok(crud::ok(crud::run_list(&st, &ct, &spec)?))
}

async fn put_children(
    State(st): State<AppState>,
    Path((name, id, child)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let link = child_link(&t, &child)?;
    let ct = st.reg.table(&child)?.clone();
    let idv = crud::parse_id(&id)?;

    let items = body
        .as_array()
        .cloned()
        .or_else(|| body.get("items").and_then(|v| v.as_array()).cloned())
        .ok_or_else(|| ApiError::param("请求体需要是数组，或 { items: [...] }"))?;
    if items.len() > parse::MAX_BATCH {
        return Err(ApiError::over_limit(format!(
            "批量上限 {} 行",
            parse::MAX_BATCH
        )));
    }

    let mut replaced: i64 = 0;
    st.db.with_tx(|tx| {
        tx.execute(
            &format!("DELETE FROM \"{}\" WHERE \"{}\" = ?", ct.name, link),
            rusqlite::params![idv],
        )?;
        for item in &items {
            let mut obj = match item.as_object() {
                Some(o) => o.clone(),
                None => return Err(ApiError::param("数组元素必须是对象")),
            };
            // 强制写入关联列
            obj.insert(link.clone(), Value::from(idv));
            let writes = collect_writes_public(&ct, &obj, true)?;
            let cols: Vec<String> = writes.iter().map(|(c, _)| c.clone()).collect();
            let params: Vec<SqlValue> = writes.iter().map(|(_, v)| v.clone()).collect();
            let sql = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({})",
                ct.name,
                cols.iter()
                    .map(|c| format!("\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
                vec!["?"; cols.len()].join(", ")
            );
            let mut stmt = tx.prepare(&sql)?;
            stmt.execute(rusqlite::params_from_iter(params.iter()))?;
            replaced += 1;
        }
        Ok(())
    })?;

    Ok(crud::ok(json!({ "replaced": replaced })))
}

async fn export(
    State(st): State<AppState>,
    Path(name): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Response, ApiError> {
    let t = st.reg.table(&name)?.clone();
    let mut spec = parse::spec_from_query(&t, &st.reg, raw.as_deref())?;
    // 导出不分页，但要限制规模
    spec.limit = Some(parse::MAX_PAGE_SIZE * 50);
    spec.page = Some(1);
    let plan = build::build_select(&t, &st.reg, &spec)?;
    let reg = st.reg.clone();
    let raw_json = spec.raw_json;

    let csv = st.db.with(|conn| {
        let mut stmt = conn.prepare(&plan.sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(plan.params.iter()))?;
        let names: Vec<String> = (0..plan.outputs.len())
            .map(|i| match &plan.outputs[i] {
                build::OutputCol::Base(c) => c.clone(),
                build::OutputCol::Joined { key, column } => format!("{key}__{column}"),
                build::OutputCol::Agg { alias } => alias.clone(),
            })
            .collect();
        let mut out = String::from("\u{feff}");
        out.push_str(&names.join(","));
        out.push('\n');
        while let Some(row) = rows.next()? {
            let obj = build::row_to_json(row, &plan, &t, &reg, raw_json)?;
            let cells: Vec<String> = names
                .iter()
                .map(|n| csv_cell(obj.get(n)))
                .collect();
            out.push_str(&cells.join(","));
            out.push('\n');
        }
        Ok(out)
    })?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"export.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

fn csv_cell(v: Option<&Value>) -> String {
    let s = match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    };
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

async fn fallback(uri: axum::http::Uri) -> ApiError {
    ApiError::not_found(format!(
        "未知路径 {}。可用形式：/api/v1/{{table}}、/api/v1/{{table}}/{{id}}、/api/v1/meta/tables",
        uri.path()
    ))
}

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

fn pk_where(t: &Table, id: &str) -> Result<(String, Vec<SqlValue>), ApiError> {
    let pk = t
        .primary_key
        .first()
        .cloned()
        .ok_or_else(|| ApiError::param(format!("表 {} 没有单一主键", t.name)))?;
    let v = crud::parse_id(id)?;
    Ok((
        format!("\"{}\" = ?", pk),
        vec![SqlValue::Integer(v)],
    ))
}

fn child_link(t: &Table, child: &str) -> Result<String, ApiError> {
    t.has_many
        .iter()
        .find(|h| h.table == child)
        .map(|h| h.column.clone())
        .ok_or_else(|| {
            ApiError::param(format!(
                "表 {} 不存在子表 {}，可用: {}",
                t.name,
                child,
                t.childable().join(", ")
            ))
        })
}

fn coerce_path_value(col: &crate::schema::Column, raw: &str) -> Value {
    match col.ty.as_str() {
        "INTEGER" | "BOOLEAN" => raw
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "REAL" => raw
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(raw.to_string())),
        _ => Value::String(raw.to_string()),
    }
}

/// 从查询串里挑出「等值列名」（用于判断是否命中完整唯一键）
fn eq_columns_of_query(raw: Option<&str>) -> Vec<String> {
    const CTRL: [&str; 13] = [
        "page", "page_size", "limit", "offset", "order_by", "select", "group_by", "agg",
        "expand", "children", "raw", "count_only", "q",
    ];
    let mut out: Vec<String> = Vec::new();
    let raw = match raw {
        Some(r) => r,
        None => return out,
    };
    for (k, _v) in form_urlencoded::parse(raw.as_bytes()) {
        let k = k.into_owned();
        let inner = if k.starts_with("filter[") && k.ends_with(']') {
            k[7..k.len() - 1].to_string()
        } else {
            k
        };
        if CTRL.contains(&inner.as_str()) || inner == "logic" || inner == "where" {
            continue;
        }
        if inner.contains("__") || inner.contains('.') || inner.contains('[') {
            continue;
        }
        out.push(inner);
    }
    out
}

fn collect_writes_public(
    t: &Table,
    body: &serde_json::Map<String, Value>,
    for_insert: bool,
) -> Result<Vec<(String, SqlValue)>, ApiError> {
    // 复用 crud 内部的校验逻辑
    crud::collect_writes_public(t, body, for_insert)
}

// ---------------------------------------------------------------------------
// 路由
// ---------------------------------------------------------------------------

/// 跨源放行策略。
///
/// 服务只监听 `127.0.0.1`，但仍显式限定来源，避免本机上其他任意网页
/// 都能读取/改写客户数据。需要放行的三类：
/// - `null`：前端以 `file://` 直接打开构建产物（单文件版）
/// - `127.0.0.1` / `localhost` / `[::1]` 的任意端口：Vite dev / preview
/// - `tauri://localhost` 与 `http(s)://tauri.localhost`：Tauri 2 WebView 的源
///
/// 注意：Tauri 里前端跑在 `tauri://localhost`、API 在 `http://127.0.0.1:<随机端口>`，
/// 两者**依然是跨源**，所以这一层不是开发期的临时措施，而是打包后同样必需的组件。
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _req| {
            let o = origin.to_str().unwrap_or("");
            o == "null"
                || o.starts_with("http://127.0.0.1:")
                || o.starts_with("http://localhost:")
                || o.starts_with("http://[::1]:")
                || o == "tauri://localhost"
                || o == "http://tauri.localhost"
                || o == "https://tauri.localhost"
        }))
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/meta/tables", get(meta_tables))
        .route("/api/v1/meta/tables/{table}", get(meta_table))
        .route("/api/v1/meta/enums", get(meta_enums))
        .route("/api/v1/meta/schema", get(meta_schema))
        .route("/api/v1/meta/health", get(meta_health))
        .route(
            "/api/v1/{table}",
            get(list).post(crud::create).delete(delete_by_filter),
        )
        .route("/api/v1/{table}/count", get(count))
        .route("/api/v1/{table}/exists", get(exists))
        .route("/api/v1/{table}/by", get(get_by_keys))
        .route(
            "/api/v1/{table}/batch",
            post(crud::batch_create)
                .patch(batch_patch)
                .delete(batch_delete),
        )
        .route("/api/v1/{table}/query", post(query))
        .route("/api/v1/{table}/upsert", post(crud::upsert))
        .route("/api/v1/{table}/export", get(export))
        .route(
            "/api/v1/{table}/{id}",
            get(get_by_id).put(put_by_id).patch(patch_by_id).delete(delete_by_id),
        )
        .route(
            "/api/v1/{table}/{column}/{value}",
            get(get_by_column),
        )
        .route(
            "/api/v1/{table}/{id}/children/{child}",
            get(get_children).put(put_children),
        )
        .fallback(fallback)
        .layer(cors_layer())
        .with_state(state)
}

/// 「Web 版」路由：在 API 之外，**由同一个进程托管前端静态产物**。
///
/// 这是「形态 A · 单进程内嵌托管」的核心。一个进程、一个端口，
/// 浏览器访问 `http://<host>:<port>/` 就是完整应用 —— 目标机不需要
/// Node / Rust 任何工具链，也不会派生新的可执行文件。
///
/// 与 [`router`] 的唯一区别就是多了一层静态文件回落，所以「前后端分离部署」
/// （形态 B：前端交给 Nginx/IIS、后端只出 API）继续用 [`router`] 即可，
/// 两种形态共用同一份 API 定义，不会漂移。
///
/// 为什么回落用 `fallback` 而不是 `not_found_service`：
/// 前端是**单页应用**（视图由 `App.vue` 里的 `view` ref 切换，没有真实路由），
/// 但用户仍可能直接敲 `/workbench` 这类路径、或在深链接上按 F5。
/// 磁盘上没有对应文件，必须回落 `index.html` 交给前端接管，否则一刷新就白屏。
///
/// ⚠️ 这里**不能用 `not_found_service`**：它的返回类型是 `ServeDir<SetStatus<F2>>`，
/// 会把回落响应的状态码强制改写成 404。结果就是「内容是对的 index.html，
/// 状态码却是 404」—— 实测确认过（`GET /some/deep/link` → 404 + 519 字节 index.html）。
/// 页面确实能渲染，但语义错了，也容易被前置代理 / 监控当成坏请求。
/// `fallback` 原样透传 `ServeFile` 的 200，才是正确的 SPA 回落。
///
/// 已知行为：未注册的 `/api/...` 路径也会落到 `index.html`（返回 HTML 而非 JSON 404），
/// 因为 `fallback_service` 覆盖了 [`router`] 里那个 JSON 404 handler。
/// 前端只调用已注册的接口，正常运行不会触发；用 curl 调试时看 Content-Type 即可分辨。
pub fn router_with_web(state: AppState, web_dir: impl Into<std::path::PathBuf>) -> Router {
    let dir = web_dir.into();
    let index = dir.join("index.html");
    router(state).fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index)))
}
