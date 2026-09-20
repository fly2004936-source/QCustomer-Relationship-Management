//! BCRM 桌面版启动器（Tauri 壳 + 内嵌 axum 后端）。
//!
//! # 架构
//!
//! ```text
//!   bcrm-desktop.exe
//!   ├── axum 服务       监听 127.0.0.1:<系统分配的随机空闲端口>
//!   ├── SQLite 数据库   优先 exe 同级 crm.db，不可写则回退 %LOCALAPPDATA%\Bcrm\crm.db
//!   └── Tauri WebView   加载编进二进制的前端产物，页面脚本执行前注入 API 地址
//! ```
//!
//! 为什么不另起一个后端 exe（sidecar）：那样交付物就是两个文件，
//! 还得处理"先起后端再起界面"的时序和残留进程。直接把 `bcrm_backend::build_app()`
//! 挂进同一个进程，交付物就是一个 exe，关掉窗口进程就整体退出。
//!
//! # 端口为什么不能写死
//!
//! 8787 是开发期用的固定端口，但打包后用户机器上它完全可能被别的程序占着。
//! 所以这里 `bind("127.0.0.1:0")` 让系统分配一个空闲端口，
//! 再通过 `initialization_script` 把真实地址注入成 `window.__BCRM_API_BASE__`，
//! 前端 `api/client.js` 优先读它。这样打包后不用改一行前端代码。
//!
//! # 前端与 API 仍然跨源
//!
//! 界面跑在 `tauri://localhost`，API 在 `http://127.0.0.1:<port>`，两者不同源。
//! 后端 `routes.rs` 的 CORS 层已经放行了 `tauri://localhost` / `http://tauri.localhost`，
//! 所以这一层是打包后必需的、不是开发期临时措施。
//!
//! # 「双击没反应」的三大根因，都在出窗口之前
//!
//! 交付给非技术用户后，最常见的故障是**双击了完全没动静**。它们都发生在
//! 窗口创建之前，release 版又没有控制台，所以这里逐条做了前置处理：
//!
//! 1. **WebView2 运行时缺失**：Tauri 在 Windows 上用 Edge WebView2 渲染界面。
//!    Win10 2018 年 4 月版之前、或做过精简的镜像里可能没有这个运行时，
//!    此时窗口根本建不出来。安装包（NSIS）会自带在线引导器自动装，
//!    但**便携版 exe 不会** —— 所以这里在启动前主动检测，缺了就弹出中文提示
//!    并给出一键打开微软官方在线安装页，而不是让用户面对一个"闪一下"的窗口。
//! 2. **数据库目录不可写**：典型是安装到 `C:\Program Files` 后普通用户双击。
//!    位置解析放在 `bcrm_backend::db::open_with_fallback()`，会逐个候选位实测
//!    （目录可写 → 已有库可写 → 写探针）并自动回退，失败时把每个位置的
//!    失败原因一起弹出来。
//! 3. **WebView2 用户数据目录被提权进程创建过**：见下面
//!    [`heal_webview_data_dir`] 的注释。这个坑的特点是**进程能起来、日志正常
//!    打到「窗口已创建」之前，但窗口永远不出现**，而且一旦踩上就会一直复现
//!    （因为每次启动都会去碰同一个目录）。本机实测过，所以改成启动时自愈。
//!
//! ⚠️ 还有一类「双击完全没反应、连一行日志都没有」的原因，**程序自身处理不了**：
//! **终端安全软件拦截**。典型是企业统一部署的 EDR/管控客户端（奇安信天擎、
//! 深信服、CrowdStrike 等）用云端策略判定「未签名 = 陌生软件」并在进程创建
//! 之前就拦掉。此时日志一个字都不会有（进程根本没被创建），程序内部做任何
//! 兜底都无济于事。本机实测被奇安信天擎以 `CloudRule.Block.Strangesoftware`
//! 拦过，排查方法写在交付的 `使用说明.txt` 里（查天擎自己日志里的
//! `fcEng/defender/xdlog/`、把程序加入信任区、或临时用管理员身份运行）。
//!
//! 排查用：`bcrm-desktop.exe --db-check` 会在 exe 同级生成 `bcrm-db-check.txt`。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bcrm_backend::{build_app, AppState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// WebView2 用户数据目录的兜底标识。
///
/// ⚠️ 必须与 `tauri.conf.json` 的 `identifier` 一致：Tauri 默认把 WebView2 的用户数据
/// 目录放在 `%LOCALAPPDATA%\` + identifier 下，出故障时要靠它定位。
/// 运行时以 `app.config().identifier` 为准（见 setup），这里只在取不到配置时兜底。
const APP_IDENTIFIER_FALLBACK: &str = "com.bcrm.desktop";

