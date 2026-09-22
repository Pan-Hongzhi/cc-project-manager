//! CC 把真实路径有损编码成 projects/ 下的目录名。规则来自本机实测（设计 F3/F4）。

pub fn normalize_path(real: &str) -> String {
    real.replace('\\', "/")
}

fn encode_with(real: &str, keep: impl Fn(char) -> bool) -> String {
    normalize_path(real)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || keep(c) { c } else { '-' })
        .collect()
}

/// 当前规则：非 [A-Za-z0-9-] 的每个字符 → '-'
pub fn encode_path(real: &str) -> String {
    encode_with(real, |c| c == '-')
}

/// 旧规则：与当前规则相同，但保留下划线。
pub fn encode_path_legacy(real: &str) -> String {
    encode_with(real, |c| c == '-' || c == '_')
}

fn canonical_for_compare(p: &str) -> String {
    let n = normalize_path(p);
    let trimmed = n.trim_end_matches('/');
    trimmed.to_ascii_lowercase()
}

pub fn same_path(a: &str, b: &str) -> bool {
    canonical_for_compare(a) == canonical_for_compare(b)
}

pub fn is_same_or_child(child: &str, parent: &str) -> bool {
    let c = canonical_for_compare(child);
    let p = canonical_for_compare(parent);
    c == p || c.starts_with(&format!("{p}/"))
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn encodes_windows_path_with_underscore_to_dash() {
        assert_eq!(
            encode_path(r"C:\Users\alice\Desktop\work\win_project\cc-project-manager"),
            "C--Users-alice-Desktop-work-win-project-cc-project-manager"
        );
    }

    #[test]
    fn encodes_forward_slash_config_key_identically() {
        assert_eq!(
            encode_path("C:/Users/alice/Desktop/work/win_project/cc-project-manager"),
            "C--Users-alice-Desktop-work-win-project-cc-project-manager"
        );
    }

    #[test]
    fn encodes_unc_and_drive_root() {
        assert_eq!(encode_path("//192.168.1.10/share/camera"), "--192-168-1-10-share-camera");
        assert_eq!(encode_path("Y:/imv350"), "Y--imv350");
    }

    #[test]
    fn each_non_ascii_char_becomes_one_dash() {
        // 「工作记录」四个字 → 四个 -，前后分隔符各一个 -
        assert_eq!(encode_path("C:/Users/x/work/工作记录/2026.3"), "C--Users-x-work------2026-3");
    }

    #[test]
    fn legacy_rule_keeps_underscore() {
        assert_eq!(
            encode_path_legacy("C:/Users/alice/Desktop/work/win_project/food-ordering"),
            "C--Users-alice-Desktop-work-win_project-food-ordering"
        );
    }

    #[test]
    fn same_path_ignores_slash_style_case_and_trailing_slash() {
        assert!(same_path(r"C:\Proj\A\", "c:/proj/a"));
        assert!(!same_path("C:/proj/a", "C:/proj/ab"));
    }

    #[test]
    fn child_detection() {
        assert!(is_same_or_child(r"C:\proj\a\sub", "C:/proj/a"));
        assert!(is_same_or_child("C:/proj/a", "C:/proj/a"));
        assert!(!is_same_or_child("C:/proj/ab", "C:/proj/a"));
    }
}
