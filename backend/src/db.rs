//! SQLite 连接与事务。
//!
//! 设计取舍：桌面单用户应用，SQLite 本身串行化写入，因此这里用**单个连接 + Mutex**。
//! 比连接池简单得多，且不存在 `database is locked` 的竞态；后续若真出现并发读瓶颈，
//! 再把 `with()` 换成读连接池即可（接口不变）。
//!
//! 每个连接都必须设置 `PRAGMA foreign_keys = ON`，否则 `ON DELETE CASCADE` 静默失效
//! —— 建库脚本本身在建表阶段是 OFF 的。

use crate::error::ApiError;
use rusqlite::{Connection, Transaction};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 建库 SQL（62 表 + 32 索引 + 组织根节点 1 行初始数据），编译期内嵌进二进制。
///
/// ⚠️ 表数/索引数必须与 `说明/08-…SQLite建库语句.md` 保持一致；
/// 改了 DDL 就跑 `python tools/build_schema_resources.py` 重新生成本文件所依赖的资源，
/// 再重编后端（`include_str!` 是编译期读取）。构建脚本 ① 会做这个校验。
pub const SCHEMA_SQL: &str = include_str!("../resources/schema.sql");
/// 表结构元数据（白名单），编译期内嵌。
pub const SCHEMA_JSON: &str = include_str!("../resources/schema.json");

pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    /// 打开（必要时创建）数据库。首次运行会自动执行建库脚本。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ApiError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ApiError::internal(format!("创建数据目录失败: {e}")))?;
            }
        }

        let conn = Connection::open(&path)
            .map_err(|e| ApiError::internal(format!("打开数据库失败 {}: {e}", path.display())))?;
        apply_pragmas(&conn)?;

        // 判断是否已建库：以 customer 表是否存在为准
        let has_schema: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='customer'",
            [],
            |r| r.get(0),
        )?;
        if has_schema == 0 {
            conn.execute_batch(SCHEMA_SQL)
                .map_err(|e| ApiError::internal(format!("建库失败: {e}")))?;
        }
        // 建库脚本末尾会改 PRAGMA，这里再确认一次
        apply_pragmas(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ApiError> {
        self.conn
            .lock()
            .map_err(|_| ApiError::internal("数据库连接锁已中毒"))
    }

    /// 只读/单语句执行：闭包拿到 `&Connection`。
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T, ApiError>) -> Result<T, ApiError> {
        let guard = self.lock()?;
        f(&guard)
    }

    /// 写事务：闭包拿到 `&Transaction`，返回 Ok 时提交，Err 时回滚。
    pub fn with_tx<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, ApiError>,
    ) -> Result<T, ApiError> {
        let mut guard = self.lock()?;
        let tx = guard
            .transaction()
            .map_err(|e| ApiError::db(format!("开启事务失败: {e}")))?;
        let out = f(&tx)?;
        tx.commit()
            .map_err(|e| ApiError::db(format!("提交事务失败: {e}")))?;
        Ok(out)
    }

    /// 写探针：确认这个连接**真的能写库**。
    ///
    /// 为什么必须单独探一次：`Connection::open` 成功 ≠ 能写。典型漏网场景——
    /// 库文件被继承成了只读属性、目录 ACL 只给了「创建/读取」没给「写入」、
    /// 或者 Windows 受控文件夹访问（Controlled Folder Access）拦了桌面/文档目录，
    /// 这些都要等第一次真正写库时才炸，那时用户已经看到界面了，报错也更难懂。
    ///
    /// 实现：在事务里建表再删表，最后 **ROLLBACK**，不留任何残留对象。
    /// `BEGIN IMMEDIATE` 会立刻申请写锁，只读库/只读目录在这一步就会失败。
    pub fn probe_write(&self) -> Result<(), ApiError> {
        self.with(|conn| {
            conn.execute_batch(
                "BEGIN IMMEDIATE;\
                 CREATE TABLE IF NOT EXISTS \"__bcrm_write_probe\" (id INTEGER PRIMARY KEY);\
                 DROP TABLE IF EXISTS \"__bcrm_write_probe\";\
                 ROLLBACK;",
            )
            .map_err(|e| {
                ApiError::internal(format!(
                    "数据库不可写（{}）：{e}",
                    self.path.display()
                ))
            })?;
            Ok(())
        })
    }

    /// 写探针（不返回明细，只报能否写）——自检报告用
    pub fn writable(&self) -> bool {
        self.probe_write().is_ok()
    }

    /// 表行数（/meta/health 用）
    pub fn table_row_counts(&self, limit: usize) -> Result<Vec<(String, i64)>, ApiError> {
        self.with(|conn| {
            let mut names: Vec<String> = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )?;
                let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
                for r in rows {
                    names.push(r?);
                }
            }
            let mut out = Vec::new();
            for n in names.into_iter().take(limit) {
                // 表名来自 sqlite_master，非用户输入
                let c: i64 = conn.query_row(&format!("SELECT count(*) FROM \"{n}\""), [], |r| r.get(0))?;
                out.push((n, c));
            }
            Ok(out)
        })
    }
}

