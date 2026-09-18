//! 值转换：JSON ⇄ SQLite，以及「数据库行 → JSON 对象」。
//!
//! 规则（对应接口文档 1.3 与第四章）：
//! - 布尔列：接受 0/1、true/false、"0"/"1"，落库为 INTEGER 0/1，返回也是 0/1（不是 true/false）
//! - JSON 列：写入可传对象/数组（序列化落库），也可传已序列化字符串（校验合法性）；
//!   读取默认自动反序列化为对象，`raw=1` 时返回原始字符串
//! - 数值列：接受数字或数字字符串，不接受空串
//! - 空字符串 `""` 是合法值，与 `null`（NULL）区分

use crate::error::{ApiError, FieldError};
use crate::schema::Column;
use rusqlite::types::Value as SqlValue;
use rusqlite::Row;
use serde_json::{Map, Value};

fn type_err(col: &Column, v: &Value) -> ApiError {
    ApiError::validation_many(vec![FieldError::new(&col.name, format!("类型不匹配，期望 {}", col.ty))
        .with_actual(v.clone())])
}

/// JSON 值 → 可绑定的 SQLite 值（含枚举/必填/类型校验）
pub fn json_to_sql(col: &Column, v: &Value) -> Result<SqlValue, ApiError> {
    if v.is_null() {
        if col.not_null {
            return Err(ApiError::validation_many(vec![FieldError::new(
                &col.name,
                "必填字段不可为空",
            )]));
        }
        return Ok(SqlValue::Null);
    }

    if let Some(allowed) = &col.enum_values {
        let ok = matches!(v, Value::String(s) if allowed.iter().any(|a| a == s));
        if !ok {
            return Err(ApiError::validation_many(vec![
                FieldError::new(&col.name, "取值不在枚举内")
                    .with_expected(allowed.clone())
                    .with_actual(v.clone()),
            ]));
        }
    }

    match col.ty.as_str() {
        "BOOLEAN" => {
            let b = match v {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
                Value::String(s) => match s.trim() {
                    "1" | "true" | "TRUE" | "True" => true,
                    "0" | "false" | "FALSE" | "False" | "" => false,
                    _ => return Err(type_err(col, v)),
                },
                _ => return Err(type_err(col, v)),
            };
            Ok(SqlValue::Integer(if b { 1 } else { 0 }))
        }
        "INTEGER" => {
            let i = match v {
                Value::Number(n) => n.as_i64().ok_or_else(|| type_err(col, v))?,
                Value::String(s) => s
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| type_err(col, v))?,
                Value::Bool(b) => {
                    if *b {
                        1
                    } else {
                        0
                    }
                }
                _ => return Err(type_err(col, v)),
            };
            Ok(SqlValue::Integer(i))
        }
        "REAL" => {
            let f = match v {
                Value::Number(n) => n.as_f64().ok_or_else(|| type_err(col, v))?,
                Value::String(s) => s
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| type_err(col, v))?,
                _ => return Err(type_err(col, v)),
            };
            Ok(SqlValue::Real(f))
        }
        _ => {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
                Value::Array(_) | Value::Object(_) => {
                    serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
                }
                Value::Null => return Ok(SqlValue::Null),
            };
            if col.json {
                if let Value::String(raw) = v {
                    if !raw.trim().is_empty() {
                        serde_json::from_str::<Value>(raw).map_err(|_| ApiError::bad_json(&col.name))?;
                    }
                }
            }
            Ok(SqlValue::Text(s))
        }
    }
}

/// JSON 标量 → 可直接绑定的 SQLite 值。
///
/// 为什么不能沿用 `json_to_sql`：JSON 列在 DDL 上是 `TEXT` 类型，走 `json_to_sql`
/// 会把数字 `1` 转成文本 `"1"`。而 `json_extract` 对**标量**返回的是 INTEGER/REAL，
/// 拿它跟文本比会走 SQLite 的类型排序 —— 不同存储类之间 **INTEGER/REAL 永远小于 TEXT**，
/// 于是 `json_array_length(...) >= '1'` 恒为假、`json_eq` 对数字标量恒不命中。
/// 这里按「SQLite `json_extract` 会返回什么类型」来选绑定类型，跟它对齐。
pub fn json_scalar_to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        // 数组/对象：json_extract 会把整段 JSON 以文本返回
        Value::Array(_) | Value::Object(_) => {
            SqlValue::Text(serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()))
        }
    }
}

/// SQLite 值 → JSON（读取路径）
pub fn sql_to_json(col: &Column, v: SqlValue, raw_json: bool) -> Value {
    match v {
        SqlValue::Null => Value::Null,
        SqlValue::Integer(i) => Value::from(i),
        SqlValue::Real(f) => Value::from(f),
        SqlValue::Text(s) => {
            if col.json && !raw_json && !s.trim().is_empty() {
                serde_json::from_str::<Value>(&s).unwrap_or(Value::String(s))
            } else {
                Value::String(s)
            }
        }
        SqlValue::Blob(_) => Value::Null,
    }
}

/// 从结果行按序号取列值并转 JSON
pub fn col_from_row(
    col: &Column,
    row: &Row<'_>,
    idx: usize,
    raw_json: bool,
) -> rusqlite::Result<Value> {
    let v: SqlValue = row.get(idx)?;
    Ok(sql_to_json(col, v, raw_json))
}

/// 空对象（用于初始化 expand 出来的嵌套结构）
pub fn empty_object() -> Map<String, Value> {
    Map::new()
}

/// 解析 JSON 请求体为对象；非对象直接报 40002
pub fn body_to_object(body: &Value) -> Result<&Map<String, Value>, ApiError> {
    body.as_object()
        .ok_or_else(|| ApiError::param("请求体必须是 JSON 对象"))
}

/// 把用户传入的排序方向规范化
pub fn normalize_direction(d: &str) -> Result<(&'static str, bool), ApiError> {
    match d.to_ascii_lowercase().as_str() {
        "asc" => Ok(("ASC", false)),
        "desc" => Ok(("DESC", false)),
        "asc_nulls_last" => Ok(("ASC", true)),
        "desc_nulls_last" => Ok(("DESC", true)),
        "asc_nulls_first" => Ok(("ASC", false)),
        "desc_nulls_first" => Ok(("DESC", false)),
        other => Err(ApiError::param(format!("非法排序方向: {other}"))),
    }
}
