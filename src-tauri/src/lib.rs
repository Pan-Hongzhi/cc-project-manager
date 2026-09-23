mod commands;
mod crisp_icon;

use cc_core::cache::tool_data_dir;
use cc_core::engine::Engine;
use cc_core::paths::resolve_root;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub engine: Mutex<Engine>,
}

pub fn run() {
    let root = resolve_root().expect("USERPROFILE 未设置，无法启动");
    let engine = Engine::new(root, tool_data_dir());
    tauri::Builder::default()
        .manage(AppState { engine: Mutex::new(engine) })
        .invoke_handler(tauri::generate_handler![
            commands::get_overview,
            commands::refresh,
            commands::open_in_explorer,
            commands::run_claude,
            commands::open_insights_report,
            commands::open_tool_data_dir,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 事件循环就绪后窗口一定已创建，此时设置标题栏 / 任务栏图标才不会被创建流程覆盖
            if let tauri::RunEvent::Ready = event {
                if let Some(window) = app.get_webview_window("main") {
                    crisp_icon::apply(&window);
                }
            }
        });
}
