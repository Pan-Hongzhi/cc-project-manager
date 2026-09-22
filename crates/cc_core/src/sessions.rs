//! 运行中会话检测。sessions/<pid>.json 会残留僵尸文件且 PID 会被复用（设计 F6），
//! 因此做三重校验：PID 存活 + 映像名 + 进程启动时间。
use crate::encoding::is_same_or_child;
use crate::model::{RunningSession, Verified};
use serde_json::Value;
use std::path::Path;

pub const HOST_IMAGES: [&str; 3] = ["claude.exe", "node.exe", "bun.exe"];
/// FILETIME 单位 100 ns；2 s 容差
pub const START_TOLERANCE_TICKS: u64 = 20_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    NotFound,
    Unknown,
    Found { image_name: String, creation_filetime: u64 },
}

pub trait ProcessProbe {
    fn probe(&self, pid: u32) -> ProbeResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub started_at_ms: i64,
    pub proc_start: Option<u64>,
    pub status: String,
}

pub fn parse_session_file(text: &str) -> Option<SessionRecord> {
    let v: Value = serde_json::from_str(text).ok()?;
    let pid = v.get("pid")?.as_u64()? as u32;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Some(SessionRecord {
        pid,
        session_id: s("sessionId"),
        cwd: s("cwd"),
        started_at_ms: v.get("startedAt").and_then(Value::as_i64).unwrap_or(0),
        proc_start: v.get("procStart").and_then(Value::as_str).and_then(|t| t.parse().ok()),
        status: s("status"),
    })
}

/// 规范化映像文件名以便与 HOST_IMAGES 比对：转小写，并截断到首个 ".exe" 之后
/// （CC 自更新会把正在运行的 claude.exe 重命名为 claude.exe.old.<ts>，进程仍存活）。
/// 找不到 ".exe" 时原样返回小写结果。
fn host_image_name(image_name: &str) -> String {
    let lower = image_name.to_ascii_lowercase();
    match lower.find(".exe") {
        Some(i) => lower[..i + 4].to_string(),
        None => lower,
    }
}

pub fn verify(rec: &SessionRecord, probe: &dyn ProcessProbe) -> Verified {
    match probe.probe(rec.pid) {
        ProbeResult::NotFound => Verified::Stale,
        ProbeResult::Unknown => Verified::Unknown,
        ProbeResult::Found { image_name, creation_filetime } => {
            let img = host_image_name(&image_name);
            if !HOST_IMAGES.contains(&img.as_str()) {
                return Verified::Stale;
            }
            if let Some(ps) = rec.proc_start {
                if ps.abs_diff(creation_filetime) > START_TOLERANCE_TICKS {
                    return Verified::Stale;
                }
            }
            Verified::Alive
        }
    }
}

pub fn list_sessions(root: &Path, probe: &dyn ProcessProbe) -> Vec<RunningSession> {
    let Ok(rd) = std::fs::read_dir(root.join("sessions")) else { return Vec::new() };
    let mut out = Vec::new();
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        if p.extension().map_or(true, |x| x != "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let Some(rec) = parse_session_file(&text) else { continue };
        let verified = verify(&rec, probe);
        out.push(RunningSession {
            pid: rec.pid,
            session_id: rec.session_id,
            cwd: rec.cwd,
            started_at_ms: rec.started_at_ms,
            status: rec.status,
            verified,
        });
    }
    out.sort_by_key(|s| s.pid);
    out
}

/// 项目运行中 = 存在 Alive/Unknown 会话，其 cwd 等于项目路径或是其子目录。
pub fn session_for_project(sessions: &[RunningSession], real_path: &str) -> Option<RunningSession> {
    sessions
        .iter()
        .find(|s| s.verified != Verified::Stale && is_same_or_child(&s.cwd, real_path))
        .cloned()
}

#[cfg(windows)]
pub struct WinProcessProbe;

#[cfg(windows)]
impl ProcessProbe for WinProcessProbe {
    fn probe(&self, pid: u32) -> ProbeResult {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, FILETIME};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        const STILL_ACTIVE: u32 = 259;
        // SAFETY: 纯 Win32 查询调用；句柄在所有分支中都被关闭；缓冲区长度按 API 约定传入。
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return if GetLastError() == ERROR_INVALID_PARAMETER { ProbeResult::NotFound } else { ProbeResult::Unknown };
            }
            let mut exit_code: u32 = 0;
            if GetExitCodeProcess(h, &mut exit_code) != 0 && exit_code != STILL_ACTIVE {
                CloseHandle(h);
                return ProbeResult::NotFound;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok_name = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let (mut c, mut e, mut k, mut u) = (zero, zero, zero, zero);
            let ok_time = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
            CloseHandle(h);
            if ok_name == 0 || ok_time == 0 {
                return ProbeResult::Unknown;
            }
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            let image_name = full.rsplit(['\\', '/']).next().unwrap_or("").to_string();
            let creation_filetime = ((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64;
            ProbeResult::Found { image_name, creation_filetime }
        }
    }
}
