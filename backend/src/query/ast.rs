//! 查询条件 AST 与操作符 → SQL 渲染。
//!
//! 安全要点：本模块只把 **白名单内的列名** 映射为 SQL 片段（结构），
//! 所有用户提供的**值**一律进入 `params` 走 `?` 占位符，不存在字符串拼接进 SQL 的路径。

use crate::error::ApiError;
use crate::schema::{Registry, Table};
use rusqlite::types::Value as SqlValue;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    NLike,
    Prefix,
    Suffix,
    In,
    Nin,
    Between,
    IsNull,
    IsNotNull,
    JsonContains,
    JsonHasAll,
    JsonEq,
    JsonLike,
    JsonLenGte,
}

impl Op {
    pub fn from_suffix(s: &str) -> Option<Op> {
        Some(match s.to_ascii_lowercase().as_str() {
            "eq" => Op::Eq,
            "ne" | "not" => Op::Ne,
            "gt" => Op::Gt,
            "gte" | "ge" => Op::Gte,
            "lt" => Op::Lt,
            "lte" | "le" => Op::Lte,
            "like" => Op::Like,
            "nlike" => Op::NLike,
            "prefix" => Op::Prefix,
            "suffix" => Op::Suffix,
            "in" => Op::In,
            "nin" => Op::Nin,
            "between" => Op::Between,
            "null" | "isnull" => Op::IsNull,
            "notnull" | "isnotnull" => Op::IsNotNull,
            "json_contains" => Op::JsonContains,
            "json_has_all" => Op::JsonHasAll,
            "json_eq" => Op::JsonEq,
            "json_like" => Op::JsonLike,
            "json_len_gte" => Op::JsonLenGte,
            _ => return None,
        })
    }

    /// 供 POST /query 的 JSON 节点使用（op 字段）
    pub fn from_name(s: &str) -> Option<Op> {
        Op::from_suffix(s)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Op::Eq => "eq",
            Op::Ne => "ne",
            Op::Gt => "gt",
            Op::Gte => "gte",
            Op::Lt => "lt",
            Op::Lte => "lte",
            Op::Like => "like",
            Op::NLike => "nlike",
            Op::Prefix => "prefix",
            Op::Suffix => "suffix",
            Op::In => "in",
            Op::Nin => "nin",
            Op::Between => "between",
            Op::IsNull => "null",
            Op::IsNotNull => "notnull",
            Op::JsonContains => "json_contains",
            Op::JsonHasAll => "json_has_all",
            Op::JsonEq => "json_eq",
            Op::JsonLike => "json_like",
            Op::JsonLenGte => "json_len_gte",
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(
            self,
            Op::JsonContains | Op::JsonHasAll | Op::JsonEq | Op::JsonLike | Op::JsonLenGte
        )
    }

