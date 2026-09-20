//! 统一错误类型与响应体（对应接口文档第十三章「错误码手册」）

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::Value;

/// 字段级校验错误，前端可据此高亮表单。
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<Value>,
}

impl FieldError {
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn with_expected(mut self, expected: Vec<String>) -> Self {
        self.expected = Some(expected);
        self
    }

    pub fn with_actual(mut self, actual: Value) -> Self {
        self.actual = Some(actual);
        self
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub code: u32,
    pub status: StatusCode,
    pub message: String,
    pub errors: Vec<FieldError>,
    pub allowed_columns: Option<Vec<String>>,
}

impl ApiError {
    pub fn new(code: u32, status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            status,
            message: message.into(),
            errors: Vec::new(),
            allowed_columns: None,
        }
    }

    pub fn with_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.errors = errors;
        self
    }

    pub fn with_allowed(mut self, cols: Vec<String>) -> Self {
        self.allowed_columns = Some(cols);
        self
    }

    // ---- 40001 字段校验失败 ----
    pub fn validation(field: impl Into<String>, reason: impl Into<String>) -> Self {
        let fe = FieldError::new(field, reason);
        Self::new(40001, StatusCode::BAD_REQUEST, "字段校验失败").with_errors(vec![fe])
    }

    pub fn validation_many(errors: Vec<FieldError>) -> Self {
        Self::new(40001, StatusCode::BAD_REQUEST, "字段校验失败").with_errors(errors)
    }

    // ---- 40002 参数格式错误 ----
    pub fn param(msg: impl Into<String>) -> Self {
        Self::new(40002, StatusCode::BAD_REQUEST, msg)
    }

    // ---- 40003 JSON 列内容非法 ----
    pub fn bad_json(field: impl Into<String>) -> Self {
        let f = field.into();
        Self::new(40003, StatusCode::BAD_REQUEST, "JSON 列内容非法")
            .with_errors(vec![FieldError::new(f, "不是合法的 JSON")])
    }

    // ---- 40004 未知表名 / 列名 / 操作符 ----
    pub fn unknown_table(name: &str, allowed: Vec<String>) -> Self {
        Self::new(40004, StatusCode::BAD_REQUEST, format!("未知表名: {name}")).with_allowed(allowed)
    }

    pub fn unknown_column(table: &str, column: &str, allowed: Vec<String>) -> Self {
        Self::new(
            40004,
            StatusCode::BAD_REQUEST,
            format!("表 {table} 上不存在列 {column}"),
        )
        .with_allowed(allowed)
    }

    pub fn unknown_operator(op: &str) -> Self {
        Self::new(
            40004,
            StatusCode::BAD_REQUEST,
            format!("未知查询操作符: {op}（可用操作符见接口文档 6.3）"),
        )
    }

    pub fn bad_path(msg: impl Into<String>) -> Self {
        Self::new(40004, StatusCode::BAD_REQUEST, msg)
    }

    // ---- 40005 危险操作被拒 ----
    pub fn dangerous(msg: impl Into<String>) -> Self {
        Self::new(40005, StatusCode::BAD_REQUEST, msg)
    }

    // ---- 40006 超出上限 ----
    pub fn over_limit(msg: impl Into<String>) -> Self {
        Self::new(40006, StatusCode::BAD_REQUEST, msg)
    }

    // ---- 40008 外键目标不存在 ----
    pub fn foreign_key(msg: impl Into<String>) -> Self {
        Self::new(40008, StatusCode::BAD_REQUEST, msg)
    }

    // ---- 40401 记录不存在 ----
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(40401, StatusCode::NOT_FOUND, msg)
    }

    // ---- 40901 唯一键冲突 ----
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(40901, StatusCode::CONFLICT, msg)
    }

    // ---- 50001 / 50002 ----
    pub fn db(msg: impl Into<String>) -> Self {
        Self::new(50001, StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(50002, StatusCode::INTERNAL_SERVER_ERROR, msg)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut body = serde_json::json!({
            "code": self.code,
            "message": self.message,
            "data": Value::Null,
        });
        if !self.errors.is_empty() {
            body["errors"] = serde_json::to_value(&self.errors).unwrap_or(Value::Null);
        }
        if let Some(a) = &self.allowed_columns {
            body["allowed_columns"] = serde_json::json!(a);
        }
        (self.status, Json(body)).into_response()
    }
}

/// SQLite 约束错误的整数扩展码（见 sqlite3.h）
const SQLITE_CONSTRAINT_CHECK: i32 = 275;
const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;
const SQLITE_CONSTRAINT_NOTNULL: i32 = 1299;
const SQLITE_CONSTRAINT_PRIMARYKEY: i32 = 1555;
const SQLITE_CONSTRAINT_UNIQUE: i32 = 2067;

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(f, msg) = &e {
            let detail = msg.clone().unwrap_or_default();
            match f.extended_code {
                SQLITE_CONSTRAINT_UNIQUE | SQLITE_CONSTRAINT_PRIMARYKEY => {
                    return ApiError::conflict(if detail.is_empty() {
                        "唯一键冲突：记录已存在".to_string()
                    } else {
                        format!("唯一键冲突：{detail}")
                    });
                }
                SQLITE_CONSTRAINT_NOTNULL => {
                    return ApiError::new(
                        40001,
                        StatusCode::BAD_REQUEST,
                        if detail.is_empty() {
                            "必填字段缺失".to_string()
                        } else {
                            detail
                        },
                    );
                }
                SQLITE_CONSTRAINT_CHECK => {
                    return ApiError::new(
                        42201,
                        StatusCode::UNPROCESSABLE_ENTITY,
                        if detail.is_empty() {
                            "CHECK 约束失败".to_string()
                        } else {
                            detail
                        },
                    );
                }
                SQLITE_CONSTRAINT_FOREIGNKEY => {
                    return ApiError::foreign_key(if detail.is_empty() {
                        "外键目标不存在".to_string()
                    } else {
                        detail
                    });
                }
                _ => {}
            }
            if f.code == rusqlite::ErrorCode::DatabaseBusy {
                return ApiError::new(50301, StatusCode::SERVICE_UNAVAILABLE, "数据库忙，请重试");
            }
        }
        ApiError::db(format!("数据库错误: {e}"))
    }
}
