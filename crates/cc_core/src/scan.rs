//! 分类空间扫描（只读）。类别与保留策略见设计 6.3 表；白名单见设计 6.2。
use crate::model::{Category, CategoryEntry, CategoryMeta, Retention, SizeBreakdown};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub const SCAN_SCHEMA_VERSION: u32 = 1;
pub const REUSE_WINDOW_MS: i64 = 24 * 3600 * 1000;
const DAY_MS: i64 = 86_400_000;

const PROTECTED_EXACT: &[&str] = &[
    ".credentials.json", "settings.json", "settings.local.json", "plugins", "agent-memory", "jobs", "daemon",
    "CLAUDE.md", "memory", "skills", "hooks", "sessions", "telemetry", "cache", "feedback",
    ".last-cleanup", ".last-update-result.json", "remote-settings.json",
];
const LEGACY: &[&str] = &["todos", "statsig", "logs", "image-cache"];
/// 参与 30 天清扫的顶层目录（不含 projects/，它按文件单独处理）
const AUTO_CLEANUP_DIRS: &[(&str, Category)] = &[
    ("file-history", Category::FileHistory),
    ("paste-cache", Category::PasteCache),
    ("uploads", Category::Uploads),
    ("debug", Category::Debug),
    ("plans", Category::Plans),
    ("tasks", Category::Tasks),
    ("session-env", Category::SessionEnv),
];

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn mtime_ms(md: &fs::Metadata) -> Option<i64> {
    md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64)
}

fn is_protected_name(name: &str) -> bool {
    PROTECTED_EXACT.contains(&name)
        || name.starts_with("daemon")
        || name.ends_with(".lock")
        || name.starts_with("policy-limits")
}

pub fn classify_top_level(name: &str) -> Category {
    if let Some((_, c)) = AUTO_CLEANUP_DIRS.iter().find(|(n, _)| *n == name) {
        return c.clone();
    }
    match name {
        "history.jsonl" => Category::HistoryLog,
        "stats-cache.json" => Category::StatsCache,
        n if LEGACY.contains(&n) => Category::Legacy(n.to_string()),
        n if is_protected_name(n) => Category::Protected(n.to_string()),
        n => Category::Unknown(n.to_string()),
    }
}

/// projects/<dir>/ 内部：memory/ → AutoMemory；.jsonl、subagents/、tool-results/ → Transcripts；其他 → Unknown
fn classify_in_project(first_component: &str, is_jsonl: bool) -> Category {
    match first_component {
        "memory" => Category::AutoMemory,
        "subagents" | "tool-results" => Category::Transcripts,
        _ if is_jsonl => Category::Transcripts,
        other => Category::Unknown(format!("projects/{other}")),
    }
}

