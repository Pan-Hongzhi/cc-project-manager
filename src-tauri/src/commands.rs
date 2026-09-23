use crate::AppState;
use cc_core::cli::StdRunner;
use cc_core::engine::Overview;
use cc_core::sessions::WinProcessProbe;
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn get_overview(state: State<'_, AppState>) -> Result<Overview, String> {
    let engine = state.engine.lock().unwrap_or_else(|e| e.into_inner());
    Ok(engine.quick_overview(&WinProcessProbe, &StdRunner))
}

/// 后台全量刷新。扫描期间持有引擎锁；再次调用返回错误码 err.scanning（前端按语言翻译）。
#[tauri::command]
pub fn refresh(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.engine.try_lock().is_err() {
        return Err("err.scanning".into());
    }
    let app2 = app.clone();
    std::thread::spawn(move || {
        let state = app2.state::<AppState>();
        let mut engine = state.engine.lock().unwrap_or_else(|e| e.into_inner());
        let progress_app = app2.clone();
        let overview = engine.full_refresh(&WinProcessProbe, &StdRunner, &mut |p| {
            let _ = progress_app.emit("scan-progress", &p);
        });
        let _ = app2.emit("overview", &overview);
    });
    Ok(())
}

fn spawn_detached(program: &str, args: &[&str]) -> Result<(), String> {
    Command::new(program).args(args).spawn().map(|_| ()).map_err(|e| format!("err.spawn_failed|{program}|{e}"))
}

#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("err.path_not_dir|{path}"));
    }
    spawn_detached("explorer.exe", &[path.as_str()])
}

/// 打开 PowerShell 窗口并在项目目录运行 claude（设计 7.2）。
#[tauri::command]
pub fn run_claude(path: String, resume: bool) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("err.project_missing|{path}"));
    }
    let escaped = path.replace('\'', "''");
    let cmd = if resume { "claude --resume" } else { "claude" };
    let script = format!("Set-Location -LiteralPath '{escaped}'; {cmd}");
    spawn_detached("powershell.exe", &["-NoExit", "-Command", &script])
}

#[tauri::command]
pub fn open_insights_report(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.engine.lock().unwrap_or_else(|e| e.into_inner());
    let report = engine.root().root.join("usage-data").join("report.html");
    if !report.is_file() {
        return Err("err.no_insights_report".into());
    }
    let report_str = report.to_string_lossy();
    spawn_detached("explorer.exe", &[report_str.as_ref()])
}

#[tauri::command]
pub fn open_tool_data_dir() -> Result<(), String> {
    let dir = cc_core::cache::tool_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir_str = dir.to_string_lossy();
    spawn_detached("explorer.exe", &[dir_str.as_ref()])
}
