//! 全局 token 口径：直接读 stats-cache.json（/usage 的聚合），不自行累加（设计 5.3）。
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const KNOWN_STATS_VERSION: u64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelTotals {
    pub model: String,
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalStats {
    pub available: bool,
    pub reason: Option<String>,
    pub version: Option<u64>,
    pub last_computed_date: Option<String>,
    pub first_session_date: Option<String>,
    pub total_sessions: u64,
    pub total_messages: u64,
    pub models: Vec<ModelTotals>,
}

fn unavailable(reason: impl Into<String>) -> GlobalStats {
    GlobalStats { available: false, reason: Some(reason.into()), ..Default::default() }
}

pub fn parse_stats(text: &str) -> GlobalStats {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return unavailable("stats-cache.json 不是合法 JSON") };
    let version = v.get("version").and_then(Value::as_u64);
    if let Some(ver) = version.filter(|ver| *ver > KNOWN_STATS_VERSION) {
        let mut s = unavailable(format!("格式未识别（version {ver}），本工具已知版本为 {KNOWN_STATS_VERSION}"));
        s.version = Some(ver);
        return s;
    }
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let n = |k: &str| v.get(k).and_then(Value::as_u64).unwrap_or(0);
    let mut models: Vec<ModelTotals> = v
        .get("modelUsage")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(name, u)| {
                    let g = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
                    ModelTotals {
                        model: name.clone(),
                        input: g("inputTokens"),
                        output: g("outputTokens"),
                        cache_creation: g("cacheCreationInputTokens"),
                        cache_read: g("cacheReadInputTokens"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    models.sort_by_key(|m| std::cmp::Reverse(m.input + m.output + m.cache_creation + m.cache_read));
    GlobalStats {
        available: true,
        reason: None,
        version,
        last_computed_date: s("lastComputedDate"),
        first_session_date: s("firstSessionDate"),
        total_sessions: n("totalSessions"),
        total_messages: n("totalMessages"),
        models,
    }
}

pub fn read_global_stats(root: &Path) -> GlobalStats {
    match std::fs::read_to_string(root.join("stats-cache.json")) {
        Ok(t) => parse_stats(&t),
        Err(e) => unavailable(format!("无法读取 stats-cache.json：{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "version": 5, "lastComputedDate": "2026-09-02", "firstSessionDate": "2026-03-03T08:17:32.654Z",
      "totalSessions": 123, "totalMessages": 44507,
      "modelUsage": {
        "claude-opus-4-6": {"inputTokens": 383588, "outputTokens": 4454584, "cacheReadInputTokens": 925829370, "cacheCreationInputTokens": 38983905, "costUSD": 0},
        "claude-haiku-4-5-20251001": {"inputTokens": 1, "outputTokens": 2}
      },
      "dailyActivity": [], "hourCounts": {}
    }"#;

    #[test]
    fn parses_known_version() {
        let s = parse_stats(SAMPLE);
        assert!(s.available);
        assert_eq!(s.version, Some(5));
        assert_eq!(s.last_computed_date.as_deref(), Some("2026-09-02"));
        assert_eq!(s.total_sessions, 123);
        assert_eq!(s.models.len(), 2);
        let opus = s.models.iter().find(|m| m.model == "claude-opus-4-6").unwrap();
        assert_eq!((opus.input, opus.output, opus.cache_creation, opus.cache_read), (383588, 4454584, 38983905, 925829370));
        let haiku = s.models.iter().find(|m| m.model.starts_with("claude-haiku")).unwrap();
        assert_eq!(haiku.cache_read, 0);
    }

    #[test]
    fn models_sorted_by_total_tokens_desc() {
        let s = parse_stats(SAMPLE);
        assert_eq!(s.models[0].model, "claude-opus-4-6");
    }

    #[test]
    fn newer_version_is_unavailable_with_reason() {
        let s = parse_stats(r#"{"version": 9, "totalSessions": 1}"#);
        assert!(!s.available);
        assert!(s.reason.as_deref().unwrap().contains("version 9"));
        assert_eq!(s.version, Some(9));
    }

    #[test]
    fn garbage_is_unavailable() {
        assert!(!parse_stats("nope").available);
    }

    #[test]
    fn missing_file_is_unavailable() {
        let d = tempfile::tempdir().unwrap();
        let s = read_global_stats(d.path());
        assert!(!s.available);
        assert!(s.reason.is_some());
    }
}