pub fn category_meta(cat: &Category, cleanup_days: u32) -> CategoryMeta {
    let auto = Retention::AutoCleanup { days: cleanup_days };
    let (retention, deletable, consequence) = match cat {
        Category::Transcripts => (auto, true, "失去 resume/continue 与会话回溯"),
        Category::AutoMemory => (Retention::MemoryRule, false, "失去 CC 积累的项目知识（第一版不开放删除）"),
        Category::FileHistory => (auto, false, "失去编辑前快照的 checkpoint 回滚"),
        Category::PasteCache | Category::Uploads | Category::Debug | Category::Plans | Category::Tasks | Category::SessionEnv => {
            (auto, false, "临时/辅助数据，CC 会自动清扫")
        }
        Category::HistoryLog => (Retention::Permanent, false, "全部 prompt 历史，永久保留"),
        Category::StatsCache => (Retention::Permanent, false, "/usage 聚合数据，永久保留"),
        Category::Legacy(_) => (Retention::LegacyRemoved, false, "旧版本遗留目录，CC 清扫时会移除"),
        Category::Protected(_) => (Retention::Protected, false, "受保护：凭据、配置、插件或运行时状态，工具永不触碰"),
        Category::Unknown(_) => (Retention::Unknown, false, "未识别数据，只展示大小"),
    };
    CategoryMeta { category: cat.clone(), retention, deletable_by_tool: deletable, consequence: consequence.to_string() }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupDaysSource {
    Explicit,
    Default,
}

pub fn read_cleanup_days(root: &Path) -> (u32, CleanupDaysSource) {
    let explicit = fs::read_to_string(root.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("cleanupPeriodDays").and_then(serde_json::Value::as_u64))
        .map(|d| d as u32);
    match explicit {
        Some(d) => (d, CleanupDaysSource::Explicit),
        None => (30, CleanupDaysSource::Default),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectScan {
    pub size: SizeBreakdown,
    pub last_active_ms: Option<i64>,
    pub session_count: u64,
    pub memory_file_count: u32,
    pub memory_md_lines: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    pub scanned_at_ms: i64,
    pub root_total_bytes: u64,
    pub global: Vec<CategoryEntry>,
    pub per_project: BTreeMap<String, ProjectScan>,
    pub inaccessible: u64,
    pub cleanup_days: u32,
    pub cleanup_days_source: CleanupDaysSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopLevelCache {
    pub mtime_ms: i64,
    pub scanned_at_ms: i64,
    pub entries: Vec<CategoryEntry>,
    pub inaccessible: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanCache {
    pub schema_version: u32,
    pub top_level: BTreeMap<String, TopLevelCache>,
    pub last_result: Option<ScanResult>,
}

impl Default for ScanCache {
    fn default() -> Self {
        ScanCache { schema_version: SCAN_SCHEMA_VERSION, top_level: BTreeMap::new(), last_result: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanProgress {
    pub done: usize,
    pub total: usize,
    pub current: String,
    pub bytes_so_far: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupPreview {
    pub cleanup_days: u32,
    pub files: u64,
    pub bytes: u64,
    pub by_category: Vec<CategoryEntry>,
}

/// 按类别累加器
#[derive(Default)]
struct Acc {
    map: BTreeMap<Category, CategoryEntry>,
}

impl Acc {
    fn add(&mut self, cat: Category, bytes: u64, mtime: Option<i64>) {
        self.add_entry(&CategoryEntry { category: cat, bytes, file_count: 1, oldest_mtime_ms: mtime, newest_mtime_ms: mtime });
    }
    fn add_entry(&mut self, e: &CategoryEntry) {
        let slot = self.map.entry(e.category.clone()).or_insert_with(|| CategoryEntry {
            category: e.category.clone(),
            bytes: 0,
            file_count: 0,
            oldest_mtime_ms: None,
            newest_mtime_ms: None,
        });
        slot.bytes += e.bytes;
        slot.file_count += e.file_count;
        slot.oldest_mtime_ms = match (slot.oldest_mtime_ms, e.oldest_mtime_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        slot.newest_mtime_ms = match (slot.newest_mtime_ms, e.newest_mtime_ms) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
    }
    fn total(&self) -> u64 {
        self.map.values().map(|e| e.bytes).sum()
    }
    fn files(&self) -> u64 {
        self.map.values().map(|e| e.file_count).sum()
    }
    fn into_entries(self) -> Vec<CategoryEntry> {
        self.map.into_values().collect()
    }
}

/// 递归遍历普通文件；不跟随符号链接/重解析点；返回无法访问的条目数。
fn walk_files(dir: &Path, mut f: impl FnMut(&Path, &fs::Metadata)) -> u64 {
    let mut bad = 0;
    for entry in WalkDir::new(dir).follow_links(false) {
        match entry {
            Ok(e) => {
                let ft = e.file_type();
                if ft.is_symlink() || !ft.is_file() {
                    continue;
                }
                match e.metadata() {
                    Ok(md) => f(e.path(), &md),
                    Err(_) => bad += 1,
                }
            }
            Err(_) => bad += 1,
        }
    }
    bad
}

fn first_component(rel: &Path) -> String {
    rel.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default()
}

fn scan_project_dir(dir: &Path) -> (ProjectScan, u64) {
    let mut acc = Acc::default();
    let mut ps = ProjectScan::default();
    let bad = walk_files(dir, |p, md| {
        let rel = p.strip_prefix(dir).unwrap_or(p);
        let first = first_component(rel);
        let depth = rel.components().count();
        let is_jsonl = p.extension().map_or(false, |x| x == "jsonl");
        let cat = classify_in_project(&first, is_jsonl);
        let m = mtime_ms(md);
        if cat == Category::AutoMemory {
            ps.memory_file_count += 1;
            if depth == 2 && p.file_name().map_or(false, |n| n == "MEMORY.md") {
                ps.memory_md_lines = fs::read_to_string(p).ok().map(|s| s.lines().count() as u32);
            }
        }
        if cat == Category::Transcripts && is_jsonl {
            if let Some(m) = m {
                ps.last_active_ms = Some(ps.last_active_ms.map_or(m, |o| o.max(m)));
            }
            if depth == 1 {
                ps.session_count += 1;
            }
        }
        acc.add(cat, md.len(), m);
    });
    ps.size.total_bytes = acc.total();
    ps.size.by_category = acc.into_entries();
    (ps, bad)
}

pub fn scan(root: &Path, cache: &mut ScanCache, now: i64, progress: &mut dyn FnMut(ScanProgress)) -> ScanResult {
    let entries: Vec<fs::DirEntry> = fs::read_dir(root).map(|it| it.filter_map(Result::ok).collect()).unwrap_or_default();
    let total = entries.len();
    let mut global = Acc::default();
    let mut per_project = BTreeMap::new();
    let mut inaccessible = 0u64;
    let mut bytes_so_far = 0u64;

    for (i, de) in entries.into_iter().enumerate() {
        let name = de.file_name().to_string_lossy().into_owned();
        let path = de.path();
        let Ok(md) = fs::symlink_metadata(&path) else {
            inaccessible += 1;
            continue;
        };
        if md.file_type().is_symlink() {
            continue;
        }
        if name == "projects" && md.is_dir() {
            for pd in fs::read_dir(&path).into_iter().flatten().filter_map(Result::ok) {
                if !pd.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let enc = pd.file_name().to_string_lossy().into_owned();
                let (ps, bad) = scan_project_dir(&pd.path());
                inaccessible += bad;
                for e in &ps.size.by_category {
                    global.add_entry(e);
                }
                bytes_so_far += ps.size.total_bytes;
                per_project.insert(enc, ps);
            }
        } else {
            let dir_mtime = mtime_ms(&md).unwrap_or(0);
            let reused = cache
                .top_level
                .get(&name)
                .filter(|c| c.mtime_ms == dir_mtime && now - c.scanned_at_ms <= REUSE_WINDOW_MS)
                .cloned();
            let (entries_vec, bad) = match reused {
                Some(c) => (c.entries, c.inaccessible),
                None => {
                    let cat = classify_top_level(&name);
                    let mut acc = Acc::default();
                    let bad = if md.is_dir() {
                        walk_files(&path, |_, fmd| acc.add(cat.clone(), fmd.len(), mtime_ms(fmd)))
                    } else {
                        acc.add(cat.clone(), md.len(), mtime_ms(&md));
                        0
                    };
                    (acc.into_entries(), bad)
                }
            };
            cache.top_level.insert(
                name.clone(),
                TopLevelCache { mtime_ms: dir_mtime, scanned_at_ms: now, entries: entries_vec.clone(), inaccessible: bad },
            );
            inaccessible += bad;
            for e in &entries_vec {
                global.add_entry(e);
                bytes_so_far += e.bytes;
            }
        }
        progress(ScanProgress { done: i + 1, total, current: name, bytes_so_far });
    }

    let (cleanup_days, cleanup_days_source) = read_cleanup_days(root);
    let result = ScanResult {
        scanned_at_ms: now,
        root_total_bytes: bytes_so_far,
        global: global.into_entries(),
        per_project,
        inaccessible,
        cleanup_days,
        cleanup_days_source,
    };
    cache.last_result = Some(result.clone());
    result
}

pub fn simulate_cleanup(root: &Path, cleanup_days: u32, now: i64) -> CleanupPreview {
    let cutoff = now - cleanup_days as i64 * DAY_MS;
    let mut acc = Acc::default();
    let is_old = |md: &fs::Metadata| mtime_ms(md).map_or(false, |m| m < cutoff);

    for (dir, cat) in AUTO_CLEANUP_DIRS {
        let p = root.join(dir);
        if p.is_dir() {
            walk_files(&p, |_, md| {
                if is_old(md) {
                    acc.add(cat.clone(), md.len(), mtime_ms(md));
                }
            });
        }
    }
    if let Ok(rd) = fs::read_dir(root.join("projects")) {
        for pd in rd.filter_map(Result::ok).filter(|e| e.path().is_dir()) {
            let base = pd.path();
            walk_files(&base, |p, md| {
                let rel = p.strip_prefix(&base).unwrap_or(p);
                let is_jsonl = p.extension().map_or(false, |x| x == "jsonl");
                if classify_in_project(&first_component(rel), is_jsonl) == Category::Transcripts && is_old(md) {
                    acc.add(Category::Transcripts, md.len(), mtime_ms(md));
                }
            });
        }
    }
    CleanupPreview { cleanup_days, files: acc.files(), bytes: acc.total(), by_category: acc.into_entries() }
}
