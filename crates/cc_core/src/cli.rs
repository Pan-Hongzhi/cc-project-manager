//! 探测 claude 可执行文件（设计 6.5）。M1 只用于诊断页；M2 purge 复用 CommandRunner。
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> std::io::Result<CommandOutput>;
}

pub struct StdRunner;

impl CommandRunner for StdRunner {
    fn run(&self, program: &str, args: &[&str]) -> std::io::Result<CommandOutput> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd.output()?;
        Ok(CommandOutput {
            status: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CliKind {
    NpmCmd,
    NativeExe,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliInfo {
    pub path: String,
    pub version: Option<String>,
    pub kind: CliKind,
}

fn kind_of(path: &str) -> CliKind {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".cmd") {
        CliKind::NpmCmd
    } else if lower.ends_with(".exe") {
        CliKind::NativeExe
    } else {
        CliKind::Other
    }
}

pub fn version_args(path: &str) -> (String, Vec<String>) {
    match kind_of(path) {
        CliKind::NpmCmd => ("cmd.exe".into(), vec!["/c".into(), path.into(), "--version".into()]),
        _ => (path.into(), vec!["--version".into()]),
    }
}

fn read_version(runner: &dyn CommandRunner, path: &str) -> Option<String> {
    let (prog, args) = version_args(path);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = runner.run(&prog, &arg_refs).ok()?;
    let line = out.stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.to_string())
}

pub fn detect_cli(runner: &dyn CommandRunner, user_profile: &Path, exists: &dyn Fn(&Path) -> bool) -> Option<CliInfo> {
    let from_where = runner.run("where.exe", &["claude"]).ok().filter(|o| o.status == 0).and_then(|o| {
        o.stdout
            .lines()
            .map(str::trim)
            .find(|l| {
                let lower = l.to_ascii_lowercase();
                lower.ends_with(".cmd") || lower.ends_with(".exe")
            })
            .map(str::to_owned)
    });
    let path = from_where.or_else(|| {
        let local = user_profile.join(".local").join("bin").join("claude.exe");
        exists(&local).then(|| local.to_string_lossy().into_owned())
    })?;
    let version = read_version(runner, &path);
    Some(CliInfo { kind: kind_of(&path), path, version })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::Path;

    struct FakeRunner {
        where_out: Option<String>,
        calls: RefCell<Vec<(String, Vec<String>)>>,
    }
    impl CommandRunner for FakeRunner {
        fn run(&self, program: &str, args: &[&str]) -> std::io::Result<CommandOutput> {
            self.calls.borrow_mut().push((program.to_string(), args.iter().map(|s| s.to_string()).collect()));
            if program.eq_ignore_ascii_case("where.exe") {
                return match &self.where_out {
                    Some(o) => Ok(CommandOutput { status: 0, stdout: o.clone(), stderr: String::new() }),
                    None => Ok(CommandOutput { status: 1, stdout: String::new(), stderr: "INFO: not found".into() }),
                };
            }
            Ok(CommandOutput { status: 0, stdout: "2.1.278 (Claude Code)\n".into(), stderr: String::new() })
        }
    }

    #[test]
    fn picks_first_cmd_or_exe_from_where_and_reads_version_via_cmd() {
        let r = FakeRunner {
            where_out: Some("C:\\Users\\u\\AppData\\Roaming\\npm\\claude\r\nC:\\Users\\u\\AppData\\Roaming\\npm\\claude.cmd\r\n".into()),
            calls: RefCell::new(vec![]),
        };
        let info = detect_cli(&r, Path::new(r"C:\Users\u"), &|_| false).unwrap();
        assert_eq!(info.path, r"C:\Users\u\AppData\Roaming\npm\claude.cmd");
        assert_eq!(info.kind, CliKind::NpmCmd);
        assert_eq!(info.version.as_deref(), Some("2.1.278 (Claude Code)"));
        let calls = r.calls.borrow();
        assert_eq!(calls[1].0, "cmd.exe");
        assert_eq!(calls[1].1, vec!["/c", r"C:\Users\u\AppData\Roaming\npm\claude.cmd", "--version"]);
    }

    #[test]
    fn falls_back_to_local_bin_exe() {
        let r = FakeRunner { where_out: None, calls: RefCell::new(vec![]) };
        let expected = Path::new(r"C:\Users\u\.local\bin\claude.exe");
        let info = detect_cli(&r, Path::new(r"C:\Users\u"), &|p| p == expected).unwrap();
        assert_eq!(info.kind, CliKind::NativeExe);
        assert_eq!(info.path, expected.to_string_lossy());
    }

    #[test]
    fn none_when_nothing_found() {
        let r = FakeRunner { where_out: None, calls: RefCell::new(vec![]) };
        assert!(detect_cli(&r, Path::new(r"C:\Users\u"), &|_| false).is_none());
    }
}
