fn main() {
    // tauri-build 只在 tauri.conf.json / capabilities 变化时重跑，
    // 图标文件更新后 exe 内嵌的资源不会刷新，这里显式声明依赖。
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