/// 实际使用的 WebView2 用户数据目录，setup 里按配置算出来存这里，
/// 供窗口创建失败后的诊断/修复使用（此时已拿不到 app 句柄）。
static WEBVIEW_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// WebView2 Evergreen 在线引导器（微软官方短链，约 2MB，装的是最新稳定版）。
/// 安装包在检测到运行时缺失时也会去下同一个地址。
const WV2_BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
/// WebView2 官方主页：需要离线安装包 / 分架构下载时给用户指到这里。
const WV2_HOME_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

/// 程序所在目录。日志、错误文件、自检报告都落在这里，方便用户直接发回来。
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 启动日志写在 exe 同级，方便"双击没反应"时排查（release 版没有控制台）
fn log(msg: &str) {
    let path = exe_dir().join("bcrm-launcher.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

fn fatal(msg: &str) -> ! {
    log(&format!("[FATAL] {msg}"));
    let detail = format!(
        "BCRM 启动失败：\n\n{msg}\n\n排查线索：程序所在目录下的\n\
         · bcrm-launcher.log（启动过程）\n\
         · bcrm-launcher-error.log（本次错误）\n\
         · bcrm-db-check.txt（运行 bcrm-desktop.exe --db-check 生成）"
    );
    // ⚠️ 这里**不能**写相对路径：桌面上双击快捷方式时，工作目录是快捷方式的
    // "起始位置"，可能是个只读目录，写不进去就等于把唯一的错误线索丢了。
    let dir = exe_dir();
    let mut wrote = std::fs::write(dir.join("bcrm-launcher-error.log"), &detail).is_ok();
    if !wrote {
        // exe 目录也写不了（正好就是出错原因）：退回临时目录，保证线索不丢
        wrote = std::fs::write(std::env::temp_dir().join("bcrm-launcher-error.log"), &detail).is_ok();
    }
    log(&format!(
        "错误详情已写入: {}",
        if wrote { "exe 同级或临时目录" } else { "失败" }
    ));
    // release 版没有控制台，错误会被静默吞掉；用系统对话框弹出来，避免「双击没反应」无从排查
    show_error_box(&detail);
    std::process::exit(1);
}

/// 用 Windows 系统对话框显示一条信息。
///
/// 为什么借道 PowerShell：`MessageBoxW` 需要 winapi/windows-sys 依赖，
/// 而这里只为了弹个框就把整个依赖链加进来不划算。系统自带的 PowerShell
/// 在所有受支持的 Windows 上都有。
fn message_box(msg: &str, title: &str, buttons: &str, icon: &str) -> bool {
    let tmp = std::env::temp_dir().join("bcrm_dialog_msg.txt");
    if std::fs::write(&tmp, msg).is_err() {
        return false;
    }
    // 只读文件内容，避免消息里的引号/换行破坏脚本
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $m=[IO.File]::ReadAllText('{p}'); \
         $r=[System.Windows.Forms.MessageBox]::Show($m,'{t}', \
            [System.Windows.Forms.MessageBoxButtons]::{b}, \
            [System.Windows.Forms.MessageBoxIcon]::{i}); \
         if($r -eq 'Yes'){{exit 0}}else{{exit 1}}",
        p = tmp.display(),
        t = title,
        b = buttons,
        i = icon
    );
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn show_error_box(msg: &str) {
    let _ = message_box(msg, "BCRM 启动失败", "OK", "Error");
}

/// 确认框：用户点了「是」返回 true（按钮文案由 Windows 按系统语言本地化）。
fn ask_yes_no(msg: &str, title: &str) -> bool {
    message_box(msg, title, "YesNo", "Warning")
}

/// 用系统默认浏览器打开链接。
fn open_url(url: &str) {
    log(&format!("打开链接: {url}"));
    // `start` 是 cmd 内建命令，第一个引号参数会被当成窗口标题，所以补一个空标题占位。
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

// ---------------------------------------------------------------------------
// WebView2 用户数据目录：本机实测过的「双击没反应」第二根因
// ---------------------------------------------------------------------------
//
// Tauri 在 Windows 上把 WebView2 的用户数据目录放在 `%LOCALAPPDATA%\<identifier>`。
// 这个目录**一旦被提权（管理员）进程创建过**，之后普通用户双击时 WebView2 无法接管它，
// 表现就是「进程起来了、没有任何窗口」——和缺 WebView2 运行时长得一模一样，
// 但日志里 WebView2 版本那一行是正常打出来的。
//
// 什么时候会被提权进程创建：
//   · 右键「以管理员身份运行」过，
//   · 安装包结尾勾了「运行 BCRM客户管理系统」那次（安装程序是提权身份），
//   · 开发期用提权的 shell 反复跑过。
//
// 目录本身只是浏览器缓存（Cookie/GPU 缓存等），删掉会自己重建，**不影响 crm.db**。
// 所以这里在窗口创建失败时把这三件事一起告诉用户：是谁建的、为什么不能用、怎么修。

/// WebView2 用户数据目录 = `%LOCALAPPDATA%\<identifier>`
///
/// ⚠️ 这只是**兜底**用的默认值（Tauri 自己会算这个路径）。
/// 正常路径由 [`webview_data_dir_candidates`] 决定，并且会显式传给
/// `WebviewWindowBuilder::data_directory()` —— 显式指定有两个好处：
/// ① 不再依赖 `identifier` 推断，路径可预期；② 可以挑一个当前用户确实能写的
/// 位置，和 `crm.db` 用同一套「候选位 + 可写性实测」思路。
fn default_webview_data_dir(identifier: &str) -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(identifier)
}

/// WebView2 用户数据目录的候选位（按优先级）。
///
/// 与 `crm.db` 的候选位保持一致的取向：**便携优先，其次用户数据目录**。
/// 返回 `(路径, 是否便携, 说明)`。
fn webview_data_dir_candidates(identifier: &str) -> Vec<(PathBuf, bool, String)> {
    let mut out: Vec<(PathBuf, bool, String)> = Vec::new();

    // ① 便携位：exe 同级的隐藏目录。整目录拷走仍能用，且不碰用户系统目录。
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            out.push((
                d.join(".bcrm-webview"),
                true,
                format!("exe 同级（便携）: {}", d.join(".bcrm-webview").display()),
            ));
        }
    }

    // ② 用户数据位：和 crm.db 放在同一个 Bcrm 目录下，便于一并备份/清理。
    if let Some(base) = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .ok()
    {
        let p = PathBuf::from(&base).join("Bcrm").join("WebView2");
        out.push((p.clone(), false, format!("用户数据目录: {}", p.display())));
    }

    // ③ Tauri 默认位：前两个都不可写时的保命选项
    let d = default_webview_data_dir(identifier);
    out.push((d.clone(), false, format!("Tauri 默认位: {}", d.display())));

    out
}

