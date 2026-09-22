mod common;
use cc_core::model::{Category, Retention};
use cc_core::scan::{
    category_meta, classify_top_level, read_cleanup_days, scan, simulate_cleanup, CleanupDaysSource, ScanCache,
};
use common::FakeRoot;
use std::fs;
use std::time::{Duration, SystemTime};

fn set_mtime(p: &std::path::Path, ms_ago: u64) {
    let t = SystemTime::now() - Duration::from_millis(ms_ago);
    let f = fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_modified(t).unwrap();
}

fn bytes_of(entries: &[cc_core::model::CategoryEntry], cat: &Category) -> u64 {
    entries.iter().find(|e| &e.category == cat).map(|e| e.bytes).unwrap_or(0)
}

#[test]
fn classify_top_level_names() {
    assert_eq!(classify_top_level("file-history"), Category::FileHistory);
    assert_eq!(classify_top_level("history.jsonl"), Category::HistoryLog);
    assert_eq!(classify_top_level("stats-cache.json"), Category::StatsCache);
    assert_eq!(classify_top_level("todos"), Category::Legacy("todos".into()));
    assert_eq!(classify_top_level("plugins"), Category::Protected("plugins".into()));
    assert_eq!(classify_top_level("CLAUDE.md"), Category::Protected("CLAUDE.md".into()));
    assert_eq!(classify_top_level("daemon.lock"), Category::Protected("daemon.lock".into()));
    assert_eq!(classify_top_level("policy-limits.json.stamp.json"), Category::Protected("policy-limits.json.stamp.json".into()));
    assert_eq!(classify_top_level("something-new"), Category::Unknown("something-new".into()));
}

#[test]
fn meta_marks_retention_and_deletability() {
    let m = category_meta(&Category::Transcripts, 30);
    assert_eq!(m.retention, Retention::AutoCleanup { days: 30 });
    assert!(m.deletable_by_tool);
    assert!(!category_meta(&Category::AutoMemory, 30).deletable_by_tool);
    assert_eq!(category_meta(&Category::HistoryLog, 30).retention, Retention::Permanent);
    assert_eq!(category_meta(&Category::Unknown("x".into()), 30).retention, Retention::Unknown);
    assert!(!category_meta(&Category::Unknown("x".into()), 30).deletable_by_tool);
}

