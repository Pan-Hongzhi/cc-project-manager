//! 项目发现：配置项 × projects/ 目录 → 状态判定 + 编码自校验。
use crate::config::ClaudeConfig;
use crate::encoding::{encode_path, encode_path_legacy};
use crate::model::{EncodingSelfCheck, Project, ProjectState};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathProbe {
    Exists,
    Missing,
    Unreachable,
}

/// 带超时的存在性探测：网络盘/UNC 离线时 metadata() 可能阻塞数十秒。
pub fn probe_real_path(real: &str, timeout: Duration) -> PathProbe {
    let (tx, rx) = mpsc::channel();
    let p = real.to_string();
    std::thread::spawn(move || {
        let _ = tx.send(Path::new(&p).is_dir());
    });
    match rx.recv_timeout(timeout) {
        Ok(true) => PathProbe::Exists,
        Ok(false) => PathProbe::Missing,
        Err(_) => PathProbe::Unreachable,
    }
}

pub fn list_encoded_dirs(root: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root.join("projects")) else { return Vec::new() };
    rd.filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

fn blank_project(id: String, real_path: Option<String>, encoded_dir: Option<String>, state: ProjectState) -> Project {
    Project {
        id,
        real_path,
        encoded_dir,
        state,
        legacy_of: None,
        running: None,
        last_active_ms: None,
        config_hint: None,
        size: None,
        usage: None,
        memory_summary: None,
    }
}

pub fn discover(
    config: Option<&ClaudeConfig>,
    encoded_dirs: &[String],
    probe: &dyn Fn(&str) -> PathProbe,
) -> (Vec<Project>, EncodingSelfCheck) {
    let dir_set: BTreeSet<&str> = encoded_dirs.iter().map(String::as_str).collect();
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    let mut legacy_claims: BTreeMap<String, String> = BTreeMap::new();
    let mut projects = Vec::new();
    let mut check = EncodingSelfCheck::default();

    if let Some(cfg) = config {
        check.total_entries = cfg.entries.len();
        for e in &cfg.entries {
            let current = encode_path(&e.key);
            let mut encoded_dir = None;
            if dir_set.contains(current.as_str()) {
                claimed.insert(current.clone());
                encoded_dir = Some(current.clone());
                check.matched_by_current_rule += 1;
            }
            let legacy = encode_path_legacy(&e.key);
            if legacy != current && dir_set.contains(legacy.as_str()) && !legacy_claims.contains_key(&legacy) {
                legacy_claims.insert(legacy, e.key.clone());
                check.matched_by_legacy_rule += 1;
            }
            let state = match probe(&e.key) {
                PathProbe::Missing => ProjectState::Orphan,
                PathProbe::Unreachable => ProjectState::Unreachable,
                PathProbe::Exists if encoded_dir.is_some() => ProjectState::Normal,
                PathProbe::Exists => ProjectState::ConfigOnly,
            };
            let mut p = blank_project(e.key.clone(), Some(e.key.clone()), encoded_dir, state);
            p.config_hint = Some(e.hint.clone());
            projects.push(p);
        }
    }

    for d in encoded_dirs {
        if claimed.contains(d) {
            continue;
        }
        let mut p = blank_project(format!("dir:{d}"), None, Some(d.clone()), ProjectState::Unowned);
        if let Some(owner) = legacy_claims.get(d) {
            p.state = ProjectState::LegacyEncoded;
            p.legacy_of = Some(owner.clone());
        } else {
            check.unmatched.push(d.clone());
        }
        projects.push(p);
    }
    check.unmatched.sort();
    check.migration_enabled = config.is_some() && check.unmatched.is_empty();
    (projects, check)
}