/// 体检并自愈 WebView2 用户数据目录，返回一行给日志的说明。
///
/// # 为什么必须自愈，而不是"出错了再问用户"
///
/// WebView2 的用户数据目录**一旦被提权（管理员）进程创建过**，之后普通用户
/// 双击时 WebView2 会拒绝接管它。表现是「进程起来了、日志一路正常，但窗口
/// 永远不出现」—— 和缺 WebView2 运行时长得一模一样，极难现场判断。
///
/// 更麻烦的是**它会一直复现**：每次启动都读同一个目录，踩一次就永久生效。
/// 触发场景都很日常：
///   · 安装包结尾勾了「运行 BCRM客户管理系统」—— 安装程序是提权身份，
///     它拉起来的子进程继承管理员令牌（这是最常见的一条）；
///   · 用户自己右键「以管理员身份运行」过一次；
///   · 开发/排查时用提权的 shell 跑过（本项目就是被自己这样坑到的）。
///
/// 而 `offer_webview_repair()` 那条"弹框问用户要不要删"的路子在这里**不够用**：
/// 用户遇到的是「双击没反应」，连窗口都没有，更不会有耐心去点确认框。
/// 所以启动时（建窗口之前）直接处理掉。
///
/// # 为什么删它是安全的
///
/// 这个目录里只有浏览器缓存（Cookie / GPU 缓存 / Local Storage / Crashpad 报告），
/// **业务数据全在 crm.db**，两者没有任何关系。删掉后 WebView2 会自己重建。
///
/// # 关键实现细节：怎么判断"当前进程是不是提权"
///
/// 不引 winapi/windows-sys：**建一个探针文件，读它的所有者**。
/// 提权进程创建的文件所有者是 `BUILTIN\Administrators`，普通进程创建的是
/// 当前用户名。拿这个当身份参照，和目录所有者比对即可 —— 只有"目录属于
/// Administrators 而我显然不是"这一种组合才需要清理。
fn heal_webview_data_dir(dir: &Path) -> String {
    if !dir.exists() {
        return format!("WebView2 用户数据目录: {}（不存在，首次启动会创建）", dir.display());
    }

    let dir_owner_name = match path_owner(dir) {
        Some(o) => o,
        None => {
            return format!(
                "WebView2 用户数据目录: {}（所有者读不到，跳过体检）",
                dir.display()
            )
        }
    };

    if !owner_is_privileged(&dir_owner_name) {
        return format!(
            "WebView2 用户数据目录: {}（所有者 {dir_owner_name}，正常）",
            dir.display()
        );
    }

    // 目录所有者是管理员/系统 —— 那"我"是谁？若我也是管理员身份，
    // 说明是自己人建的，不会有接管问题；只有"提权的建、非提权的用"才是坑。
    let my_owner = current_token_owner().unwrap_or_default();
    if owner_is_privileged(&my_owner) {
        return format!(
            "WebView2 用户数据目录: {}（所有者 {dir_owner_name}，当前进程同为管理员身份，可用）",
            dir.display()
        );
    }

    // 到这里就是确凿的坑：目录被提权进程创建，而当前是普通用户身份。
    log(&format!(
        "[WARN] WebView2 用户数据目录属于 {dir_owner_name}，但当前进程是普通用户（{my_owner}）—— \
         这正是「双击没反应、窗口不出现」的已知根因。尝试删除后重建。"
    ));
    match std::fs::remove_dir_all(dir) {
        Ok(()) => format!(
            "WebView2 用户数据目录: {} —— 已自动修复（原属 {dir_owner_name}，已删除，稍后重建）",
            dir.display()
        ),
        Err(e) => format!(
            "WebView2 用户数据目录: {} —— ⚠️ 属 {dir_owner_name} 且删除失败（{e}）。\
             请关闭所有 BCRM 窗口后手动删除该目录再启动。",
            dir.display()
        ),
    }
}

