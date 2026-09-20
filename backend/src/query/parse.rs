//! 查询参数解析：GET 查询串 → QuerySpec，POST /query 的 JSON 条件树 → QuerySpec。
//!
//! 两种入口最终产出同一个 `QuerySpec`，交给 `build` 模块编译成**参数化 SQL**。
//! 这里只做解析与白名单校验，绝不拼接值进 SQL。

use crate::error::ApiError;
use crate::query::ast::{resolve, Cond, Leaf, Op};
use crate::schema::{Column, Registry, Table};
use serde_json::Value;
use std::collections::BTreeMap;

pub const DEFAULT_PAGE_SIZE: i64 = 20;
pub const MAX_PAGE_SIZE: i64 = 200;
pub const MAX_BATCH: usize = 1000;
pub const MAX_DEPTH: usize = 5;
pub const MAX_PAGE: i64 = 10_000;

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub direction: String,
    pub nulls_last: bool,
}

#[derive(Debug, Clone)]
pub struct Agg {
    pub func: String,
    pub column: Option<String>,
    pub alias: String,
}

#[derive(Debug, Clone, Default)]
pub struct QuerySpec {
    pub select: Vec<String>,
    pub cond: Option<Cond>,
    pub q: Option<String>,
    pub order_by: Vec<OrderBy>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub distinct: bool,
    pub group_by: Vec<String>,
    pub aggregates: Vec<Agg>,
    pub expand: Vec<String>,
    pub children: Vec<String>,
    pub raw_json: bool,
    pub count_only: bool,
}

impl QuerySpec {
    /// 生效的分页参数（page/page_size 与 limit/offset 二选一）
    pub fn effective_limit(&self) -> i64 {
        if let Some(l) = self.limit {
            return l.clamp(1, MAX_PAGE_SIZE);
        }
        self.page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE)
    }

    pub fn effective_offset(&self) -> i64 {
        if let Some(o) = self.offset {
            return o.max(0);
        }
        let page = self.page.unwrap_or(1).clamp(1, MAX_PAGE);
        (page - 1) * self.effective_limit()
    }

    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).clamp(1, MAX_PAGE)
    }

    pub fn page_size(&self) -> i64 {
        self.effective_limit()
    }
}

fn truthy(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y" | "on")
}

fn parse_i64(s: &str, field: &str) -> Result<i64, ApiError> {
    s.trim()
        .parse::<i64>()
        .map_err(|_| ApiError::param(format!("{field} 必须是整数，收到: {s}")))
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// 键名解析
// ---------------------------------------------------------------------------

/// 解析 `col__op` / `col` / `关联表.col`，返回 (列表达式, 操作符)，并做白名单校验
pub fn split_op(t: &Table, reg: &Registry, key: &str) -> Result<(String, Op), ApiError> {
    if t.column(key).is_some() {
        return Ok((key.to_string(), Op::Eq));
    }
    if let Some(idx) = key.rfind("__") {
        let col = &key[..idx];
        let opname = &key[idx + 2..];
        if let Some(op) = Op::from_suffix(opname) {
            resolve(t, reg, col, op)?;
            return Ok((col.to_string(), op));
        }
        // 后缀不是合法操作符：可能列名本身就带双下划线，继续往下走
    }
    resolve(t, reg, key, Op::Eq)?;
    Ok((key.to_string(), Op::Eq))
}

/// 找到表达式对应的列定义（用于类型转换）
fn coldef<'a>(t: &'a Table, reg: &'a Registry, raw: &str, op: Op) -> Option<&'a Column> {
    match resolve(t, reg, raw, op).ok()? {
        crate::query::ast::ColRef::Base(c) | crate::query::ast::ColRef::Json { column: c, .. } => {
            t.column(&c)
        }
        crate::query::ast::ColRef::Related { table, column } => {
            reg.table(&table).ok()?.column(&column)
        }
    }
}

fn conv(s: &str, col: Option<&Column>, numeric_json: bool) -> Value {
    if numeric_json {
        if let Ok(i) = s.trim().parse::<i64>() {
            return Value::from(i);
        }
        if let Ok(f) = s.trim().parse::<f64>() {
            return Value::from(f);
        }
        return Value::String(s.to_string());
    }
    match col.map(|c| c.ty.as_str()) {
        Some("INTEGER") => s
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(s.to_string())),
        Some("BOOLEAN") => {
            if truthy(s) {
                Value::from(1)
            } else if matches!(s.trim(), "0" | "false" | "no" | "off") {
                Value::from(0)
            } else {
                Value::String(s.to_string())
            }
        }
        Some("REAL") => s
            .trim()
            .parse::<f64>()
            .map(|f| serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::String(s.to_string())))
            .unwrap_or_else(|_| Value::String(s.to_string())),
        _ => Value::String(s.to_string()),
    }
}

