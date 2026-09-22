#![allow(dead_code)]
use cc_core::paths::{DataRoot, RootSource};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// 在临时目录里伪造一个 .claude 数据根 + .claude.json。绝不指向真实目录。
pub struct FakeRoot {
    pub dir: TempDir,
    pub root: DataRoot,
    projects: Map<String, Value>,
}

impl FakeRoot {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root_path = dir.path().join(".claude");
        fs::create_dir_all(root_path.join("projects")).unwrap();
        fs::create_dir_all(root_path.join("sessions")).unwrap();
        let root = DataRoot {
            root: root_path,
            config_file: dir.path().join(".claude.json"),
            source: RootSource::Default,
        };
        let me = FakeRoot { dir, root, projects: Map::new() };
        me.write_config();
        me
    }

    pub fn config_entry(&mut self, key: &str, value: Value) -> &mut Self {
        self.projects.insert(key.to_string(), value);
        self.write_config();
        self
    }

    pub fn write_config(&self) {
        let v = json!({ "numStartups": 1, "projects": Value::Object(self.projects.clone()) });
        fs::write(&self.root.config_file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }

    pub fn project_dir(&self, encoded: &str) -> PathBuf {
        let p = self.root.root.join("projects").join(encoded);
        fs::create_dir_all(&p).unwrap();
        p
    }

    pub fn project_file(&self, encoded: &str, rel: &str, content: &str) -> PathBuf {
        let p = self.project_dir(encoded).join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        p
    }

    pub fn top_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.root.root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        p
    }

    pub fn session(&self, pid: u32, cwd: &str, proc_start: Option<u64>) -> PathBuf {
        let mut v = json!({
            "pid": pid, "sessionId": format!("sess-{pid}"), "cwd": cwd,
            "startedAt": 1789901109617u64, "status": "idle", "version": "2.1.278"
        });
        if let Some(ps) = proc_start {
            v["procStart"] = Value::String(ps.to_string());
        }
        self.top_file(&format!("sessions/{pid}.json"), v.to_string().as_bytes())
    }

    pub fn path(&self) -> &Path {
        &self.root.root
    }
}