fn apply_pragmas(conn: &Connection) -> Result<(), ApiError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", -20000)?;
    // journal_mode / wal_checkpoint 会返回结果行，必须用 query_row 消费掉
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    let _cp: (i64, i64, i64) =
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 数据库位置解析
// ---------------------------------------------------------------------------
//
// 设计目标：**普通用户双击就一定能打开**，同时保留「便携」能力。
//
// 会踩到的权限坑（都是真实场景，不是假想）：
//   a. 安装包以管理员身份装到 `C:\Program Files\...` → 普通用户双击，目录不可写，
//      建库失败，进程在出窗口前就退出（用户看到的是「双击没反应」）。
//   b. 同 a，但库里**已经有数据**（管理员导入过）。若只做回退，用户会看到一个空库，
//      以为数据丢了 —— 所以回退时要先把已存在的库**拷一份**过去当初始数据。
//   c. `crm.db` 文件本身被继承成只读属性 / 从只读介质（光盘、只读共享）拷来，
//      目录可写但文件不可写 → SQLite 的 `journal_mode=WAL` 或首次写库才报错。
//   d. exe 放在桌面/文档且开了 **受控文件夹访问**（Windows Defender 勒索防护），
//      或放在 OneDrive 同步目录 / 网络盘 → 写 wal 文件被拦或产生同步冲突。
//   e. `%LOCALAPPDATA%` 因漫游配置异常也写不了 → 需要最后一级兜底。
//
// 因此这里的策略不是「一次判定」，而是**候选位逐个实测、失败即下移**：
//   目录可写 → 已有库可写 → 真正执行一次写探针（BEGIN IMMEDIATE + 建表删表 + ROLLBACK）。
// 三段全过才采用，否则记录原因换下一个候选位。

/// 候选位属于哪一类。分类不只是为了打印好看：
/// 「从便携位搬初始数据」只对**用户数据位**做，兜底的当前工作目录不搬 ——
/// 否则会在用户当前目录里凭空多出一个 crm.db。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationKind {
    /// exe 同级目录（便携模式）
    Portable,
    /// `%LOCALAPPDATA%\Bcrm`（安装到受保护目录时的落点）
    UserData,
    /// 当前工作目录（最后兜底）
    WorkingDir,
}

/// 一个候选数据库位置。
#[derive(Debug, Clone)]
pub struct DbCandidate {
    pub path: PathBuf,
    pub kind: LocationKind,
    /// 人话说明这个位置是哪来的，写进日志便于排查
    pub label: String,
}

impl DbCandidate {
    pub fn is_portable(&self) -> bool {
        self.kind == LocationKind::Portable
    }
}

/// 位置判定结果（含跳过记录，供启动日志与自检报告使用）。
#[derive(Debug, Clone)]
pub struct DbLocation {
    pub path: PathBuf,
    pub portable: bool,
    pub label: String,
    /// 被跳过的候选位及原因
    pub skipped: Vec<String>,
}

impl DbLocation {
    /// 单行摘要：`D:\app\crm.db（便携模式）`
    pub fn brief(&self) -> String {
        format!(
            "{}（{}）",
            self.path.display(),
            if self.portable { "便携模式" } else { "用户目录" }
        )
    }
}

/// 用户数据目录基底：优先 `%LOCALAPPDATA%`，退化 `%APPDATA%`。
fn user_data_base() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// 按优先级列出候选数据库位置（同路径去重，避免报告里出现重复项）。
pub fn db_candidates() -> Vec<DbCandidate> {
    let mut out: Vec<DbCandidate> = Vec::new();

    // ① 便携位：exe 同级。数据库跟着程序走，整目录拷到 U 盘/别的机器直接能用。
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(DbCandidate {
                path: dir.join("crm.db"),
                kind: LocationKind::Portable,
                label: format!("exe 同级目录（便携）: {}", dir.display()),
            });
        }
    }

    // ② 用户数据位：安装到受保护目录时的落点，也是普通用户一定有权限的地方。
    if let Some(base) = user_data_base() {
        out.push(DbCandidate {
            path: base.join("Bcrm").join("crm.db"),
            kind: LocationKind::UserData,
            label: format!("用户数据目录: {}", base.join("Bcrm").display()),
        });
    }

    // ③ 最后兜底：当前工作目录（前两个都取不到时的保命选项）
    out.push(DbCandidate {
        path: PathBuf::from("./crm.db"),
        kind: LocationKind::WorkingDir,
        label: "当前工作目录（最后兜底）".to_string(),
    });

    // 去重：exe 装在 %LOCALAPPDATA%\Bcrm 里、或把程序放在当前目录运行时，
    // ①②③ 可能指向同一个文件。保留先出现的那个（便携语义优先）。
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|c| {
        let key = path_key(&c.path);
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });

    out
}