/// 取「当前进程令牌」对应的所有者，用作身份参照（见 [`heal_webview_data_dir`]）。
///
/// 在自己的临时目录里建一个探针文件读它的 Owner：提权进程创建的文件所有者是
/// `BUILTIN\Administrators`，普通进程创建的是当前用户名。这样不引 winapi 依赖
/// 就能判断"我是不是提权跑着"。
fn current_token_owner() -> Option<String> {
    let probe = std::env::temp_dir().join(format!(".bcrm_owner_probe_{}", std::process::id()));
    std::fs::write(&probe, b"x").ok()?;
    let owner = path_owner(&probe);
    let _ = std::fs::remove_file(&probe);
    owner
}

/// 所有者是不是「管理员 / 系统」这类非当前用户的身份。
fn owner_is_privileged(owner: &str) -> bool {
    let o = owner.to_ascii_lowercase();
    o.contains("administrators") || o.contains("system")
}

/// 目录是否可写（探测法：建一个临时文件再删掉）。
/// 注意：**可写不等于能用** —— 被提权创建过的目录往往对当前用户仍然是「完全控制」，
/// 但 WebView2 自己会拒绝接管，所以这个结果只能当参考，不能当判据。
fn dir_writable(dir: &Path) -> bool {
    let _ = std::fs::create_dir_all(dir);
    let probe = dir.join(format!(".bcrm_wv_probe_{}.tmp", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 取路径（文件或目录）的所有者，用于身份比对。
fn path_owner(path: &Path) -> Option<String> {
    let script = format!(
        "if (Test-Path -LiteralPath '{p}') {{ (Get-Acl -LiteralPath '{p}').Owner }} else {{ '' }}",
        p = path.display()
    );
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 描述 WebView2 用户数据目录的现状，附上判断与修法。
fn webview_dir_hint(dir: &Path) -> String {
    let exists = dir.is_dir();
    let owner = if exists {
        path_owner(dir).unwrap_or_else(|| "(取不到)".to_string())
    } else {
        "(目录不存在)".to_string()
    };
    let writable = if exists { dir_writable(dir) } else { true };
    let suspicious = exists && owner_is_privileged(&owner);

    let mut s = String::new();
    s.push_str("WebView2 用户数据目录诊断：\n");
    s.push_str(&format!("  路径        : {}\n", dir.display()));
    s.push_str(&format!("  是否存在    : {}\n", if exists { "是" } else { "否" }));
    s.push_str(&format!("  所有者      : {owner}\n"));
    s.push_str(&format!(
        "  当前用户可写: {}\n",
        if writable { "是" } else { "否" }
    ));
    if suspicious {
        s.push_str(
            "\n⚠️ 所有者不是当前用户，而是 Administrators/System —— 这是「双击没反应」的\n\
             已知原因：该目录被提权进程创建过，普通用户身份下 WebView2 无法接管它。\n\
             删除该目录（只是浏览器缓存，不影响 crm.db 数据）后，以普通用户双击即可重建。\n",
        );
    }
    s
}

/// 窗口创建失败时：给出诊断，并问用户要不要删除该目录后重试。
/// 返回 true = 已经处理（调用方不必再弹 fatal 框）。
fn offer_webview_repair(err: &str) -> bool {
    let dir = WEBVIEW_DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| default_webview_data_dir(APP_IDENTIFIER_FALLBACK));
    let hint = webview_dir_hint(&dir);
    let msg = format!(
        "BCRM 界面无法创建。\n\n错误：{err}\n\n{hint}\n\
         ▶ 点「是」：删除上面的目录并重新启动程序。\n\
           该目录只是 WebView2 的浏览器缓存，删除不影响 crm.db 里的业务数据。\n\
         ▶ 点「否」：保持现状，只看日志（bcrm-launcher.log）。\n\n\
         说明：以后请【直接双击】启动，不要右键「以管理员身份运行」——\n\
         用管理员身份跑过之后，这个目录又会变成管理员所有，问题重现。"
    );
    if !ask_yes_no(&msg, "BCRM 界面创建失败") {
        return false;
    }

    log(&format!("尝试删除 WebView2 用户数据目录: {}", dir.display()));
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => log("已删除，准备重启"),
        Err(e) => {
            log(&format!("[ERROR] 删除失败: {e}"));
            show_error_box(&format!(
                "删除失败：{e}\n\n请手动删除下面这个目录（先关掉所有 BCRM 窗口），再重新双击程序：\n\n{}",
                dir.display()
            ));
            return false;
        }
    }

    // 以当前身份重新启动自己（父进程若是提权，子进程也是提权 —— 这点无法由程序改变，
    // 但至少用户不必再手动双击一次）
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = std::process::Command::new(&exe).spawn() {
                log(&format!("[ERROR] 重启失败: {e}"));
                show_error_box(&format!("重启失败，请手动双击一次程序。\n\n{e}"));
                return false;
            }
            log("已重新启动，本进程退出");
            std::process::exit(0);
        }
        Err(e) => {
            log(&format!("[ERROR] 取自身路径失败: {e}"));
            return false;
        }
    }
}

