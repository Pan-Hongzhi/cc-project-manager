mod common;
use cc_core::model::Verified;
use cc_core::sessions::{list_sessions, parse_session_file, session_for_project, verify, ProbeResult, ProcessProbe, SessionRecord};
use common::FakeRoot;
use std::collections::HashMap;

struct FakeProbe(HashMap<u32, ProbeResult>);
impl ProcessProbe for FakeProbe {
    fn probe(&self, pid: u32) -> ProbeResult {
        self.0.get(&pid).cloned().unwrap_or(ProbeResult::NotFound)
    }
}

fn rec(pid: u32, proc_start: Option<u64>) -> SessionRecord {
    SessionRecord { pid, session_id: "s".into(), cwd: r"C:\proj\a".into(), started_at_ms: 0, proc_start, status: "idle".into() }
}

#[test]
fn parses_real_session_shape_and_tolerates_missing_fields() {
    let text = r#"{"pid":18644,"sessionId":"815d","cwd":"C:\\Users\\u\\proj","startedAt":1789901109617,"procStart":"134343747038091623","version":"2.1.278","status":"idle","extra":{"a":1}}"#;
    let r = parse_session_file(text).unwrap();
    assert_eq!(r.pid, 18644);
    assert_eq!(r.cwd, r"C:\Users\u\proj");
    assert_eq!(r.proc_start, Some(134343747038091623));
    assert_eq!(r.started_at_ms, 1789901109617);
    assert!(parse_session_file(r#"{"pid":1}"#).is_some(), "只有 pid 也算合法记录");
    assert!(parse_session_file(r#"{"cwd":"x"}"#).is_none(), "没有 pid 就不是会话记录");
    assert!(parse_session_file("garbage").is_none());
}

#[test]
fn verify_three_checks() {
    let probe = FakeProbe(HashMap::from([
        (1, ProbeResult::Found { image_name: "claude.exe".into(), creation_filetime: 1_000_000_000 }),
        (2, ProbeResult::Found { image_name: "StartMenuExperienceHost.exe".into(), creation_filetime: 1_000_000_000 }),
        (3, ProbeResult::Found { image_name: "node.exe".into(), creation_filetime: 1_000_000_000 + 50_000_000 }),
        (4, ProbeResult::Unknown),
        (5, ProbeResult::Found { image_name: "Node.EXE".into(), creation_filetime: 1_000_000_000 + 10_000_000 }),
        (6, ProbeResult::Found { image_name: "claude.exe.old.1758530000".into(), creation_filetime: 1_000_000_000 }),
    ]));
    assert_eq!(verify(&rec(1, Some(1_000_000_000)), &probe), Verified::Alive);
    assert_eq!(verify(&rec(2, Some(1_000_000_000)), &probe), Verified::Stale, "PID 被其他程序复用");
    assert_eq!(verify(&rec(3, Some(1_000_000_000)), &probe), Verified::Stale, "启动时间差 5 s 超过容差");
    assert_eq!(verify(&rec(4, Some(1_000_000_000)), &probe), Verified::Unknown, "拿不到信息 → 保守");
    assert_eq!(verify(&rec(5, Some(1_000_000_000)), &probe), Verified::Alive, "映像名大小写不敏感，1 s 差在容差内");
    assert_eq!(verify(&rec(9, Some(1)), &probe), Verified::Stale, "PID 不存在");
    assert_eq!(verify(&rec(1, None), &probe), Verified::Alive, "没有 procStart 时只做前两重校验");
    assert_eq!(verify(&rec(6, Some(1_000_000_000)), &probe), Verified::Alive, "自更新后重命名的映像名仍算 claude.exe");
}

#[test]
fn list_and_match_project() {
    let f = FakeRoot::new();
    f.session(100, r"C:\proj\a\sub", Some(500));
    f.session(200, r"C:\proj\b", Some(500));
    f.top_file("sessions/100.abc.key", b"secret");
    f.top_file("sessions/broken.json", b"{");
    let probe = FakeProbe(HashMap::from([
        (100, ProbeResult::Found { image_name: "claude.exe".into(), creation_filetime: 500 }),
    ]));
    let sessions = list_sessions(f.path(), &probe);
    assert_eq!(sessions.len(), 2);
    let alive = sessions.iter().find(|s| s.pid == 100).unwrap();
    assert_eq!(alive.verified, Verified::Alive);
    assert_eq!(sessions.iter().find(|s| s.pid == 200).unwrap().verified, Verified::Stale);

    assert!(session_for_project(&sessions, "C:/proj/a").is_some(), "cwd 是项目子目录 → 运行中");
    assert!(session_for_project(&sessions, "C:/proj/b").is_none(), "Stale 不算运行中");
    assert!(session_for_project(&sessions, "C:/proj").is_some(), "父目录也会匹配到子目录里的会话");
    assert!(session_for_project(&sessions, "C:/other").is_none());
}

#[test]
#[ignore = "读取真实 sessions/ 目录，手动运行：cargo test -p cc_core --test sessions_test real_machine -- --ignored --nocapture"]
fn real_machine_sessions_smoke() {
    let root = cc_core::paths::resolve_root().unwrap();
    let sessions = list_sessions(&root.root, &cc_core::sessions::WinProcessProbe);
    for s in &sessions {
        println!("pid={} verified={:?} cwd={}", s.pid, s.verified, s.cwd);
    }
    assert!(sessions.iter().any(|s| s.verified == Verified::Alive), "当前 CC 正在运行，至少应有一个 Alive");
}

#[test]
fn session_is_assigned_only_to_the_deepest_matching_project() {
    use cc_core::sessions::assign_sessions;
    let f = FakeRoot::new();
    f.session(300, r"C:\proj\a\sub", Some(500));
    let probe = FakeProbe(HashMap::from([
        (300, ProbeResult::Found { image_name: "claude.exe".into(), creation_filetime: 500 }),
    ]));
    let sessions = list_sessions(f.path(), &probe);
    let owners = assign_sessions(&sessions, &["C:/proj", "C:/proj/a", "C:/other"]);
    assert!(owners.contains_key("C:/proj/a"), "cwd 所在的最深项目拥有该会话");
    assert!(!owners.contains_key("C:/proj"), "父目录项目不能把子项目的会话算成自己的");
    assert!(!owners.contains_key("C:/other"));
}
