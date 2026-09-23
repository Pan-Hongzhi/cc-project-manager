//! 组装 Overview：把各只读模块的结果拼成前端需要的一份数据。
use crate::cache::{load_json, save_json};
use crate::cli::{detect_cli, CliInfo, CommandRunner};
use crate::config::load_config;
use crate::discovery::{discover, list_encoded_dirs, probe_real_path};
use crate::model::{CategoryMeta, EncodingSelfCheck, MemorySummary, Project, ProjectState, RunningSession};
use crate::paths::DataRoot;
use crate::scan::{
    category_meta, list_all_categories, now_ms, scan, simulate_cleanup, CleanupPreview, ScanCache, ScanProgress,
    ScanResult, SCAN_SCHEMA_VERSION,
};
use crate::sessions::{assign_sessions, list_sessions, ProcessProbe};
use crate::stats::{read_global_stats, GlobalStats};
use crate::usage::{update_project_usage, UsageCache, USAGE_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PATH_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub root: DataRoot,
    pub config_error: Option<String>,
    pub projects: Vec<Project>,
    pub self_check: EncodingSelfCheck,
    pub sessions: Vec<RunningSession>,
    pub scan: Option<ScanResult>,
    pub cleanup_preview: Option<CleanupPreview>,
    pub category_meta: Vec<CategoryMeta>,
    pub stats: GlobalStats,
    pub cli: Option<CliInfo>,
    pub tool_data_dir: String,
    pub generated_at_ms: i64,
    pub scan_is_cached: bool,
}

pub struct Engine {
    root: DataRoot,
    data_dir: PathBuf,
    scan_cache: ScanCache,
    usage_cache: UsageCache,
}

impl Engine {
    pub fn new(root: DataRoot, data_dir: PathBuf) -> Engine {
        let scan_cache = load_json(&data_dir.join("cache").join("scan.json"), SCAN_SCHEMA_VERSION).unwrap_or_default();
        let usage_cache = load_json(&data_dir.join("cache").join("usage.json"), USAGE_SCHEMA_VERSION).unwrap_or_default();
        Engine { root, data_dir, scan_cache, usage_cache }
    }

    pub fn root(&self) -> &DataRoot {
        &self.root
    }

    fn discover_projects(&self, probe: &dyn ProcessProbe) -> (Vec<Project>, EncodingSelfCheck, Vec<RunningSession>, Option<String>) {
        let (config, config_error) = match load_config(&self.root.config_file) {
            Ok(c) => (Some(c), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let dirs = list_encoded_dirs(&self.root.root);
        let (mut projects, check) = discover(config.as_ref(), &dirs, &|p| probe_real_path(p, PATH_PROBE_TIMEOUT));
        let sessions = list_sessions(&self.root.root, probe);
        // 每个会话只归属到路径最深的项目，避免父目录项目（如用户主目录）把所有子项目算成运行中
        let real_paths: Vec<&str> = projects.iter().filter_map(|p| p.real_path.as_deref()).collect();
        let owners: std::collections::BTreeMap<String, RunningSession> = assign_sessions(&sessions, &real_paths)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        for p in &mut projects {
            if let Some(real) = &p.real_path {
                p.running = owners.get(real.as_str()).cloned();
            }
        }
        (projects, check, sessions, config_error)
    }

    fn attach_scan(&self, projects: &mut [Project], scan: &ScanResult) {
        for p in projects.iter_mut() {
            let Some(enc) = &p.encoded_dir else { continue };
            let Some(ps) = scan.per_project.get(enc) else { continue };
            p.size = Some(ps.size.clone());
            p.last_active_ms = ps.last_active_ms;
            let mut ms = MemorySummary { file_count: ps.memory_file_count, memory_md_lines: ps.memory_md_lines, ..Default::default() };
            if p.state == ProjectState::Normal {
                if let Some(real) = &p.real_path {
                    let rp = Path::new(real);
                    ms.user_claude_md = rp.join("CLAUDE.md").is_file();
                    ms.user_claude_local_md = rp.join("CLAUDE.local.md").is_file();
                    ms.user_rules_dir = rp.join(".claude").join("rules").is_dir();
                }
            }
            p.memory_summary = Some(ms);
        }
    }

    fn assemble(&self, projects: Vec<Project>, check: EncodingSelfCheck, sessions: Vec<RunningSession>, config_error: Option<String>,
                scan: Option<ScanResult>, cleanup_preview: Option<CleanupPreview>, runner: &dyn CommandRunner, scan_is_cached: bool) -> Overview {
        let cleanup_days = scan.as_ref().map(|s| s.cleanup_days).unwrap_or(30);
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        Overview {
            root: self.root.clone(),
            config_error,
            projects,
            self_check: check,
            sessions,
            scan,
            cleanup_preview,
            category_meta: list_all_categories().iter().map(|c| category_meta(c, cleanup_days)).collect(),
            stats: read_global_stats(&self.root.root),
            cli: detect_cli(runner, &profile, &|p| p.is_file()),
            tool_data_dir: self.data_dir.to_string_lossy().into_owned(),
            generated_at_ms: now_ms(),
            scan_is_cached,
        }
    }

    /// 不遍历磁盘：发现 + 会话 + stats + CLI + 上次扫描缓存。
    pub fn quick_overview(&self, probe: &dyn ProcessProbe, runner: &dyn CommandRunner) -> Overview {
        let (mut projects, check, sessions, config_error) = self.discover_projects(probe);
        let scan = self.scan_cache.last_result.clone();
        if let Some(s) = &scan {
            self.attach_scan(&mut projects, s);
            for p in &mut projects {
                if let Some(enc) = &p.encoded_dir {
                    // 末尾加分隔符：避免 C--a 的前缀匹配到兄弟目录 C--ab 的缓存条目
                    let prefix = format!(
                        "{}{}",
                        self.root.root.join("projects").join(enc).to_string_lossy(),
                        std::path::MAIN_SEPARATOR
                    );
                    let mut total = crate::model::UsageStat::default();
                    for (k, e) in &self.usage_cache.files {
                        if k.starts_with(&prefix) {
                            total.add(&e.stat);
                        }
                    }
                    total.session_count = s.per_project.get(enc).map(|ps| ps.session_count).unwrap_or(0);
                    p.usage = Some(total);
                }
            }
        }
        let preview = self.scan_cache.last_cleanup_preview.clone();
        self.assemble(projects, check, sessions, config_error, scan, preview, runner, true)
    }

    /// 全量刷新：扫描 + usage + 清扫模拟，并保存缓存。
    pub fn full_refresh(&mut self, probe: &dyn ProcessProbe, runner: &dyn CommandRunner, progress: &mut dyn FnMut(ScanProgress)) -> Overview {
        let (mut projects, check, sessions, config_error) = self.discover_projects(probe);
        let now = now_ms();
        let scan_result = scan(&self.root.root, &mut self.scan_cache, now, progress);
        self.attach_scan(&mut projects, &scan_result);
        for p in &mut projects {
            if let Some(enc) = &p.encoded_dir {
                let dir = self.root.root.join("projects").join(enc);
                p.usage = Some(update_project_usage(&dir, &mut self.usage_cache));
            }
        }
        let preview = simulate_cleanup(&self.root.root, scan_result.cleanup_days, now);
        self.scan_cache.last_cleanup_preview = Some(preview.clone());
        let _ = save_json(&self.data_dir.join("cache").join("scan.json"), &self.scan_cache);
        let _ = save_json(&self.data_dir.join("cache").join("usage.json"), &self.usage_cache);
        self.assemble(projects, check, sessions, config_error, Some(scan_result), Some(preview), runner, false)
    }
}