/// 由 `col__op` 键 + 原始字符串值构造叶子条件
pub fn leaf_from_key(
    t: &Table,
    reg: &Registry,
    key: &str,
    raw: &[String],
) -> Result<Cond, ApiError> {
    let (col, op0) = split_op(t, reg, key)?;
    // 同名参数出现多次 → IN 语义（见接口文档 6.1）
    let op = if op0 == Op::Eq && raw.len() > 1 { Op::In } else { op0 };
    let cd = coldef(t, reg, &col, op);
    // json_contains / json_has_all 比较数组元素，统一按文本处理
    let as_text = matches!(op, Op::JsonContains | Op::JsonHasAll);
    // json_eq / json_len_gte / json_like 是对标量比较，数字优先
    let numeric_json = op.is_json() && !as_text;

    let values: Vec<Value> = match op {
        Op::IsNull | Op::IsNotNull => Vec::new(),
        Op::In | Op::Nin | Op::JsonHasAll => {
            let mut out = Vec::new();
            for r in raw {
                for piece in r.split(',') {
                    if !piece.trim().is_empty() {
                        out.push(conv(piece, cd, false));
                    }
                }
            }
            out
        }
        Op::Between => {
            let mut out = Vec::new();
            for r in raw {
                for piece in r.split(|c| c == ',' || c == '~') {
                    if !piece.trim().is_empty() {
                        out.push(conv(piece, cd, numeric_json));
                    }
                }
            }
            out
        }
        _ => {
            let first = raw.first().ok_or_else(|| {
                ApiError::param(format!("列参数 {key} 缺少取值"))
            })?;
            vec![conv(first, cd, numeric_json)]
        }
    };

    Ok(Cond::Leaf(Leaf::new(col, op, values)))
}

/// 解析 `or[..]` / `and[..]` 的键。
///
/// 注意：进到这里时，外层方括号**已经被 `unwrap_bracket` 剥掉**了，所以实际会收到三种形态：
///   `or[列名]`     → "列名"        → 下标 0，列名「列名」
///   `or[0][列名]`  → "0][列名"     → 下标 0，列名「列名」
///   "[0][列名]"    → 原始形态，保险起见也一并支持
/// 之前只认前两种中的第一种，`or[0][列名]` 会走到最后一行、把 `0][列名` 整个当列名，
/// 于是报「表 X 上不存在列 0][列名」。这就是分组联合查询一直用不了的根因。
fn parse_group_key(inner: &str) -> (usize, String) {
    let s = inner.trim();

    // 形态二：`0][列名`
    if let Some(pos) = s.find("][") {
        if let Ok(idx) = s[..pos].parse::<usize>() {
            let col = s[pos + 2..].trim().trim_end_matches(']').trim();
            return (idx, col.to_string());
        }
    }

    // 形态三：`[0][列名]`
    if let Some(stripped) = s.strip_prefix('[') {
        if let Some(end) = stripped.find(']') {
            let idx = stripped[..end].parse::<usize>().unwrap_or(0);
            let rest = stripped[end + 1..].trim();
            let rest = rest.strip_prefix('[').unwrap_or(rest);
            let rest = rest.strip_suffix(']').unwrap_or(rest);
            return (idx, rest.trim().to_string());
        }
    }

    // 形态一：`列名`
    (0, s.trim_end_matches(']').trim().to_string())
}

fn unwrap_bracket<'a>(key: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}[");
    if key.starts_with(&prefix) && key.ends_with(']') {
        Some(&key[prefix.len()..key.len() - 1])
    } else {
        None
    }
}

