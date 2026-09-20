//! 表结构元数据（白名单）。
//!
//! 启动时从 `resources/schema.json`（由 tools/build_api_docs.py 从建库 SQL 生成）加载
//! 61 张表、838 个列的定义。**所有动态查询必须在这份白名单内解析**：
//! 表名、列名、排序字段、分组字段、聚合字段全部要命中，否则直接 40004 拒绝。
//! 结构部分（表名/列名）永远映射回这里的常量，值一律走 `?` 占位符。

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reference {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    /// 语义类型：TEXT / INTEGER / REAL / BOOLEAN
    #[serde(rename = "type")]
    pub ty: String,
    pub sql_type: String,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub not_null: bool,
    pub unique: bool,
    /// 枚举取值（TEXT + CHECK IN (...)）
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    pub boolean: bool,
    pub default: Option<String>,
    pub references: Option<Reference>,
    /// 该列存 JSON 字符串（依赖 JSON1 扩展）
    pub json: bool,
}

impl Column {
    pub fn is_numeric(&self) -> bool {
        matches!(self.ty.as_str(), "INTEGER" | "REAL" | "BOOLEAN")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BelongsTo {
    pub table: String,
    pub column: String,
    pub ref_column: String,
    pub on_delete: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HasMany {
    pub table: String,
    pub column: String,
    pub on_delete: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    pub group_code: String,
    pub group: String,
    pub primary_key: Vec<String>,
    pub unique_keys: Vec<Vec<String>>,
    pub foreign_keys: Vec<ForeignKey>,
    pub columns: Vec<Column>,
    pub indexes: Vec<IndexDef>,
    pub belongs_to: Vec<BelongsTo>,
    pub has_many: Vec<HasMany>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// 取列定义，否则返回带 `allowed_columns` 的 40004
    pub fn require_column(&self, name: &str) -> Result<&Column, ApiError> {
        self.column(name).ok_or_else(|| {
            ApiError::unknown_column(
                &self.name,
                name,
                self.columns.iter().map(|c| c.name.clone()).collect(),
            )
        })
    }

    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }

    pub fn is_single_pk(&self) -> bool {
        self.primary_key.len() == 1
    }

    /// 可作为 `expand` 目标的外键表名（去重）
    pub fn expandable(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for b in &self.belongs_to {
            if !v.contains(&b.table) {
                v.push(b.table.clone());
            }
        }
        v
    }

    /// 可作为 `children` 目标的子表名（去重）
    pub fn childable(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for h in &self.has_many {
            if !v.contains(&h.table) {
                v.push(h.table.clone());
            }
        }
        v
    }

    /// 全局模糊搜索 `q` 命中的列（全部 TEXT 列）
    pub fn searchable_columns(&self) -> Vec<String> {
        self.columns
            .iter()
            .filter(|c| c.sql_type == "TEXT" && !c.primary_key)
            .map(|c| c.name.clone())
            .collect()
    }
}

pub struct Registry {
    tables: BTreeMap<String, Table>,
    enums: BTreeMap<String, Value>,
    /// 列名 -> 出现该列的表（用于诊断信息）
    by_column: HashMap<String, Vec<String>>,
}

impl Registry {
    pub fn from_json(s: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct File {
            enums: BTreeMap<String, Value>,
            tables: Vec<Table>,
        }
        let f: File = serde_json::from_str(s).map_err(|e| format!("schema.json 解析失败: {e}"))?;
        let mut tables = BTreeMap::new();
        let mut by_column: HashMap<String, Vec<String>> = HashMap::new();
        for t in f.tables {
            for c in &t.columns {
                by_column
                    .entry(c.name.clone())
                    .or_default()
                    .push(t.name.clone());
            }
            tables.insert(t.name.clone(), t);
        }
        if tables.is_empty() {
            return Err("schema.json 中没有任何表定义".to_string());
        }
        Ok(Self {
            tables,
            enums: f.enums,
            by_column,
        })
    }

    pub fn table(&self, name: &str) -> Result<&Table, ApiError> {
        self.tables.get(name).ok_or_else(|| {
            let mut allowed: Vec<String> = self.tables.keys().cloned().collect();
            allowed.sort();
            ApiError::unknown_table(name, allowed)
        })
    }

    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.tables.values()
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn enums(&self) -> &BTreeMap<String, Value> {
        &self.enums
    }

    pub fn tables_with_column(&self, column: &str) -> Vec<String> {
        self.by_column.get(column).cloned().unwrap_or_default()
    }

    /// 该列是否为某表的主键
    pub fn is_pk(&self, table: &str, column: &str) -> bool {
        self.tables
            .get(table)
            .map(|t| t.primary_key.iter().any(|k| k == column))
            .unwrap_or(false)
    }
}

/// 表名集合的 JSON（/meta/tables 用）
pub fn tables_summary(reg: &Registry) -> Value {
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for t in reg.tables() {
        groups.entry(t.group.clone()).or_default().push(serde_json::json!({
            "name": t.name,
            "path": format!("/api/v1/{}", t.name),
            "primary_key": t.primary_key,
            "unique_keys": t.unique_keys,
            "column_count": t.columns.len(),
        }));
    }
    let total: usize = reg.table_count();
    serde_json::json!({
        "table_count": total,
        "groups": groups,
    })
}