/// 路径归一化用的 key：文件还不存在时退而求其次归一下父目录，
/// 否则 `./crm.db`（相对）和 `D:\app\crm.db`（绝对）明明是同一个文件却去不掉重。
fn path_key(p: &std::path::Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    if let (Some(dir), Some(name)) = (p.parent(), p.file_name()) {
        let dir = if dir.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            dir
        };
        if let Ok(cd) = dir.canonicalize() {
            return cd.join(name);
        }
    }
    p.to_path_buf()
}

/// 兼容旧签名：**纯预测**首选位置，不打开也不建库（无副作用）。
///
/// 新代码请用 [`open_with_fallback`] —— 那个函数会真的执行写探针，
/// 才是「普通用户双击必然能打开」的保证。
pub fn resolve_db_path() -> (PathBuf, bool) {
    let cands = db_candidates();
    if let Some(c) = cands.first() {
        let ok = if c.path.is_file() {
            file_writable(&c.path)
        } else {
            dir_writable(&parent_dir(&c.path))
        };
        if ok {
            return (c.path.clone(), true);
        }
    }
    match cands.iter().find(|c| !c.is_portable()) {
        Some(c) => (c.path.clone(), false),
        None => (PathBuf::from("./crm.db"), false),
    }
}

/// 单个候选位的「只体检、不落库」结果（自检报告 / 启动日志用）。
#[derive(Debug, Clone)]
pub struct CandidateProbe {
    pub label: String,
    pub path: PathBuf,
    pub portable: bool,
    /// 存放目录
    pub dir: PathBuf,
    /// 目录里是否已存在 crm.db
    pub file_exists: bool,
    /// 目录可写（能建临时文件）
    pub dir_writable: bool,
    /// 已有 crm.db 时可写；不存在时为 None
    pub file_writable: Option<bool>,
    /// 这个候选位能不能用
    pub ok: bool,
    pub reason: String,
}

impl CandidateProbe {
    /// 单行摘要，直接进日志
    pub fn describe(&self) -> String {
        format!(
            "[{}] {} | 目录可写={} | 库已存在={} | 库可写={} | {} | {}",
            if self.ok { "可用" } else { "跳过" },
            self.label,
            if self.dir_writable { "是" } else { "否" },
            if self.file_exists { "是" } else { "否" },
            match self.file_writable {
                Some(true) => "是",
                Some(false) => "否",
                None => "—",
            },
            self.reason,
            self.path.display()
        )
    }
}

/// 体检一个候选位（**只读检查**，除了必要时补建存放目录，不会建库/写库）。
pub fn inspect_candidate(cand: &DbCandidate) -> CandidateProbe {
    let dir = parent_dir(&cand.path);
    let file_exists = cand.path.is_file();
    // ⚠️ 顺序很重要：先确保目录存在再探写，否则「目录还不存在」会被误判成「不可写」
    let dir_w = dir_writable(&dir);
    let file_w = if file_exists {
        Some(file_writable(&cand.path))
    } else {
        None
    };

    let (ok, reason) = if file_exists {
        match file_w {
            Some(true) => (true, "已存在的 crm.db 可读写"),
            _ => (false, "已存在的 crm.db 不可写（只读属性或 ACL 拒绝写）"),
        }
    } else if dir_w {
        (true, "目录可写，可直接新建库")
    } else {
        (false, "目录不可写（权限不足 / 只读 / 受控文件夹访问拦截）")
    };

    CandidateProbe {
        label: cand.label.clone(),
        path: cand.path.clone(),
        portable: cand.is_portable(),
        dir,
        file_exists,
        dir_writable: dir_w,
        file_writable: file_w,
        ok,
        reason: reason.to_string(),
    }
}

