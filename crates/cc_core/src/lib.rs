//! cc_core：CC Project Manager 的核心库。无 UI 依赖，所有逻辑可独立测试。
//! 只读原则：本 crate 中 M1 阶段不存在任何对数据根目录的写操作。

pub mod config;
pub mod discovery;
pub mod encoding;
pub mod model;
pub mod paths;
pub mod sessions;

#[cfg(test)]
mod smoke {
    #[test]
    fn workspace_builds() {
        assert_eq!(2 + 2, 4);
    }
}
