//! 让 Windows 标题栏 / 任务栏图标在小尺寸下保持清晰。
//!
//! Tauri 默认把 `icon.ico` 解码成一张位图创建成单一 HICON 再设为窗口小图标，
//! 与系统实际绘制尺寸不一致时会被缩放而发糊。这里内嵌为每个尺寸单独绘制的 PNG
//! （由 design/icons/build_ico.py 生成到 icons/win/），启动后按系统当前 DPI 给出的
//! 小图标 / 大图标尺寸选取对应 PNG 创建 HICON，标题栏与任务栏 1:1 绘制。

#[cfg(windows)]
const FRAMES: &[(i32, &[u8])] = &[
    (16, include_bytes!("../icons/win/16.png")),
    (20, include_bytes!("../icons/win/20.png")),
    (24, include_bytes!("../icons/win/24.png")),
    (28, include_bytes!("../icons/win/28.png")),
    (32, include_bytes!("../icons/win/32.png")),
    (40, include_bytes!("../icons/win/40.png")),
    (48, include_bytes!("../icons/win/48.png")),
    (56, include_bytes!("../icons/win/56.png")),
    (64, include_bytes!("../icons/win/64.png")),
];

/// 选出不小于目标尺寸的最小帧；都不够就用最大帧（由系统缩小，比放大清楚）。
#[cfg(windows)]
fn pick(size: i32) -> &'static [u8] {
    FRAMES
        .iter()
        .find(|(s, _)| *s >= size)
        .map(|(_, b)| *b)
        .unwrap_or(FRAMES[FRAMES.len() - 1].1)
}

#[cfg(windows)]
pub fn apply(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Foundation::{GetLastError, HWND};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateIconFromResourceEx, GetSystemMetrics, SendMessageW, ICON_BIG, ICON_SMALL,
        LR_DEFAULTCOLOR, SM_CXICON, SM_CXSMICON, WM_SETICON,
    };

    let hwnd = match window.hwnd() {
        Ok(h) => h.0 as HWND,
        Err(e) => {
            log_failure(&format!("hwnd() failed: {e}"));
            return;
        }
    };

    // SAFETY: 纯 Win32 调用；PNG 字节为编译期内嵌的只读数据；
    // 创建出的图标交给窗口持有（进程生命周期内不释放），WM_SETICON 参数按 API 约定传入。
    unsafe {
        // 应用是 DPI 感知的，这两个值已是物理像素（100% → 16/32，125% → 20/40，150% → 24/48）
        let small = GetSystemMetrics(SM_CXSMICON);
        let big = GetSystemMetrics(SM_CXICON);
        for (kind, size) in [(ICON_SMALL, small), (ICON_BIG, big)] {
            let png = pick(size);
            let hicon = CreateIconFromResourceEx(
                png.as_ptr(),
                png.len() as u32,
                1,          // 图标而非光标
                0x0003_0000, // 资源格式版本，固定值
                size,
                size,
                LR_DEFAULTCOLOR,
            );
            if hicon.is_null() {
                log_failure(&format!(
                    "CreateIconFromResourceEx({size}px) failed, GetLastError={}",
                    GetLastError()
                ));
            } else {
                SendMessageW(hwnd, WM_SETICON, kind as usize, hicon as isize);
            }
        }
    }
}

/// 图标设置失败只影响观感，不能影响启动；把原因写到 %TEMP%\cc-project-manager-icon.log 便于排查。
#[cfg(windows)]
fn log_failure(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("cc-project-manager-icon.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

#[cfg(not(windows))]
pub fn apply(_window: &tauri::WebviewWindow) {}
