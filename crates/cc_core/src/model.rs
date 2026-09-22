//! 核心库与前端共享的数据模型。字段名即 JSON 字段名（snake_case）。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Normal,
    ConfigOnly,
    Orphan,
    Unreachable,
    Unowned,
    LegacyEncoded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verified {
    Alive,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunningSession {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub started_at_ms: i64,
    pub status: String,
    pub verified: Verified,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigHint {
    pub last_session_id: Option<String>,
    pub trust_accepted: bool,
    pub last_total_input_tokens: u64,
    pub last_total_output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum Category {
    Transcripts,
    AutoMemory,
    FileHistory,
    PasteCache,
    Uploads,
    Debug,
    Plans,
    Tasks,
    SessionEnv,
    HistoryLog,
    StatsCache,
    Legacy(String),
    Protected(String),
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Retention {
    AutoCleanup { days: u32 },
    Permanent,
    MemoryRule,
    LegacyRemoved,
    Protected,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryMeta {
    pub category: Category,
    pub retention: Retention,
    pub deletable_by_tool: bool,
    pub consequence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryEntry {
    pub category: Category,
    pub bytes: u64,
    pub file_count: u64,
    pub oldest_mtime_ms: Option<i64>,
    pub newest_mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SizeBreakdown {
    pub total_bytes: u64,
    pub by_category: Vec<CategoryEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageStat {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    pub message_count: u64,
    pub session_count: u64,
    pub skipped_lines: u64,
}

impl UsageStat {
    pub fn add(&mut self, o: &UsageStat) {
        self.input += o.input;
        self.output += o.output;
        self.cache_creation += o.cache_creation;
        self.cache_read += o.cache_read;
        self.message_count += o.message_count;
        self.session_count += o.session_count;
        self.skipped_lines += o.skipped_lines;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySummary {
    pub file_count: u32,
    pub memory_md_lines: Option<u32>,
    pub user_claude_md: bool,
    pub user_claude_local_md: bool,
    pub user_rules_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    /// 稳定标识：有配置项时 = 配置 key（正斜杠真实路径）；无主目录 = "dir:<编码目录名>"
    pub id: String,
    pub real_path: Option<String>,
    pub encoded_dir: Option<String>,
    pub state: ProjectState,
    /// LegacyEncoded 时：按旧规则匹配到的配置 key
    pub legacy_of: Option<String>,
    pub running: Option<RunningSession>,
    pub last_active_ms: Option<i64>,
    pub config_hint: Option<ConfigHint>,
    pub size: Option<SizeBreakdown>,
    pub usage: Option<UsageStat>,
    pub memory_summary: Option<MemorySummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodingSelfCheck {
    pub total_entries: usize,
    pub matched_by_current_rule: usize,
    pub matched_by_legacy_rule: usize,
    /// 无法被任何规则解释的 projects/ 子目录名（= Unowned 目录）
    pub unmatched: Vec<String>,
    pub migration_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_json_shape() {
        assert_eq!(serde_json::to_string(&Category::Transcripts).unwrap(), r#"{"kind":"transcripts"}"#);
        assert_eq!(
            serde_json::to_string(&Category::Legacy("todos".into())).unwrap(),
            r#"{"kind":"legacy","name":"todos"}"#
        );
        let back: Category = serde_json::from_str(r#"{"kind":"unknown","name":"foo"}"#).unwrap();
        assert_eq!(back, Category::Unknown("foo".into()));
    }

    #[test]
    fn retention_json_shape() {
        assert_eq!(
            serde_json::to_string(&Retention::AutoCleanup { days: 30 }).unwrap(),
            r#"{"kind":"auto_cleanup","days":30}"#
        );
        assert_eq!(serde_json::to_string(&Retention::Permanent).unwrap(), r#"{"kind":"permanent"}"#);
    }

    #[test]
    fn project_state_is_snake_case() {
        assert_eq!(serde_json::to_string(&ProjectState::ConfigOnly).unwrap(), r#""config_only""#);
    }

    #[test]
    fn usage_stat_add() {
        let mut a = UsageStat { input: 1, output: 2, cache_creation: 3, cache_read: 4, message_count: 1, session_count: 0, skipped_lines: 0 };
        a.add(&UsageStat { input: 10, output: 20, cache_creation: 30, cache_read: 40, message_count: 2, session_count: 1, skipped_lines: 5 });
        assert_eq!((a.input, a.output, a.cache_creation, a.cache_read), (11, 22, 33, 44));
        assert_eq!((a.message_count, a.session_count, a.skipped_lines), (3, 1, 5));
    }
}
