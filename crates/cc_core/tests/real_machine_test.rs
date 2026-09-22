//! 真机只读验收辅助：打印 Overview 摘要。默认 ignore，手动运行：
//! cargo test -p cc_core --test real_machine_test -- --ignored --nocapture
use cc_core::cli::StdRunner;
use cc_core::engine::Engine;
use cc_core::model::{ProjectState, Verified};
use cc_core::paths::resolve_root;
use cc_core::sessions::WinProcessProbe;
use std::collections::BTreeMap;

#[test]
#[ignore = "读取真实数据根目录（只读），仅用于人工验收"]
fn print_real_machine_overview() {
    let root = resolve_root().unwrap();
    let data_dir = std::env::temp_dir().join("cc-project-manager-acceptance");
    let mut engine = Engine::new(root, data_dir);
    let started = std::time::Instant::now();
    let ov = engine.full_refresh(&WinProcessProbe, &StdRunner, &mut |_| {});
    let elapsed = started.elapsed();
    println!("root = {:?} ({:?})", ov.root.root, ov.root.source);
    println!("config_error = {:?}", ov.config_error);
    println!("cli = {:?}", ov.cli);
    println!("full_refresh took {:?}", elapsed);
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    for p in &ov.projects {
        *states.entry(format!("{:?}", p.state)).or_default() += 1;
    }
    println!("projects = {} by state = {:?}", ov.projects.len(), states);
    for p in ov.projects.iter().filter(|p| matches!(p.state, ProjectState::LegacyEncoded | ProjectState::Unowned)) {
        println!("  {:?}: {:?} legacy_of={:?}", p.state, p.encoded_dir, p.legacy_of);
    }
    println!("self_check = {:?}", ov.self_check);
    let alive = ov.sessions.iter().filter(|s| s.verified == Verified::Alive).count();
    let stale = ov.sessions.iter().filter(|s| s.verified == Verified::Stale).count();
    println!("sessions = {} alive={} stale={} unknown={}", ov.sessions.len(), alive, stale, ov.sessions.len() - alive - stale);
    for s in &ov.sessions {
        println!("  pid={} {:?} cwd={}", s.pid, s.verified, s.cwd);
    }
    let scan = ov.scan.as_ref().unwrap();
    println!("cleanup_days = {} ({:?}) root_total_bytes = {} inaccessible = {}", scan.cleanup_days, scan.cleanup_days_source, scan.root_total_bytes, scan.inaccessible);
    for e in &scan.global {
        println!("  {:?}: {} bytes, {} files", e.category, e.bytes, e.file_count);
    }
    if let Some(pv) = &ov.cleanup_preview {
        println!("cleanup_preview: {} files, {} bytes", pv.files, pv.bytes);
    }
    println!("stats available={} version={:?} last_computed={:?} models={}", ov.stats.available, ov.stats.version, ov.stats.last_computed_date, ov.stats.models.len());
    let with_usage = ov.projects.iter().filter(|p| p.usage.as_ref().map_or(false, |u| u.message_count > 0)).count();
    println!("projects with token usage > 0: {}", with_usage);
    let sample = ov.projects.iter().find(|p| p.usage.as_ref().map_or(false, |u| u.message_count > 0));
    if let Some(p) = sample {
        println!("  sample {:?}: usage={:?} size={:?}", p.real_path, p.usage, p.size.as_ref().map(|s| s.total_bytes));
    }
}
