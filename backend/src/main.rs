//! 独立 HTTP 服务入口（开发/联调/Web 交付/自检用）。
//!
//! ```text
//! cargo run -- --port 8787 --db ./data/crm.db
//! cargo run                    # 自动选端口 + 自动定位数据库（exe 同级或 %APPDATA%）
//! cargo run -- --db-check      # 只做数据库自检（权限/位置/写探针），打印报告后退出
//!
//! # Web 版（形态 A · 单进程内嵌托管）：同一个进程把前端页面一起管了
//! bcrm-backend.exe --web-dir ..\frontend\dist
//! bcrm-backend.exe --web-dir .\web --port 80        # 给本机浏览器用
//! bcrm-backend.exe --web-dir .\web --host 0.0.0.0   # 供局域网访问（⚠️ 当前无鉴权，慎用）
//! ```
//!
//! 不给 `--web-dir` 时按候选位自动探测；探测不到就退化成纯 API 服务
//! （形态 B · 前后端分离：前端交给 Nginx/IIS，这里只出接口）。
//!
//! Tauri 集成时不需要这个 main：直接把 `build_app()` 挂到本地监听即可。

use bcrm_backend::{build_app, build_app_with_web, AppState};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut port: u16 = 8787;
    let mut host: String = "127.0.0.1".to_string();
    let mut db_path: Option<String> = None;
    let mut web_dir: Option<PathBuf> = None;
    let mut no_web = false;
    let mut db_check = false;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.get(i + 1) {
                    port = v.parse().unwrap_or(8787);
                }
                i += 2;
            }
            "--host" => {
                if let Some(v) = args.get(i + 1) {
                    host = v.clone();
                }
                i += 2;
            }
            "--db" | "-d" => {
                db_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--web-dir" | "-w" => {
                web_dir = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--no-web" => {
                no_web = true;
                i += 1;
            }
            "--db-check" | "--db-info" => {
                db_check = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => i += 1,
        }
    }

    if db_check {
        std::process::exit(run_db_check());
    }

    let (state, loc) = if let Some(p) = db_path {
        let st = AppState::init(&p).unwrap_or_else(|e| {
            eprintln!("初始化失败: {e}");
            std::process::exit(1);
        });
        (st, None)
    } else {
        match AppState::open_default() {
            Ok((st, l)) => (st, Some(l)),
            Err(e) => {
                eprintln!("初始化失败: {e}");
                println!("提示：运行 `bcrm-server --db-check` 可以看到每个候选位置的权限诊断。");
                std::process::exit(1);
            }
        }
    };

    let tables = state.reg.table_count();
    let db_display = match &loc {
        Some(l) => l.brief(),
        None => state.db.path().to_string_lossy().to_string(),
    };

    // 没显式给 --web-dir、也没 --no-web 时，按候选位自动探测前端产物：
    // 找到就自动变成 Web 版（形态 A · 单进程内嵌托管），找不到就退化成纯 API 服务。
    if web_dir.is_none() && !no_web {
        web_dir = detect_web_dir();
    }
    if let Some(d) = &web_dir {
        if !d.join("index.html").is_file() {
            eprintln!("--web-dir 指向的目录里没有 index.html：{}", d.display());
            std::process::exit(1);
        }
    }

    let app = match &web_dir {
        Some(d) => build_app_with_web(state, d.clone()),
        None => build_app(state),
    };
    let addr = format!("{host}:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("监听 {addr} 失败: {e}");
            std::process::exit(1);
        }
    };

    println!("客户管理系统后端已启动");
    match &web_dir {
        Some(d) => {
            println!("  模式    : Web 版（接口 + 前端页面，同一进程托管）");
            println!("  界面    : http://{addr}/");
            println!("  静态产物: {}", d.display());
        }
        None => println!("  模式    : 纯 API（未托管前端页面）"),
    }
    println!("  接口    : http://{addr}/api/v1");
    println!("  数据库  : {db_display}");
    println!("  表白名单: {tables} 张");
    println!("  健康检查: http://{addr}/api/v1/meta/health");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("服务异常退出: {e}");
    }
}

/// `--db-check`：数据库权限自检（报告文本由 `bcrm_backend::diagnostics_report()` 生成，
/// 与桌面版共用同一份逻辑）。
///
/// 用途：现场排查「普通用户双击打不开」。报告逐个列出候选位置的目录可写性、
/// 库文件可写性，以及最终真正选中的库、表数、行数和写探针结果。
/// 返回进程退出码（0=可用，1=不可用），方便脚本判断。
fn run_db_check() -> i32 {
    let (report, code) = bcrm_backend::diagnostics_report();
    println!("{report}");

    // 同时落一份文件到 exe 同级（现场排查时用户可以直接把文件发回来）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("bcrm-db-check.txt");
            if std::fs::write(&p, &report).is_ok() {
                println!("报告已写入: {}", p.display());
            }
        }
    }

    code
}

/// 自动探测前端构建产物目录（`--web-dir` 未显式给出时用）。
///
/// 覆盖三种典型摆放：
///  · **交付目录**：把 `dist/` 拷成 exe 同级的 `web/`（推荐，见根目录的一键 bat）
///  · **源码树**：exe 在 `backend/target/release/`，产物在 `frontend/dist/`
///  · **仓库根启动**：当前目录下的 `frontend/dist/`
///
/// 全部找不到就返回 `None` —— 调用方退化成「纯 API 服务」，不报错。
fn detect_web_dir() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("web"));
            cands.push(d.join("dist"));
            // backend/target/release/ → 仓库根 → frontend/dist
            cands.push(d.join("../../../frontend/dist"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join("web"));
        cands.push(cwd.join("frontend/dist"));
        cands.push(cwd.join("../frontend/dist"));
    }
    cands.into_iter().find(|p| p.join("index.html").is_file())
}

fn print_help() {
    println!("bcrm-backend —— BCRM 客户管理系统后端服务");
    println!();
    println!("用法: bcrm-backend [选项]");
    println!();
    println!("  --port <端口>     监听端口（默认 8787）");
    println!("  --host <地址>     监听地址（默认 127.0.0.1）");
    println!("                    ⚠️ 当前 API 无鉴权，改成 0.0.0.0 前请先确认网络边界");
    println!("  --db <文件>       指定数据库文件（默认按 exe 同级 → %LOCALAPPDATA%\\Bcrm 自动定位）");
    println!("  --web-dir <目录>  Web 版：顺带托管该目录的前端产物（需含 index.html）");
    println!("                    不给则自动探测 web/ 、frontend/dist/");
    println!("  --no-web          强制纯 API 模式，不做静态托管");
    println!("  --db-check        只做数据库权限自检并打印报告后退出");
    println!("  -h, --help        显示本帮助");
}