/// 检查 WebView2 运行时；缺失就给出官方在线安装链接并退出。
///
/// `tauri::webview_version()` 走的是 WebView2 自身的版本查询接口
/// （不是靠窗口创建成功与否来判断），所以能在**建窗口之前**就给用户一个
/// 可执行的下一步，而不是一闪而过。
fn ensure_webview2() {
    match tauri::webview_version() {
        Ok(v) => log(&format!("WebView2 运行时: v{v}")),
        Err(e) => {
            log(&format!("[ERROR] 未检测到 WebView2 运行时: {e}"));
            let msg = format!(
                "BCRM 客户管理系统需要「Microsoft Edge WebView2 运行时」来显示界面，\n\
                 本机没有检测到它。\n\n\
                 ▶ 点「是」：立刻打开微软官方在线安装页（约 2MB，需联网），\n\
                    下载并安装后重新双击本程序即可。\n\n\
                 也可手动访问：\n{WV2_BOOTSTRAPPER_URL}\n\n\
                 离线内网机器：访问 {WV2_HOME_URL}\n\
                 下载对应架构的 Evergreen 独立安装包，拷到本机安装即可。\n\n\
                 （安装包版本 BCRM客户管理系统_x64-setup.exe 会自动处理这一步，\n\
                    只有便携版 exe 需要手工装。）"
            );
            if ask_yes_no(&msg, "BCRM 缺少 WebView2 运行时") {
                open_url(WV2_BOOTSTRAPPER_URL);
            }
            std::process::exit(1);
        }
    }
}

