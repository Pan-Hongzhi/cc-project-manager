mod commands;

use cc_core::cache::tool_data_dir;
use cc_core::engine::Engine;
use cc_core::paths::resolve_root;
use std::sync::Mutex;

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
