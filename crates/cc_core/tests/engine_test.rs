mod common;
use cc_core::cli::{CommandOutput, CommandRunner};
use cc_core::engine::Engine;
use cc_core::model::{Category, ProjectState, Verified};
use cc_core::sessions::{ProbeResult, ProcessProbe};
use common::FakeRoot;
use serde_json::json;

struct NoProcess;
impl ProcessProbe for NoProcess {
    fn probe(&self, _: u32) -> ProbeResult {
        ProbeResult::NotFound
    }
}
struct AllAlive;
impl ProcessProbe for AllAlive {
    fn probe(&self, _: u32) -> ProbeResult {
        ProbeResult::Found { image_name: "claude.exe".into(), creation_filetime: 500 }
    }
}
struct NoCli;
impl CommandRunner for NoCli {
    fn run(&self, _: &str, _: &[&str]) -> std::io::Result<CommandOutput> {
        Ok(CommandOutput { status: 1, stdout: String::new(), stderr: String::new() })
    }
}

fn assistant(id: &str, input: u64) -> String {
    format!(r#"{{"type":"assistant","message":{{"id":"{id}","usage":{{"input_tokens":{input},"output_tokens":1}}}}}}"#)
}

#[test]
fn full_refresh_assembles_everything_and_quick_overview_reuses_cache() {
    let mut f = FakeRoot::new();
    // 真实存在的目录当作项目路径，让状态为 Normal
    let real = f.dir.path().join("realproj");
    std::fs::create_dir_all(real.join(".claude").join("rules")).unwrap();
    std::fs::write(real.join("CLAUDE.md"), "# rules").unwrap();
    let key = real.to_string_lossy().replace('\\', "/");
    f.config_entry(&key, json!({"hasTrustDialogAccepted": true}));
    let enc = cc_core::encoding::encode_path(&key);
    f.project_file(&enc, "s1.jsonl", &assistant("m1", 42));
    f.project_file(&enc, "memory/MEMORY.md", "a\nb\n");
    f.session(7, &real.to_string_lossy(), Some(500));
    f.top_file("stats-cache.json", br#"{"version":5,"totalSessions":3,"modelUsage":{}}"#);

    let data_dir = f.dir.path().join("tooldata");
    let mut eng = Engine::new(f.root.clone(), data_dir.clone());
    let ov = eng.full_refresh(&AllAlive, &NoCli, &mut |_| {});

    assert!(ov.config_error.is_none());
    let p = ov.projects.iter().find(|p| p.id == key).expect("project present");
    assert_eq!(p.state, ProjectState::Normal);
    assert_eq!(p.running.as_ref().unwrap().verified, Verified::Alive);
    assert_eq!(p.usage.as_ref().unwrap().input, 42);
    assert_eq!(p.usage.as_ref().unwrap().session_count, 1);
    assert!(p.last_active_ms.is_some());
    let ms = p.memory_summary.as_ref().unwrap();
    assert_eq!(ms.memory_md_lines, Some(2));
    assert!(ms.user_claude_md && ms.user_rules_dir && !ms.user_claude_local_md);
    assert!(p.size.as_ref().unwrap().by_category.iter().any(|e| e.category == Category::Transcripts));
    assert!(ov.scan.is_some() && !ov.scan_is_cached);
    assert!(ov.cleanup_preview.is_some());
    assert!(ov.stats.available);
    assert!(ov.cli.is_none());
    assert!(ov.self_check.migration_enabled);
    assert!(!ov.category_meta.is_empty());
    assert!(data_dir.join("cache").join("scan.json").is_file());
    assert!(data_dir.join("cache").join("usage.json").is_file());

    // 新 Engine 从磁盘缓存恢复：quick_overview 不扫描也能给出上次体积
    let eng2 = Engine::new(f.root.clone(), data_dir);
    let quick = eng2.quick_overview(&NoProcess, &NoCli);
    assert!(quick.scan_is_cached);
    assert!(quick.cleanup_preview.is_some(), "快速视图要带出上次的清扫模拟结果");
    assert_eq!(quick.scan.as_ref().unwrap().root_total_bytes, ov.scan.as_ref().unwrap().root_total_bytes);
    let p2 = quick.projects.iter().find(|p| p.id == key).unwrap();
    assert!(p2.running.is_none(), "进程探测说没进程 → 不算运行中");
    assert!(p2.size.is_some(), "quick 视图也带上次扫描的体积");
    let u2 = p2.usage.as_ref().expect("cached path must still report usage");
    assert_eq!(u2.session_count, 1, "会话数必须从扫描缓存带出，不能因重启变成 0");
    assert_eq!(u2.input, 42);
}

#[test]
fn missing_config_is_reported_not_fatal() {
    let f = FakeRoot::new();
    std::fs::remove_file(&f.root.config_file).unwrap();
    f.project_dir("C--x");
    let eng = Engine::new(f.root.clone(), f.dir.path().join("td"));
    let ov = eng.quick_overview(&NoProcess, &NoCli);
    assert!(ov.config_error.is_some());
    assert_eq!(ov.projects.len(), 1);
    assert_eq!(ov.projects[0].state, ProjectState::Unowned);
    assert!(!ov.self_check.migration_enabled);
}

#[test]
fn cache_helpers_roundtrip_and_reject_wrong_schema() {
    use cc_core::cache::{load_json, save_json};
    use cc_core::scan::ScanCache;
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("nested").join("scan.json");
    save_json(&p, &ScanCache::default()).unwrap();
    save_json(&p, &ScanCache::default()).unwrap();
    assert!(load_json::<ScanCache>(&p, cc_core::scan::SCAN_SCHEMA_VERSION).is_some());
    assert!(load_json::<ScanCache>(&p, 99).is_none());
    assert!(load_json::<ScanCache>(&d.path().join("missing.json"), 1).is_none());
}