/// `--db-check`：数据库权限自检。
///
/// 桌面版是 GUI 子系统，`println!` 没有地方可去，所以报告写到 exe 同级的
/// `bcrm-db-check.txt`，并用对话框给出结论摘要。
///
/// 加 `--quiet`（或设环境变量 `BCRM_NO_DIALOG=1`）可以跳过对话框，
/// 让自动化脚本/批处理能非交互地拿到退出码与报告文件。
fn run_db_check() -> i32 {
    let quiet = std::env::args().any(|a| a == "--quiet")
        || std::env::var("BCRM_NO_DIALOG").map(|v| v == "1").unwrap_or(false);

    let (report, code) = bcrm_backend::diagnostics_report();
    log(&report);

    let mut shown = report.clone();
    let mut saved_at: Option<PathBuf> = None;
    for candidate in [
        exe_dir().join("bcrm-db-check.txt"),
        std::env::temp_dir().join("bcrm-db-check.txt"),
    ] {
        if std::fs::write(&candidate, &report).is_ok() {
            saved_at = Some(candidate);
            break;
        }
    }
    match &saved_at {
        Some(p) => shown.push_str(&format!("\n\n报告已保存到：\n{}", p.display())),
        None => shown.push_str("\n\n（报告文件写入失败，以上为完整内容）"),
    }

    if !quiet {
        let _ = message_box(&shown, "BCRM 数据库自检", "OK", "Information");
    } else {
        log("[db-check] --quiet：已跳过对话框");
    }
    code
}