#[test]
fn cleanup_days_default_and_explicit() {
    let f = FakeRoot::new();
    assert_eq!(read_cleanup_days(f.path()), (30, CleanupDaysSource::Default));
    f.top_file("settings.json", br#"{"cleanupPeriodDays": 7, "other": true}"#);
    assert_eq!(read_cleanup_days(f.path()), (7, CleanupDaysSource::Explicit));
    f.top_file("settings.json", b"{ broken");
    assert_eq!(read_cleanup_days(f.path()), (30, CleanupDaysSource::Default));
}

#[test]
fn scans_projects_and_top_level_categories() {
    let f = FakeRoot::new();
    let t1 = f.project_file("C--a", "s1.jsonl", "0123456789"); // 10 B
    f.project_file("C--a", "subagents/x/agent.jsonl", "01234"); // 5 B
    f.project_file("C--a", "memory/MEMORY.md", "l1\nl2\nl3\n");
    f.project_file("C--a", "memory/topic.md", "xx");
    f.project_file("C--a", "weird/file.bin", "z");
    f.project_file("C--a", "11111111-2222-3333-4444-555555555555/subagents/agent-1.jsonl", "0123456789"); // 10 B
    f.project_file("C--a", "11111111-2222-3333-4444-555555555555/tool-results/r.txt", "abc"); // 3 B
    f.top_file("file-history/sess/abc@v1", &[0u8; 100]);
    f.top_file("history.jsonl", &[0u8; 7]);
    f.top_file("plugins/p/a.js", &[0u8; 3]);
    f.top_file("mystery/a", &[0u8; 11]);
    set_mtime(&t1, 1000);

    let mut cache = ScanCache::default();
    let mut progress_calls = 0;
    let r = scan(f.path(), &mut cache, cc_core::scan::now_ms(), &mut |_| progress_calls += 1);

    let a = &r.per_project["C--a"];
    assert_eq!(bytes_of(&a.size.by_category, &Category::Transcripts), 28);
    assert_eq!(bytes_of(&a.size.by_category, &Category::AutoMemory), 11);
    assert_eq!(bytes_of(&a.size.by_category, &Category::Unknown("projects/其他".into())), 1);
    assert_eq!(a.size.total_bytes, 40);
    assert_eq!(a.session_count, 1, "只数顶层 .jsonl");
    assert_eq!(a.memory_file_count, 2);
    assert_eq!(a.memory_md_lines, Some(3));
    let t1_mtime = cc_core::scan::mtime_ms(&fs::metadata(&t1).unwrap()).unwrap();
    assert!(a.last_active_ms.unwrap() >= t1_mtime, "活跃时间 = 转录 mtime 最大值");

    assert_eq!(bytes_of(&r.global, &Category::Transcripts), 28);
    assert_eq!(bytes_of(&r.global, &Category::FileHistory), 100);
    assert_eq!(bytes_of(&r.global, &Category::HistoryLog), 7);
    assert_eq!(bytes_of(&r.global, &Category::Protected("plugins".into())), 3);
    assert_eq!(bytes_of(&r.global, &Category::Unknown("mystery".into())), 11);
    assert_eq!(r.root_total_bytes, 40 + 100 + 7 + 3 + 11);
    assert!(progress_calls >= 5);
    assert!(cache.last_result.is_some());
    assert!(cache.top_level.contains_key("file-history"));
}

#[test]
fn top_level_cache_is_reused_when_mtime_unchanged_and_fresh() {
    let f = FakeRoot::new();
    f.top_file("file-history/s/a@v1", &[0u8; 10]);
    let mut cache = ScanCache::default();
    let now = cc_core::scan::now_ms();
    scan(f.path(), &mut cache, now, &mut |_| {});
    // 伪造缓存里的数据，再扫一次：目录 mtime 没变且在 24h 内 → 应沿用伪造值
    cache.top_level.get_mut("file-history").unwrap().entries[0].bytes = 999;
    let r2 = scan(f.path(), &mut cache, now + 1000, &mut |_| {});
    assert_eq!(bytes_of(&r2.global, &Category::FileHistory), 999);
    // 超过 24h → 重扫
    let r3 = scan(f.path(), &mut cache, now + 25 * 3600 * 1000, &mut |_| {});
    assert_eq!(bytes_of(&r3.global, &Category::FileHistory), 10);
}

#[test]
fn cleanup_simulation_counts_only_old_auto_cleanup_files() {
    let f = FakeRoot::new();
    let old_fh = f.top_file("file-history/s/old@v1", &[0u8; 50]);
    f.top_file("file-history/s/new@v1", &[0u8; 60]);
    let old_tr = f.project_file("C--a", "old.jsonl", "0123456789");
    let old_nested = f.project_file("C--a", "11111111-2222-3333-4444-555555555555/subagents/agent.jsonl", "01234");
    let old_mem = f.project_file("C--a", "memory/MEMORY.md", "keep me forever");
    let old_hist = f.top_file("history.jsonl", &[0u8; 70]);
    let day = 86_400_000u64;
    for p in [&old_fh, &old_tr, &old_nested, &old_mem, &old_hist] {
        set_mtime(p, 40 * day);
    }
    let now = cc_core::scan::now_ms();
    let pv = simulate_cleanup(f.path(), 30, now);
    assert_eq!(pv.files, 3);
    assert_eq!(pv.bytes, 65);
    assert_eq!(bytes_of(&pv.by_category, &Category::FileHistory), 50);
    assert_eq!(bytes_of(&pv.by_category, &Category::Transcripts), 15);
    assert_eq!(bytes_of(&pv.by_category, &Category::AutoMemory), 0);
    assert_eq!(bytes_of(&pv.by_category, &Category::HistoryLog), 0);
}