/// 解析 GET 查询串
pub fn spec_from_query(
    t: &Table,
    reg: &Registry,
    raw: Option<&str>,
) -> Result<QuerySpec, ApiError> {
    let mut spec = QuerySpec::default();
    let raw = match raw {
        Some(r) if !r.trim().is_empty() => r,
        _ => return Ok(spec),
    };

    // 聚合同名参数（保留重复出现的值）
    let mut kv: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in form_urlencoded::parse(raw.as_bytes()) {
        let k = k.into_owned();
        let v = v.into_owned();
        if let Some(slot) = kv.iter_mut().find(|(kk, _)| *kk == k) {
            slot.1.push(v);
        } else {
            kv.push((k, vec![v]));
        }
    }

    let mut flat: Vec<(String, Vec<String>)> = Vec::new();
    let mut or_groups: BTreeMap<usize, Vec<(String, Vec<String>)>> = BTreeMap::new();
    let mut and_groups: BTreeMap<usize, Vec<(String, Vec<String>)>> = BTreeMap::new();
    let mut logic_or = false;
    let mut where_expr: Option<String> = None;

    for (key, vals) in kv {
        match key.as_str() {
            "page" => spec.page = Some(parse_i64(&vals[0], "page")?),
            "page_size" => {
                let v = parse_i64(&vals[0], "page_size")?;
                if v > MAX_PAGE_SIZE {
                    return Err(ApiError::over_limit(format!(
                        "page_size 上限为 {MAX_PAGE_SIZE}，收到 {v}"
                    )));
                }
                spec.page_size = Some(v);
            }
            "limit" => spec.limit = Some(parse_i64(&vals[0], "limit")?),
            "offset" => spec.offset = Some(parse_i64(&vals[0], "offset")?),
            "order_by" => spec.order_by = parse_order_by(t, reg, &vals)?,
            "select" => {
                for v in &vals {
                    spec.select.extend(split_csv(v));
                }
            }
            "group_by" => {
                for v in &vals {
                    spec.group_by.extend(split_csv(v));
                }
            }
            "agg" | "aggregates" => {
                for v in &vals {
                    spec.aggregates.extend(parse_agg(t, reg, v)?);
                }
            }
            "expand" => {
                for v in &vals {
                    spec.expand.extend(split_csv(v));
                }
            }
            "children" => {
                for v in &vals {
                    spec.children.extend(split_csv(v));
                }
            }
            "distinct" => spec.distinct = truthy(&vals[0]),
            "raw" => spec.raw_json = truthy(&vals[0]),
            "count_only" => spec.count_only = truthy(&vals[0]),
            "q" => spec.q = Some(vals[0].clone()),
            "logic" => logic_or = vals[0].trim().eq_ignore_ascii_case("or"),
            "where" => where_expr = Some(vals[0].clone()),
            // 导出等接口的辅助参数，不参与过滤
            "format" | "filename" | "_t" | "ts" => {}
            other => {
                if let Some(inner) = unwrap_bracket(other, "filter") {
                    flat.push((inner.to_string(), vals));
                } else if let Some(inner) = unwrap_bracket(other, "or") {
                    let (idx, k) = parse_group_key(inner);
                    or_groups.entry(idx).or_default().push((k, vals));
                } else if let Some(inner) = unwrap_bracket(other, "and") {
                    let (idx, k) = parse_group_key(inner);
                    and_groups.entry(idx).or_default().push((k, vals));
                } else {
                    flat.push((other.to_string(), vals));
                }
            }
        }
    }

    let mut and_conds: Vec<Cond> = Vec::new();

    if logic_or {
        let mut ls = Vec::new();
        for (k, v) in &flat {
            ls.push(leaf_from_key(t, reg, k, v)?);
        }
        if let Some(c) = Cond::or(ls) {
            and_conds.push(c);
        }
    } else {
        for (k, v) in &flat {
            and_conds.push(leaf_from_key(t, reg, k, v)?);
        }
    }

    for (_idx, items) in or_groups {
        let mut ls = Vec::new();
        for (k, v) in &items {
            ls.push(leaf_from_key(t, reg, k, v)?);
        }
        if let Some(c) = Cond::or(ls) {
            and_conds.push(c);
        }
    }
    for (_idx, items) in and_groups {
        let mut ls = Vec::new();
        for (k, v) in &items {
            ls.push(leaf_from_key(t, reg, k, v)?);
        }
        and_conds.push(Cond::And(ls));
    }

    if let Some(w) = where_expr {
        and_conds.push(crate::query::where_dsl::parse(t, reg, &w)?);
    }

    validate_select(t, reg, &spec)?;
    spec.cond = Cond::and(and_conds);
    Ok(spec)
}

fn validate_select(t: &Table, reg: &Registry, spec: &QuerySpec) -> Result<(), ApiError> {
    for c in &spec.select {
        resolve(t, reg, c, Op::Eq)?;
    }
    for c in &spec.group_by {
        resolve(t, reg, c, Op::Eq)?;
    }
    Ok(())
}

fn parse_order_by(
    t: &Table,
    reg: &Registry,
    vals: &[String],
) -> Result<Vec<OrderBy>, ApiError> {
    let mut out = Vec::new();
    for v in vals {
        for item in split_csv(v) {
            let (col, dir) = if let Some(rest) = item.strip_prefix('-') {
                (rest.to_string(), "desc".to_string())
            } else if let Some((c, d)) = item.split_once(':') {
                (c.trim().to_string(), d.trim().to_string())
            } else {
                (item.clone(), "asc".to_string())
            };
            resolve(t, reg, &col, Op::Eq)?;
            let (dir_sql, nulls_last) = crate::value::normalize_direction(&dir)?;
            out.push(OrderBy {
                column: col,
                direction: dir_sql.to_string(),
                nulls_last,
            });
            if out.len() > 5 {
                return Err(ApiError::over_limit("order_by 最多 5 列"));
            }
        }
    }
    Ok(out)
}

