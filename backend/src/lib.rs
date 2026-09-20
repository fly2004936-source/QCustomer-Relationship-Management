//! bcrm-backend —— 客户管理系统后端服务库。
//!
//! 对外暴露 `AppState` 与 `build_app()`，既可被独立 HTTP 服务（`main.rs`）使用，
//! 也可被 Tauri 壳直接依赖（把 `build_app()` 挂在本地 `127.0.0.1` 随机端口上）。

pub mod crud;
pub mod db;
pub mod error;
pub mod query;
pub mod routes;
pub mod schema;
pub mod value;

use std::sync::Arc;

/// 全局状态：数据库 + 表结构白名单
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Db>,
    pub reg: Arc<schema::Registry>,
}

impl AppState {
    /// 打开数据库并加载白名单
    pub fn init(db_path: impl AsRef<std::path::Path>) -> Result<Self, error::ApiError> {
        let db = db::Db::open(db_path)?;
        let reg = schema::Registry::from_json(db::SCHEMA_JSON)
            .map_err(|e| error::ApiError::internal(format!("白名单加载失败: {e}")))?;
        Ok(Self {
            db: Arc::new(db),
            reg: Arc::new(reg),
        })
    }

    /// 按应用规则自动定位数据库文件，并**保证返回的连接真的可写**。
    ///
    /// 与 [`AppState::init_default`] 的区别：这里是「候选位逐个实测、失败即下移」，
    /// 任何权限问题都不会让调用方拿到一个不可用的库（普通用户双击也能打开）。
    pub fn open_default() -> Result<(Self, db::DbLocation), error::ApiError> {
        let (db, loc) = db::open_with_fallback()?;
        let reg = schema::Registry::from_json(db::SCHEMA_JSON)
            .map_err(|e| error::ApiError::internal(format!("白名单加载失败: {e}")))?;
        Ok((
            Self {
                db: Arc::new(db),
                reg: Arc::new(reg),
            },
            loc,
        ))
    }

    /// 按应用规则自动定位数据库文件（兼容旧签名，只回传是否便携模式）
    pub fn init_default() -> Result<(Self, bool), error::ApiError> {
        let (st, loc) = Self::open_default()?;
        Ok((st, loc.portable))
    }
}

/// 构建完整路由（纯 API）
pub fn build_app(state: AppState) -> axum::Router {
    routes::router(state)
}

/// 构建「API + 前端静态产物」路由 —— Web 版（形态 A · 单进程内嵌托管）。
///
/// `web_dir` 指向前端构建产物目录（含 `index.html` + `assets/`）。
/// 不需要这个能力时继续用 [`build_app`]：桌面版（Tauri 壳）自带 WebView，
/// 前端由壳内嵌，不经过 HTTP 静态服务。
pub fn build_app_with_web(
    state: AppState,
    web_dir: impl Into<std::path::PathBuf>,
) -> axum::Router {
    routes::router_with_web(state, web_dir)
}

/// 数据库自检报告（`--db-check` / 桌面端启动排障用）。
///
/// 返回 `(报告文本, 退出码)`，退出码 0 = 数据库可读写、应用能正常打开。
///
/// 为什么要做成一个函数：桌面版（GUI 子系统，没有控制台）与控制台版
/// （`bcrm-server`）都要输出同一份诊断，逻辑放在库里只有一份，不会漂移。
pub fn diagnostics_report() -> (String, i32) {
    let mut out = String::new();

    {
        let mut w = |s: String| {
            out.push_str(&s);
            out.push('\n');
        };

        w("BCRM 数据库自检报告".to_string());
        w("==================================================".to_string());
        w(format!(
            "用户      : {}\\{}",
            std::env::var("USERDOMAIN").unwrap_or_else(|_| "?".into()),
            std::env::var("USERNAME").unwrap_or_else(|_| "?".into())
        ));
        w(format!(
            "进程位数  : {}",
            if cfg!(target_pointer_width = "64") {
                "64 位"
            } else {
                "32 位"
            }
        ));
        w(format!(
            "可执行文件: {}",
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(取不到)".into())
        ));
        w(format!(
            "数据目录基: {}",
            std::env::var("LOCALAPPDATA")
                .or_else(|_| std::env::var("APPDATA"))
                .unwrap_or_else(|_| "(取不到)".into())
        ));
        w(String::new());

        w("候选位置体检（按优先级，逐个实测）".to_string());
        w("--------------------------------------------------".to_string());
        for (idx, c) in db::db_candidates().iter().enumerate() {
            w(format!("{}. {}", idx + 1, db::inspect_candidate(c).describe()));
        }
        w(String::new());

        w("实际落点（打开 + 建表检查 + 写探针）".to_string());
        w("--------------------------------------------------".to_string());
    }

    let code = match AppState::open_default() {
        Ok((st, loc)) => {
            let mut extra = String::new();
            for s in &loc.skipped {
                extra.push_str(&format!("  · {s}\n"));
            }
            extra.push_str(&format!("  选用    : {}\n", loc.brief()));
            extra.push_str(&format!("  表白名单: {} 张\n", st.reg.table_count()));
            let counts = st.db.table_row_counts(200).unwrap_or_default();
            let total: i64 = counts.iter().map(|(_, c)| *c).sum();
            extra.push_str(&format!("  数据行数: {total} 行（{} 张表）\n", counts.len()));
            extra.push_str(&format!(
                "  写探针  : {}\n",
                if st.db.writable() { "通过" } else { "失败" }
            ));
            if let Ok(md) = std::fs::metadata(loc.path.as_path()) {
                extra.push_str(&format!("  库文件  : {} 字节\n", md.len()));
            }
            extra.push_str("\n结论: 可用（数据库可读可写，应用能正常打开）");
            out.push_str(&extra);
            0
        }
        Err(e) => {
            out.push_str(&format!("  失败: {e}\n\n"));
            out.push_str("结论: 不可用（请管理员检查目录 ACL，或把程序放到有写权限的目录）");
            1
        }
    };

    (out, code)
}