/// 逐个候选位实测，返回第一个**真的能读写**的数据库连接。
///
/// 这是「普通用户双击必须能打开」的落地实现：任何一步失败都不是直接 fatal，
/// 而是记下原因、换下一个候选位。
pub fn open_with_fallback() -> Result<(Db, DbLocation), ApiError> {
    let cands = db_candidates();
    let mut skipped: Vec<String> = Vec::new();

    // 便携位若已经存在库（管理员装机器时导入过数据），在回退到用户目录时把它拷过去，
    // 否则普通用户双击会看到一个空库、误以为数据丢了。
    let seed_src: Option<PathBuf> = cands
        .iter()
        .map(|c| c.path.clone())
        .find(|p| p.is_file() && file_readable(p));

    let mut last_err: Option<String> = None;

    for cand in &cands {
        let probe = inspect_candidate(cand);
        if !probe.ok {
            skipped.push(format!("{} —— {}", cand.label, probe.reason));
            last_err = Some(format!("{}: {}", cand.path.display(), probe.reason));
            continue;
        }

        // 回退到**用户数据位**且目标库还不存在 → 用便携位的库做初始数据。
        // （只对用户数据位做：兜底的当前工作目录不搬，免得在用户目录里凭空多出个 crm.db）
        if cand.kind == LocationKind::UserData && !probe.file_exists {
            if let Some(src) = &seed_src {
                if src != &cand.path {
                    match seed_copy(src, &cand.path) {
                        Ok(()) => skipped.push(format!(
                            "{} —— 已从 {} 拷贝初始数据库",
                            cand.label,
                            src.display()
                        )),
                        Err(e) => skipped.push(format!(
                            "{} —— 拷贝初始数据库失败（忽略，按空库继续）: {e}",
                            cand.label
                        )),
                    }
                }
            }
        }

        // 落点三连：打开 → 建表检查 → 写探针。全过才算数。
        match Db::open(&cand.path).and_then(|db| {
            db.probe_write()?;
            Ok(db)
        }) {
            Ok(db) => {
                return Ok((
                    db,
                    DbLocation {
                        path: cand.path.clone(),
                        portable: cand.is_portable(),
                        label: cand.label.clone(),
                        skipped,
                    },
                ));
            }
            Err(e) => {
                skipped.push(format!("{} —— 打开/写探针失败: {e}", cand.label));
                last_err = Some(e.to_string());
            }
        }
    }

    Err(ApiError::internal(format!(
        "所有候选数据库位置都不可用（共 {} 个）：\n{}\n最后错误：{}",
        cands.len(),
        skipped.join("\n"),
        last_err.unwrap_or_else(|| "未知".to_string())
    )))
}

fn parent_dir(p: &std::path::Path) -> PathBuf {
    match p.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// 试探目录是否可写：先确保目录存在，再尝试创建一个临时文件并删除。
/// 任何一步失败（权限不足、只读、受控文件夹访问拦截、路径非法）都视为不可写。
fn dir_writable(dir: &std::path::Path) -> bool {
    let _ = std::fs::create_dir_all(dir);
    let probe = dir.join(format!(".bcrm_writetest_{}.tmp", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 已存在的文件能否以**读写**方式打开。只读属性或 ACL 拒绝写会返回 false。
fn file_writable(p: &std::path::Path) -> bool {
    std::fs::OpenOptions::new().write(true).open(p).is_ok()
}

/// 已存在的文件能否读取（决定它能不能当初始数据源）。
fn file_readable(p: &std::path::Path) -> bool {
    std::fs::File::open(p).is_ok()
}

/// 把便携位的库拷到用户目录当初始数据。
///
/// 只拷 `crm.db` 和 `crm.db-wal`：WAL 里可能还压着没 checkpoint 的最新事务，
/// 漏了它就是「最新几条记录不见了」。`-shm` 是纯临时索引，SQLite 会自己重建，
/// 拷过去反而可能带来不一致，所以故意不拷。
///
/// ⚠️ **必须清掉只读属性**：Windows 的 `fs::copy` 会把源文件的**属性**一起复制，
/// 源库是只读的（从只读介质拷来 / 被继承成只读），副本也是只读的 ——
/// 那"回退到用户目录"就白做了，照样是 `attempt to write a readonly database`。
/// 这个坑很隐蔽：目录可写、库文件看上去也在，只有真去写才炸。
fn seed_copy(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    let dir = parent_dir(dst);
    std::fs::create_dir_all(&dir)?;
    std::fs::copy(src, dst)?;
    clear_readonly(dst);

    let wal_src = append_suffix(src, "-wal");
    if wal_src.is_file() {
        // wal 拷不过去不算致命：库主体已经在，最多丢最后一次 checkpoint 之后的事务
        let wal_dst = append_suffix(dst, "-wal");
        if std::fs::copy(&wal_src, &wal_dst).is_ok() {
            clear_readonly(&wal_dst);
        }
    }
    Ok(())
}

/// 去掉文件的只读属性（Windows 上就是清掉 FILE_ATTRIBUTE_READONLY）。
/// 文件不存在或系统拒绝改属性时静默忽略 —— 后续的写探针会把真正的问题报出来。
fn clear_readonly(p: &std::path::Path) {
    if let Ok(md) = std::fs::metadata(p) {
        let mut perm = md.permissions();
        if perm.readonly() {
            perm.set_readonly(false);
            let _ = std::fs::set_permissions(p, perm);
        }
    }
}

fn append_suffix(p: &std::path::Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}