const AGG_FUNCS: [&str; 6] = ["count", "count_distinct", "sum", "avg", "min", "max"];

/// `sum(amount):total,count(*):cnt`
fn parse_agg(t: &Table, reg: &Registry, s: &str) -> Result<Vec<Agg>, ApiError> {
    let mut out = Vec::new();
    for item in split_csv(s) {
        let (call, alias) = match item.split_once(':') {
            Some((c, a)) => (c.trim().to_string(), a.trim().to_string()),
            None => (item.clone(), String::new()),
        };
        let open = call.find('(').ok_or_else(|| {
            ApiError::param(format!("聚合表达式格式应为 func(col)[:alias]，收到: {item}"))
        })?;
        let close = call.rfind(')').ok_or_else(|| {
            ApiError::param(format!("聚合表达式缺少右括号: {item}"))
        })?;
        let func = call[..open].trim().to_ascii_lowercase();
        let inner = call[open + 1..close].trim().to_string();
        if !AGG_FUNCS.contains(&func.as_str()) {
            return Err(ApiError::param(format!(
                "不支持的聚合函数 {func}，可用: {}",
                AGG_FUNCS.join("/")
            )));
        }
        let column = if inner == "*" || inner.is_empty() {
            None
        } else {
            resolve(t, reg, &inner, Op::Eq)?;
            Some(inner)
        };
        let alias = if alias.is_empty() {
            match &column {
                Some(c) => format!("{func}_{}", c.replace('.', "_")),
                None => func.clone(),
            }
        } else {
            alias
        };
        out.push(Agg {
            func,
            column,
            alias,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// POST /{table}/query 的 JSON 条件树
// ---------------------------------------------------------------------------

pub fn spec_from_body(t: &Table, reg: &Registry, body: &Value) -> Result<QuerySpec, ApiError> {
    let obj = body
        .as_object()
        .ok_or_else(|| ApiError::param("请求体必须是 JSON 对象"))?;
    let mut spec = QuerySpec::default();

    if let Some(v) = obj.get("select") {
        spec.select = str_list(v, "select")?;
    }
    if let Some(v) = obj.get("group_by") {
        spec.group_by = str_list(v, "group_by")?;
    }
    if let Some(v) = obj.get("expand") {
        spec.expand = str_list(v, "expand")?;
    }
    if let Some(v) = obj.get("children") {
        spec.children = str_list(v, "children")?;
    }
    if let Some(v) = obj.get("distinct") {
        spec.distinct = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = obj.get("raw") {
        spec.raw_json = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = obj.get("count_only") {
        spec.count_only = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = obj.get("q").and_then(|v| v.as_str()) {
        spec.q = Some(v.to_string());
    }
    if let Some(v) = obj.get("page") {
        spec.page = Some(v.as_i64().ok_or_else(|| ApiError::param("page 必须是整数"))?);
    }
    if let Some(v) = obj.get("page_size") {
        let n = v.as_i64().ok_or_else(|| ApiError::param("page_size 必须是整数"))?;
        if n > MAX_PAGE_SIZE {
            return Err(ApiError::over_limit(format!("page_size 上限为 {MAX_PAGE_SIZE}")));
        }
        spec.page_size = Some(n);
    }
    if let Some(v) = obj.get("limit") {
        spec.limit = Some(v.as_i64().ok_or_else(|| ApiError::param("limit 必须是整数"))?);
    }
    if let Some(v) = obj.get("offset") {
        spec.offset = Some(v.as_i64().ok_or_else(|| ApiError::param("offset 必须是整数"))?);
    }

    if let Some(ob) = obj.get("order_by") {
        let mut list = Vec::new();
        match ob {
            Value::Array(arr) => {
                for item in arr {
                    match item {
                        Value::String(s) => {
                            let mut one = parse_order_by(t, reg, &[s.clone()])?;
                            list.append(&mut one);
                        }
                        Value::Object(_) => {
                            let col = item
                                .get("column")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| ApiError::param("order_by 项缺少 column"))?;
                            let dir = item
                                .get("direction")
                                .and_then(|v| v.as_str())
                                .unwrap_or("asc");
                            let mut one =
                                parse_order_by(t, reg, &[format!("{col}:{dir}")])?;
                            list.append(&mut one);
                        }
                        _ => return Err(ApiError::param("order_by 项格式非法")),
                    }
                }
            }
            Value::String(s) => list = parse_order_by(t, reg, &[s.clone()])?,
            _ => return Err(ApiError::param("order_by 必须是数组或字符串")),
        }
        spec.order_by = list;
    }

    if let Some(ag) = obj.get("aggregates") {
        for item in ag.as_array().unwrap_or(&Vec::new()) {
            match item {
                Value::String(s) => spec.aggregates.extend(parse_agg(t, reg, s)?),
                Value::Object(_) => {
                    let func = item
                        .get("func")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ApiError::param("aggregates 项缺少 func"))?;
                    let column = item.get("column").and_then(|v| v.as_str()).unwrap_or("*");
                    let alias = item.get("alias").and_then(|v| v.as_str()).unwrap_or("");
                    let expr = format!("{func}({column}):{alias}");
                    spec.aggregates.extend(parse_agg(t, reg, &expr)?);
                }
                _ => return Err(ApiError::param("aggregates 项格式非法")),
            }
        }
    }

    if let Some(w) = obj.get("where") {
        spec.cond = Some(parse_node(t, reg, w, 0)?);
    }

    validate_select(t, reg, &spec)?;
    Ok(spec)
}

fn str_list(v: &Value, field: &str) -> Result<Vec<String>, ApiError> {
    match v {
        Value::Array(a) => a
            .iter()
            .map(|x| {
                x.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| ApiError::param(format!("{field} 必须是字符串数组")))
            })
            .collect(),
        Value::String(s) => Ok(split_csv(s)),
        _ => Err(ApiError::param(format!("{field} 必须是字符串数组"))),
    }
}

fn parse_node(
    t: &Table,
    reg: &Registry,
    node: &Value,
    depth: usize,
) -> Result<Cond, ApiError> {
    if depth > MAX_DEPTH {
        return Err(ApiError::over_limit(format!("条件嵌套深度上限为 {MAX_DEPTH}")));
    }
    // 数组 = AND
    if let Some(arr) = node.as_array() {
        let mut cs = Vec::new();
        for n in arr {
            cs.push(parse_node(t, reg, n, depth + 1)?);
        }
        return Ok(Cond::And(cs));
    }
    let obj = node
        .as_object()
        .ok_or_else(|| ApiError::param("条件节点必须是对象或数组"))?;

    if let Some(opname) = obj.get("op").and_then(|v| v.as_str()) {
        let lower = opname.to_ascii_lowercase();
        if matches!(lower.as_str(), "and" | "or" | "not") {
            let conds = obj
                .get("conditions")
                .and_then(|v| v.as_array())
                .ok_or_else(|| ApiError::param(format!("逻辑节点 {lower} 缺少 conditions 数组")))?;
            let mut cs = Vec::new();
            for c in conds {
                cs.push(parse_node(t, reg, c, depth + 1)?);
            }
            return Ok(match lower.as_str() {
                "and" => Cond::And(cs),
                "or" => Cond::Or(cs),
                _ => Cond::Not(Box::new(Cond::And(cs))),
            });
        }
    }

    // 叶子节点
    let column = obj
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::param("叶子条件缺少 column"))?;
    let opname = obj
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("eq")
        .to_ascii_lowercase();
    let op = Op::from_name(&opname).ok_or_else(|| ApiError::unknown_operator(&opname))?;

    // 支持显式 path（JSON 列）
    let mut expr = column.to_string();
    if let Some(p) = obj.get("path").and_then(|v| v.as_str()) {
        let p = p.trim();
        if !p.is_empty() && p != "$" {
            let p = p.strip_prefix("$.").unwrap_or(p);
            expr = format!("{column}.{p}");
        }
    }

    let value = obj.get("value");
    let vals: Vec<Value> = match op {
        Op::IsNull | Op::IsNotNull => Vec::new(),
        Op::In | Op::Nin | Op::JsonHasAll => match value {
            Some(Value::Array(a)) => a.clone(),
            Some(v) => vec![v.clone()],
            None => Vec::new(),
        },
        Op::Between => match value {
            Some(Value::Array(a)) if a.len() == 2 => a.clone(),
            Some(Value::Array(a)) => a.clone(),
            Some(v) => vec![v.clone()],
            None => Vec::new(),
        },
        _ => match value {
            Some(v) => vec![v.clone()],
            None => Vec::new(),
        },
    };

    resolve(t, reg, &expr, op)?;
    Ok(Cond::Leaf(Leaf::new(expr, op, vals)))
}
