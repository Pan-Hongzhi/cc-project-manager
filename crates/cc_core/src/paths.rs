use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootSource {
    EnvVar,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataRoot {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub source: RootSource,
}

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("环境变量 USERPROFILE 不存在，无法定位数据根目录")]
    NoUserProfile,
}

pub fn resolve_root_from(
    config_dir_env: Option<&str>,
    user_profile: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> DataRoot {
    let home_config = user_profile.join(".claude.json");
    match config_dir_env.map(str::trim).filter(|s| !s.is_empty()) {
        Some(dir) => {
            let root = PathBuf::from(dir);
            let inside = root.join(".claude.json");
            let config_file = if exists(&inside) { inside } else { home_config };
            DataRoot { root, config_file, source: RootSource::EnvVar }
        }
        None => DataRoot {
            root: user_profile.join(".claude"),
            config_file: home_config,
            source: RootSource::Default,
        },
    }
}

pub fn resolve_root() -> Result<DataRoot, PathsError> {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or(PathsError::NoUserProfile)?;
    let env = std::env::var("CLAUDE_CONFIG_DIR").ok();
    Ok(resolve_root_from(env.as_deref(), &profile, &|p| p.is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_root_when_env_missing() {
        let r = resolve_root_from(None, Path::new(r"C:\Users\u"), &|_| false);
        assert_eq!(r.root, Path::new(r"C:\Users\u\.claude"));
        assert_eq!(r.config_file, Path::new(r"C:\Users\u\.claude.json"));
        assert_eq!(r.source, RootSource::Default);
    }

    #[test]
    fn blank_env_is_treated_as_missing() {
        let r = resolve_root_from(Some("   "), Path::new(r"C:\Users\u"), &|_| false);
        assert_eq!(r.source, RootSource::Default);
    }

    #[test]
    fn env_root_prefers_config_json_inside_it() {
        let inside = Path::new(r"D:\cc\.claude.json");
        let r = resolve_root_from(Some(r"D:\cc"), Path::new(r"C:\Users\u"), &|p| p == inside);
        assert_eq!(r.root, Path::new(r"D:\cc"));
        assert_eq!(r.config_file, inside);
        assert_eq!(r.source, RootSource::EnvVar);
    }

    #[test]
    fn env_root_falls_back_to_home_config_json() {
        let r = resolve_root_from(Some(r"D:\cc"), Path::new(r"C:\Users\u"), &|_| false);
        assert_eq!(r.config_file, Path::new(r"C:\Users\u\.claude.json"));
    }
}