    /// 该操作符需要几个值
    pub fn arity(&self) -> (usize, Option<usize>) {
        match self {
            Op::IsNull | Op::IsNotNull => (0, Some(0)),
            Op::Between => (2, Some(2)),
            Op::JsonContains | Op::JsonEq | Op::JsonLike | Op::JsonLenGte => (1, Some(1)),
            Op::JsonHasAll | Op::In | Op::Nin => (1, None),
            _ => (1, Some(1)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Leaf {
    /// 原始字符串，可能是 `col`、`json_col.路径` 或 `关联表.col`
    pub column: String,
    pub op: Op,
    pub values: Vec<Value>,
}

impl Leaf {
    pub fn new(column: impl Into<String>, op: Op, values: Vec<Value>) -> Self {
        Self {
            column: column.into(),
            op,
            values,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cond {
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Not(Box<Cond>),
    Leaf(Leaf),
}

impl Cond {
    pub fn and(mut v: Vec<Cond>) -> Option<Cond> {
        match v.len() {
            0 => None,
            1 => Some(v.remove(0)),
            _ => Some(Cond::And(v)),
        }
    }

    pub fn or(mut v: Vec<Cond>) -> Option<Cond> {
        match v.len() {
            0 => None,
            1 => Some(v.remove(0)),
            _ => Some(Cond::Or(v)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColRef {
    Base(String),
    Json { column: String, path: String },
    Related { table: String, column: String },
}

/// 把 `raw` 解析为列引用。
pub fn resolve(
    t: &Table,
    reg: &Registry,
    raw: &str,
    op: Op,
) -> Result<ColRef, ApiError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(ApiError::param("查询列名不能为空"));
    }
    let mut it = raw.splitn(2, '.');
    let head = it.next().unwrap_or("");
    let rest = it.next();

    match rest {
        None => {
            t.require_column(head)?;
            if op.is_json() {
                Ok(ColRef::Json {
                    column: head.to_string(),
                    path: "$".to_string(),
                })
            } else {
                Ok(ColRef::Base(head.to_string()))
            }
        }
        Some(tail) => {
            if tail.is_empty() {
                return Err(ApiError::param(format!("列路径不完整: {raw}")));
            }
            // JSON 路径优先（json_* 操作符 + 本表 JSON 列）
            if op.is_json() {
                if let Some(c) = t.column(head) {
                    if c.json {
                        return Ok(ColRef::Json {
                            column: head.to_string(),
                            path: json_path(tail)?,
                        });
                    }
                }
            }
            // 关联表列
            if let Some(b) = t.belongs_to.iter().find(|b| b.table == head) {
                let rt = reg.table(&b.table)?;
                rt.require_column(tail)?;
                return Ok(ColRef::Related {
                    table: b.table.clone(),
                    column: tail.to_string(),
                });
            }
            // 兜底：head 是 JSON 列
            if let Some(c) = t.column(head) {
                if c.json {
                    return Ok(ColRef::Json {
                        column: head.to_string(),
                        path: json_path(tail)?,
                    });
                }
            }
            Err(ApiError::unknown_column(&t.name, raw, t.column_names()))
        }
    }
}

/// `金融.rank` → `$.金融.rank`；`$[0].name` 原样保留。严格校验，拒绝引号/分号等。
fn json_path(rest: &str) -> Result<String, ApiError> {
    let raw = if rest.starts_with('$') {
        rest.to_string()
    } else {
        format!("$.{rest}")
    };
    for ch in raw.chars() {
        let ok = ch == '$'
            || ch == '.'
            || ch == '['
            || ch == ']'
            || ch == '_'
            || ch.is_alphanumeric()
            || ('\u{4e00}'..='\u{9fff}').contains(&ch);
        if !ok {
            return Err(ApiError::param(format!("非法的 JSON 路径: {rest}")));
        }
    }
    Ok(raw)
}

/// 生成列表达式的 SQL 片段；可能向 params 追加前缀参数（JSON 路径）。
fn col_expr(
    t: &Table,
    reg: &Registry,
    base_alias: &str,
    raw: &str,
    op: Op,
    params: &mut Vec<SqlValue>,
) -> Result<String, ApiError> {
    match resolve(t, reg, raw, op)? {
        ColRef::Base(c) => Ok(format!("\"{base_alias}\".\"{c}\"")),
        ColRef::Related { table, column } => {
            // 关联表别名 = 表名（build 阶段保证 JOIN 已建立）
            let bt = reg.table(&table)?;
            bt.require_column(&column)?;
            Ok(format!("\"{table}\".\"{column}\""))
        }
        ColRef::Json { column, path } => {
            let c = t.require_column(&column)?;
            if !c.json {
                return Err(ApiError::validation_many(vec![
                    crate::error::FieldError::new(&column, "该列不是 JSON 列，不能使用 json_* 操作符"),
                ]));
            }
            // 表达式自身消费一个「路径」参数，调用方需保证参数顺序一致
            params.push(SqlValue::Text(path));
            if matches!(op, Op::JsonContains | Op::JsonHasAll | Op::JsonLike) {
                // 供 EXISTS (SELECT 1 FROM <expr> je ...) 使用
                Ok(format!("json_each(\"{base_alias}\".\"{column}\", ?)"))
            } else {
                Ok(format!("json_extract(\"{base_alias}\".\"{column}\", ?)"))
            }
        }
    }
}

/// 任意 JSON 值 → 文本绑定值
fn as_text(v: &Value) -> SqlValue {
    match v {
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Null => SqlValue::Null,
        other => SqlValue::Text(other.to_string()),
    }
}

/// JSON 数组元素比较统一转成文本，兼容数组里存数字或字符串两种情形
fn value_as_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// 渲染单个叶子条件
pub fn render_leaf(
    t: &Table,
    reg: &Registry,
    base_alias: &str,
    leaf: &Leaf,
    params: &mut Vec<SqlValue>,
) -> Result<String, ApiError> {
    let op = leaf.op;
    let (min, max) = op.arity();
    if leaf.values.len() < min || max.map(|m| leaf.values.len() > m).unwrap_or(false) {
        return Err(ApiError::param(format!(
            "操作符 {} 需要 {} 个值，实际收到 {}",
            op.name(),
            match (min, max) {
                (a, Some(b)) if a == b => a.to_string(),
                (a, Some(b)) => format!("{a}~{b}"),
                (a, None) => format!("至少 {a}"),
            },
            leaf.values.len()
        )));
    }

    // 取列定义用于类型校验（用于把 JSON 值转成绑定值）
    let leaf_col = resolve(t, reg, &leaf.column, op)?;
    let col_def = match &leaf_col {
        ColRef::Base(c) | ColRef::Json { column: c, .. } => Some(t.require_column(c)?),
        ColRef::Related { table, column } => Some(reg.table(table)?.require_column(column)?),
    };

    // 字符串匹配类操作符（like / prefix / suffix / json_contains）不做枚举与类型校验，
    // 否则对枚举列做全局关键词搜索会被误判为"取值不在枚举内"。
    let text_only = matches!(
        op,
        Op::Like | Op::NLike | Op::Prefix | Op::Suffix | Op::JsonContains | Op::JsonHasAll | Op::JsonLike
    );
    let to_sql = |v: &Value| -> Result<SqlValue, ApiError> {
        if text_only {
            return Ok(as_text(v));
        }
        match col_def {
            Some(c) => crate::value::json_to_sql(c, v),
            None => Ok(as_text(v)),
        }
    };

    match op {
        Op::IsNull | Op::IsNotNull => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            Ok(format!(
                "{expr} IS {}NULL",
                if op == Op::IsNull { "" } else { "NOT " }
            ))
        }
        Op::JsonContains | Op::JsonHasAll => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            let mut out = Vec::new();
            for v in &leaf.values {
                params.push(SqlValue::Text(value_as_text(v)));
                out.push(format!(
                    "EXISTS (SELECT 1 FROM {expr} je WHERE CAST(je.value AS TEXT) = ?)"
                ));
            }
            Ok(format!("({})", out.join(" AND ")))
        }
        Op::JsonLike => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            Ok(format!(
                "EXISTS (SELECT 1 FROM {expr} je WHERE CAST(je.value AS TEXT) LIKE '%' || ? || '%')"
            ))
        }
        Op::JsonLenGte => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            // 必须按数字绑定：JSON 列是 TEXT，走 to_sql 会变成 '1'，
            // 而 SQLite 里 INTEGER < TEXT 恒成立，`>= '1'` 会永远为假。
            params.push(crate::value::json_scalar_to_sql(&leaf.values[0]));
            Ok(format!("json_array_length({expr}) >= ?"))
        }
        Op::JsonEq => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            // 同理：json_extract 对标量返回 INTEGER/REAL，绑定也要用数字
            params.push(crate::value::json_scalar_to_sql(&leaf.values[0]));
            Ok(format!("{expr} = ?"))
        }
        Op::Eq => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            Ok(format!("{expr} = ?"))
        }
        Op::Ne => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            Ok(format!("{expr} <> ?"))
        }
        Op::Gt | Op::Gte | Op::Lt | Op::Lte => {
            let sym = match op {
                Op::Gt => ">",
                Op::Gte => ">=",
                Op::Lt => "<",
                _ => "<=",
            };
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            Ok(format!("{expr} {sym} ?"))
        }
        Op::Like | Op::NLike => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            let neg = if op == Op::NLike { "NOT " } else { "" };
            Ok(format!("{expr} {neg}LIKE '%' || ? || '%'"))
        }
        Op::Prefix | Op::Suffix => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            if op == Op::Prefix {
                Ok(format!("{expr} LIKE ? || '%'"))
            } else {
                Ok(format!("{expr} LIKE '%' || ?"))
            }
        }
        Op::In | Op::Nin => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            let vals: Vec<Value> = if leaf.values.len() == 1 && leaf.values[0].is_array() {
                leaf.values[0].as_array().cloned().unwrap_or_default()
            } else {
                leaf.values.clone()
            };
            if vals.is_empty() {
                // IN () 在 SQLite 里语法非法，用恒假/恒真处理
                return Ok(if op == Op::In {
                    "1 = 0".to_string()
                } else {
                    "1 = 1".to_string()
                });
            }
            let mut ph = Vec::new();
            for v in &vals {
                params.push(to_sql(v)?);
                ph.push("?");
            }
            let neg = if op == Op::Nin { "NOT " } else { "" };
            Ok(format!("{expr} {neg}IN ({})", ph.join(", ")))
        }
        Op::Between => {
            let expr = col_expr(t, reg, base_alias, &leaf.column, op, params)?;
            params.push(to_sql(&leaf.values[0])?);
            params.push(to_sql(&leaf.values[1])?);
            Ok(format!("{expr} BETWEEN ? AND ?"))
        }
    }
}

/// 渲染整棵条件树
pub fn render_cond(
    t: &Table,
    reg: &Registry,
    base_alias: &str,
    cond: &Cond,
    params: &mut Vec<SqlValue>,
) -> Result<String, ApiError> {
    match cond {
        Cond::Leaf(l) => render_leaf(t, reg, base_alias, l, params),
        Cond::And(v) => {
            let mut parts = Vec::new();
            for c in v {
                parts.push(render_cond(t, reg, base_alias, c, params)?);
            }
            if parts.is_empty() {
                return Ok("1 = 1".to_string());
            }
            Ok(format!("({})", parts.join(" AND ")))
        }
        Cond::Or(v) => {
            let mut parts = Vec::new();
            for c in v {
                parts.push(render_cond(t, reg, base_alias, c, params)?);
            }
            if parts.is_empty() {
                return Ok("1 = 0".to_string());
            }
            Ok(format!("({})", parts.join(" OR ")))
        }
        Cond::Not(c) => Ok(format!("(NOT {})", render_cond(t, reg, base_alias, c, params)?)),
    }
}
