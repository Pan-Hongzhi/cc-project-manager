mod common;
use cc_core::config::load_config;
use cc_core::discovery::{discover, list_encoded_dirs, PathProbe};
use cc_core::model::ProjectState;
use common::FakeRoot;
use serde_json::json;

fn probe_all_exist(_: &str) -> PathProbe {
    PathProbe::Exists
}

#[test]
fn classifies_five_states() {
    let mut f = FakeRoot::new();
    f.config_entry("C:/proj/normal", json!({"hasTrustDialogAccepted": true}))
        .config_entry("C:/proj/config_only", json!({}))
        .config_entry("C:/proj/orphan", json!({}))
        .config_entry("Z:/net/unreachable", json!({}))
        .config_entry("C:/proj/legacy_owner", json!({}));
    f.project_dir("C--proj-normal");
    f.project_dir("C--proj-orphan");
    f.project_dir("C--proj-legacy_owner"); // 旧规则保留了下划线 → LegacyEncoded
    f.project_dir("C--nobody-knows"); // 无主

    let cfg = load_config(&f.root.config_file).unwrap();
    let dirs = list_encoded_dirs(f.path());
    let probe = |p: &str| match p {
        "C:/proj/orphan" => PathProbe::Missing,
        "Z:/net/unreachable" => PathProbe::Unreachable,
        _ => PathProbe::Exists,
    };
    let (projects, check) = discover(Some(&cfg), &dirs, &probe);

    let state_of = |id: &str| projects.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("no {id}")).state;
    assert_eq!(state_of("C:/proj/normal"), ProjectState::Normal);
    assert_eq!(state_of("C:/proj/config_only"), ProjectState::ConfigOnly);
    assert_eq!(state_of("C:/proj/orphan"), ProjectState::Orphan);
    assert_eq!(state_of("Z:/net/unreachable"), ProjectState::Unreachable);
    assert_eq!(state_of("C:/proj/legacy_owner"), ProjectState::ConfigOnly);
    assert_eq!(state_of("dir:C--proj-legacy_owner"), ProjectState::LegacyEncoded);
    assert_eq!(state_of("dir:C--nobody-knows"), ProjectState::Unowned);

    let legacy = projects.iter().find(|p| p.id == "dir:C--proj-legacy_owner").unwrap();
    assert_eq!(legacy.legacy_of.as_deref(), Some("C:/proj/legacy_owner"));
    let normal = projects.iter().find(|p| p.id == "C:/proj/normal").unwrap();
    assert_eq!(normal.encoded_dir.as_deref(), Some("C--proj-normal"));
    assert!(normal.config_hint.as_ref().unwrap().trust_accepted);
    let orphan = projects.iter().find(|p| p.id == "C:/proj/orphan").unwrap();
    assert_eq!(orphan.encoded_dir.as_deref(), Some("C--proj-orphan"), "孤儿也要保留数据目录引用");

    assert_eq!(check.total_entries, 5);
    assert_eq!(check.matched_by_current_rule, 2);
    assert_eq!(check.matched_by_legacy_rule, 1);
    assert_eq!(check.unmatched, vec!["C--nobody-knows".to_string()]);
    assert!(!check.migration_enabled);
}

#[test]
fn self_check_passes_when_every_dir_is_explained() {
    let mut f = FakeRoot::new();
    f.config_entry("C:/a", json!({})).config_entry("C:/b_c", json!({}));
    f.project_dir("C--a");
    f.project_dir("C--b_c"); // legacy-only
    let cfg = load_config(&f.root.config_file).unwrap();
    let (_, check) = discover(Some(&cfg), &list_encoded_dirs(f.path()), &probe_all_exist);
    assert!(check.migration_enabled);
    assert!(check.unmatched.is_empty());
}

#[test]
fn without_config_everything_is_unowned() {
    let f = FakeRoot::new();
    f.project_dir("C--x");
    let (projects, check) = discover(None, &list_encoded_dirs(f.path()), &probe_all_exist);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].state, ProjectState::Unowned);
    assert_eq!(check.total_entries, 0);
    assert!(!check.migration_enabled);
}

#[test]
fn list_encoded_dirs_ignores_files_and_missing_projects_dir() {
    let f = FakeRoot::new();
    f.project_dir("C--a");
    f.top_file("projects/stray.txt", b"x");
    let mut dirs = list_encoded_dirs(f.path());
    dirs.sort();
    assert_eq!(dirs, vec!["C--a".to_string()]);
    assert!(list_encoded_dirs(f.dir.path().join("nonexistent").as_path()).is_empty());
}

#[test]
fn probe_real_path_reports_missing_and_existing() {
    use cc_core::discovery::probe_real_path;
    use std::time::Duration;
    let f = FakeRoot::new();
    let existing = f.path().to_string_lossy().to_string();
    assert_eq!(probe_real_path(&existing, Duration::from_secs(2)), PathProbe::Exists);
    assert_eq!(probe_real_path(r"C:\definitely\not\here\xyz", Duration::from_secs(2)), PathProbe::Missing);
}

#[test]
fn legacy_dir_is_explained_even_when_current_dir_also_exists() {
    // 真机场景 F4：同一项目同时有新编码目录和旧编码残留目录
    let mut f = FakeRoot::new();
    f.config_entry("C:/work/win_project/food-ordering", json!({}));
    f.project_dir("C--work-win-project-food-ordering");
    f.project_dir("C--work-win_project-food-ordering");
    let cfg = load_config(&f.root.config_file).unwrap();
    let (projects, check) = discover(Some(&cfg), &list_encoded_dirs(f.path()), &probe_all_exist);
    let owner = projects.iter().find(|p| p.id == "C:/work/win_project/food-ordering").unwrap();
    assert_eq!(owner.state, ProjectState::Normal);
    assert_eq!(owner.encoded_dir.as_deref(), Some("C--work-win-project-food-ordering"));
    let legacy = projects.iter().find(|p| p.id == "dir:C--work-win_project-food-ordering").unwrap();
    assert_eq!(legacy.state, ProjectState::LegacyEncoded);
    assert_eq!(legacy.legacy_of.as_deref(), Some("C:/work/win_project/food-ordering"));
    assert_eq!(check.matched_by_current_rule, 1);
    assert_eq!(check.matched_by_legacy_rule, 1);
    assert!(check.unmatched.is_empty());
    assert!(check.migration_enabled);
}
