//! 读取 ~/.claude.json 的 projects 键（项目发现的权威来源）。只读。
use crate::model::ConfigHint;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: String,
    pub hint: ConfigHint,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeConfig {
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置文件不存在：{0}")]
    Missing(PathBuf),
    #[error("读取配置文件失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("配置文件不是合法 JSON：{0}")]
    Json(#[from] serde_json::Error),
    #[error("配置文件顶层不是 JSON 对象")]
    NotObject,
}

fn hint_from(v: &Value) -> ConfigHint {
    let Some(obj) = v.as_object() else { return ConfigHint::default() };
    ConfigHint {
        last_session_id: obj.get("lastSessionId").and_then(Value::as_str).map(str::to_owned),
        trust_accepted: obj.get("hasTrustDialogAccepted").and_then(Value::as_bool).unwrap_or(false),
        last_total_input_tokens: obj.get("lastTotalInputTokens").and_then(Value::as_u64).unwrap_or(0),
        last_total_output_tokens: obj.get("lastTotalOutputTokens").and_then(Value::as_u64).unwrap_or(0),
    }
}

pub fn parse_config(text: &str) -> Result<ClaudeConfig, ConfigError> {
    let root: Value = serde_json::from_str(text)?;
    let obj = root.as_object().ok_or(ConfigError::NotObject)?;
    let entries = obj
        .get("projects")
        .and_then(Value::as_object)
        .map(|projects| {
            projects
                .iter()
                .map(|(k, v)| ConfigEntry { key: k.clone(), hint: hint_from(v) })
                .collect()
        })
        .unwrap_or_default();
    Ok(ClaudeConfig { entries })
}

pub fn load_config(path: &Path) -> Result<ClaudeConfig, ConfigError> {
    if !path.is_file() {
        return Err(ConfigError::Missing(path.to_path_buf()));
    }
    let text = std::fs::read_to_string(path)?;
    parse_config(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "numStartups": 3,
  "projects": {
    "C:/Users/u/Desktop/video": {
      "allowedTools": [],
      "hasTrustDialogAccepted": true,
      "lastSessionId": "04be759c-5ebb-4a0c-aafc-4ea42bfc5c35",
      "lastTotalInputTokens": 120,
      "lastTotalOutputTokens": 34,
      "someFutureField": {"x": 1}
    },
    "//192.168.1.10/share/camera": {},
    "Z:/weird": "not-an-object"
  }
}"#;

    #[test]
    fn parses_entries_in_file_order_with_hints() {
        let c = parse_config(SAMPLE).unwrap();
        assert_eq!(c.entries.len(), 3);
        assert_eq!(c.entries[0].key, "C:/Users/u/Desktop/video");
        assert!(c.entries[0].hint.trust_accepted);
        assert_eq!(c.entries[0].hint.last_session_id.as_deref(), Some("04be759c-5ebb-4a0c-aafc-4ea42bfc5c35"));
        assert_eq!(c.entries[0].hint.last_total_input_tokens, 120);
        assert_eq!(c.entries[1].key, "//192.168.1.10/share/camera");
        assert_eq!(c.entries[1].hint, ConfigHint::default());
        // 值不是对象：保留 key，hint 取默认，不报错
        assert_eq!(c.entries[2].key, "Z:/weird");
    }

    #[test]
    fn missing_projects_key_is_empty() {
        let c = parse_config(r#"{"numStartups": 1}"#).unwrap();
        assert!(c.entries.is_empty());
    }

    #[test]
    fn invalid_json_is_error() {
        assert!(matches!(parse_config("{ not json"), Err(ConfigError::Json(_))));
    }

    #[test]
    fn missing_file_is_missing_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(".claude.json");
        assert!(matches!(load_config(&p), Err(ConfigError::Missing(_))));
    }
}
