//! Tauri 构建脚本。
//!
//! 两件事：
//!   1. 把 `tauri.conf.json` 里的信息（图标、Windows manifest、权限）编进 exe 资源段；
//!   2. 让前端产物变化时能触发重新编译 —— 否则改完前端直接 cargo build，
//!      exe 里嵌的还是上一次的界面（这种"改了没生效"最难排查）。
fn main() {
    // 前端构建产物被打进二进制，所以它的变化必须让 cargo 重新跑一遍
    println!("cargo:rerun-if-changed=../frontend/dist");
    let dist = std::path::Path::new("../frontend/dist");
    if dist.is_dir() {
        if let Ok(entries) = collect(dist) {
            for e in entries {
                println!("cargo:rerun-if-changed={e}");
            }
        }
    }
    tauri_build::build()
}

fn collect(dir: &std::path::Path) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            out.extend(collect(&p)?);
        } else {
            out.push(p.to_string_lossy().to_string());
        }
    }
    Ok(out)
}