fn main() {
    log("--- bcrm-desktop 启动 ---");

    // ⓪ 自检模式：不建窗口、不起服务，跑完就退出（便于脚本/现场排查）
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--db-check" || a == "--db-info" || a == "--selftest") {
        std::process::exit(run_db_check());
    }

    // ① 前置环境检查：WebView2 缺失时在出窗口之前就给出可执行方案
    ensure_webview2();

    // ①' WebView2 用户数据目录：挑一个能写的，并把「被提权进程创建过」的坑当场修掉。
    //     必须在建窗口之前做 —— WebView2 一旦接管了坏目录，错误要到建窗口时才暴露。
    let identifier = APP_IDENTIFIER_FALLBACK.to_string();
    let mut wv_dir: Option<PathBuf> = None;
    for (cand, _portable, label) in webview_data_dir_candidates(&identifier) {
        // 只有上级目录可写才可能建出缓存目录；实测"能不能写"而不是靠猜
        let parent = match cand.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let _ = std::fs::create_dir_all(&parent);
        if !dir_writable(&parent) {
            log(&format!(
                "WebView2 数据目录候选位跳过（上级目录不可写）: {label}"
            ));
            continue;
        }
        log(&heal_webview_data_dir(&cand));
        wv_dir = Some(cand);
        break;
    }
    let wv_dir = wv_dir.unwrap_or_else(|| default_webview_data_dir(&identifier));
    let _ = WEBVIEW_DATA_DIR.set(wv_dir.clone());
    log(&format!("WebView2 用户数据目录: {}", wv_dir.display()));

    // ② 定位数据库：逐个候选位实测（目录可写 → 已有库可写 → 写探针），失败自动下移。
    //    这里拿到的一定是**可读写**的连接，安装到 Program Files 后普通用户双击也能打开。
    let (state, loc) = match AppState::open_default() {
        Ok(v) => v,
        Err(e) => fatal(&format!("初始化数据库失败: {e}")),
    };
    for s in &loc.skipped {
        log(&format!("数据库候选位跳过: {s}"));
    }
    log(&format!("数据库: {}", loc.brief()));
    let tables = state.reg.table_count();
    let app_router = build_app(state);
    log(&format!("表白名单: {tables} 张"));

    // ③ 先占住一个空闲端口，再把同一个 listener 交给 axum。
    //    先探端口再重新 bind 会有竞态（探测和真正监听之间端口可能被抢走），
    //    这里全程用同一个 socket，不存在这个问题。
    let std_listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => fatal(&format!("监听端口失败: {e}")),
    };
    if let Err(e) = std_listener.set_nonblocking(true) {
        fatal(&format!("设置非阻塞失败: {e}"));
    }
    let port = match std_listener.local_addr() {
        Ok(a) => a.port(),
        Err(e) => fatal(&format!("读取端口失败: {e}")),
    };
    let api_base = format!("http://127.0.0.1:{port}/api/v1");
    log(&format!("API 地址: {api_base}"));

    let init_script = format!(
        "window.__BCRM_API_BASE__ = {};",
        serde_json::to_string(&api_base).unwrap_or_else(|_| "\"\"".into())
    );

    let result = tauri::Builder::default()
        .setup(move |app| {
            /*
             * 把 axum 挂到 Tauri 的异步运行时上。
             *
             * ⚠️ 这里有个必须踩过才知道的坑：`TcpListener::from_std` 要求**在 Tokio 运行时
             * 上下文内**调用，而 Tauri 的 `setup` 回调跑在主线程、并不在那个运行时里。
             * 一旦在 setup 里直接 from_std，会 panic：
             *     there is no reactor running, must be called from the context of a Tokio 1.x runtime
             * 表现为「双击 exe 一闪而过、日志只写到 API 地址就没了、窗口永远不出现」。
             *
             * 所以：把 std listener **移动**进 spawn 出来的 async 块，在块内做转换。
             * listener 是已经 bind 好的，转换之前到达的连接会由操作系统的 backlog 排队，
             * 不会丢请求 —— 界面哪怕抢先发请求也能被正常处理。
             */
            tauri::async_runtime::spawn(async move {
                match tokio::net::TcpListener::from_std(std_listener) {
                    Ok(listener) => {
                        if let Err(e) = axum::serve(listener, app_router).await {
                            eprintln!("axum 退出: {e}");
                        }
                    }
                    Err(e) => eprintln!("转换监听器失败: {e}"),
                }
            });

            /*
             * 建窗口。initialization_script 在页面任何脚本之前执行，
             * 所以前端模块求值时 window.__BCRM_API_BASE__ 已经就位。
             *
             * ⚠️ 窗口必须**只在这里**创建，`tauri.conf.json` 的 `app.windows` 必须留空数组。
             *
             * 原因：Tauri 启动时会先把配置里声明的窗口自动建出来，然后才调用 setup 回调。
             * 早先配置里留了一个 `label: "main"` 的窗口，这里又建一个同名的，于是 setup 直接失败：
             *     Failed to setup app: error encountered during setup hook:
             *     创建窗口失败: a webview with label `main` already exists
             * 症状和上一个坑一模一样（一闪而过、日志停在「API 地址」、窗口不出现），
             * 区别只在 panic 文案 —— 修完 from_std 才会暴露到这一层。
             *
             * 不能改用配置声明窗口：那样就没地方挂 `initialization_script`，
             * 前端拿不到随机端口上的 API 地址，运行时会退回 mock 数据。
             * 所以：配置留空 → 代码独占创建权 → 端口地址注入与窗口参数在同一处维护。
             */
            // WebView2 用户数据目录已经在 main() 里选好并自愈过（WEBVIEW_DATA_DIR），
            // 这里只做一次一致性核对，然后**显式**把它交给窗口 —— 不再依赖 Tauri
            // 按 identifier 推断路径，也保证"挑到的目录 = 实际用的目录"。
            let cfg_identifier = app.config().identifier.clone();
            if cfg_identifier != APP_IDENTIFIER_FALLBACK {
                log(&format!(
                    "[WARN] tauri.conf.json 的 identifier 是 {cfg_identifier}，\
                     与 APP_IDENTIFIER_FALLBACK={APP_IDENTIFIER_FALLBACK} 不一致（仅影响诊断兜底路径）"
                ));
            }
            let wv_dir = WEBVIEW_DATA_DIR
                .get()
                .cloned()
                .unwrap_or_else(|| default_webview_data_dir(&cfg_identifier));
            let _ = std::fs::create_dir_all(&wv_dir);

            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("BCRM 客户管理系统")
                .inner_size(1500.0, 940.0)
                .min_inner_size(1100.0, 700.0)
                .center()
                .data_directory(wv_dir)
                .initialization_script(&init_script)
                .build()
                .map_err(|e| format!("创建窗口失败: {e}"))?;

            log("窗口已创建");
            Ok(())
        })
        .run(tauri::generate_context!());

    match result {
        Ok(()) => log("正常退出"),
        Err(e) => {
            /*
             * 创建窗口失败有两条已知路径：
             *   ① 缺 WebView2 运行时 —— 启动前 ensure_webview2() 已经拦掉；
             *   ② WebView2 用户数据目录被提权进程创建过 —— 只有走到建窗口这一步才会暴露，
             *      在这里拦。先把 ② 的诊断和一键修复给出来，用户处理掉就不必再看通用错误框。
             *
             * setup 回调里 `?` 抛出的字符串会被 Tauri 包成 `Error::Setup`，
             * 它的 Display 是 "error encountered during setup hook: <我们的文案>"。
             * 所以先取 Setup 分支拿回内层消息，再用我们自己拼的前缀「创建窗口失败」判断。
             * （不要直接对 tauri::Error 调 starts_with —— 它不是字符串。）
             */
            let setup_msg = match &e {
                tauri::Error::Setup(s) => Some(s.to_string()),
                _ => None,
            };
            if let Some(msg) = setup_msg {
                if msg.contains("创建窗口失败") && offer_webview_repair(&msg) {
                    return;
                }
            }
            fatal(&format!("Tauri 运行失败: {e}"));
        }
    }
}
