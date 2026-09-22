//! 分项目 token 统计：逐行解析转录中的 assistant.usage，按 message.id 去重（设计 6.4）。
use crate::model::UsageStat;
use crate::scan::mtime_ms;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const USAGE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageFileEntry {
    pub mtime_ms: i64,
    pub size: u64,
    pub stat: UsageStat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageCache {
    pub schema_version: u32,
    /// key = 转录文件绝对路径（字符串）
    pub files: BTreeMap<String, UsageFileEntry>,
}

impl Default for UsageCache {
    fn default() -> Self {
        UsageCache { schema_version: USAGE_SCHEMA_VERSION, files: BTreeMap::new() }
    }
}

pub fn parse_transcript(path: &Path) -> std::io::Result<UsageStat> {
    let reader = BufReader::new(fs::File::open(path)?);
    let mut by_id: BTreeMap<String, [u64; 4]> = BTreeMap::new();
    let mut skipped = 0u64;
    let mut fallback_seq = 0u64;

    for line in reader.split(b'\n') {
        let Ok(line) = line else {
            skipped += 1;
            continue;
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(&line) else {
            skipped += 1;
            continue;
        };
        if v.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(msg) = v.get("message") else { continue };
        let Some(usage) = msg.get("usage").filter(|u| u.is_object()) else { continue };
        let id = msg
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| v.get("uuid").and_then(Value::as_str))
            .map(str::to_owned)
            .unwrap_or_else(|| {
                fallback_seq += 1;
                format!("__noid_{fallback_seq}")
            });
        let g = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
        by_id.insert(
            id,
            [g("input_tokens"), g("output_tokens"), g("cache_creation_input_tokens"), g("cache_read_input_tokens")],
        );
    }

    let mut s = UsageStat::default();
    for [i, o, cc, cr] in by_id.values() {
        s.input += i;
        s.output += o;
        s.cache_creation += cc;
        s.cache_read += cr;
    }
    s.message_count = by_id.len() as u64;
    s.skipped_lines = skipped;
    Ok(s)
}

fn transcript_files(project_dir: &Path) -> Vec<(PathBuf, bool)> {
    WalkDir::new(project_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |x| x == "jsonl"))
        .filter(|e| {
            let rel = e.path().strip_prefix(project_dir).unwrap_or(e.path());
            rel.components().next().map_or(true, |c| c.as_os_str() != "memory")
        })
        .map(|e| {
            let is_top = e.depth() == 1;
            (e.into_path(), is_top)
        })
        .collect()
}

pub fn update_project_usage(project_dir: &Path, cache: &mut UsageCache) -> UsageStat {
    let files = transcript_files(project_dir);
    let prefix = project_dir.to_string_lossy().to_string();
    let present: std::collections::BTreeSet<String> = files.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect();
    cache.files.retain(|k, _| !k.starts_with(&prefix) || present.contains(k));

    let mut total = UsageStat::default();
    for (path, is_top) in files {
        let key = path.to_string_lossy().to_string();
        let Ok(md) = fs::metadata(&path) else { continue };
        let (mtime, size) = (mtime_ms(&md).unwrap_or(0), md.len());
        let reuse = cache.files.get(&key).filter(|e| e.mtime_ms == mtime && e.size == size).map(|e| e.stat.clone());
        let stat = match reuse {
            Some(s) => s,
            None => match parse_transcript(&path) {
                Ok(s) => {
                    cache.files.insert(key, UsageFileEntry { mtime_ms: mtime, size, stat: s.clone() });
                    s
                }
                Err(_) => continue, // 正在被 CC 写入或无权限：本次跳过，缓存不更新
            },
        };
        total.add(&stat);
        if is_top {
            total.session_count += 1;
        }
    }
    total
}
