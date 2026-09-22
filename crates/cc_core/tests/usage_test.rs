mod common;
use cc_core::usage::{parse_transcript, update_project_usage, UsageCache};
use common::FakeRoot;

fn assistant(id: &str, input: u64, output: u64, cc: u64, cr: u64) -> String {
    format!(
        r#"{{"type":"assistant","uuid":"u-{id}-{output}","message":{{"id":"{id}","role":"assistant","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_creation_input_tokens":{cc},"cache_read_input_tokens":{cr},"service_tier":"standard"}}}}}}"#
    )
}

#[test]
fn dedupes_by_message_id_taking_last_row() {
    let f = FakeRoot::new();
    let text = [
        r#"{"type":"user","message":{"role":"user","content":"hi"}}"#.to_string(),
        assistant("m1", 10, 1, 100, 1000),
        assistant("m1", 10, 5, 100, 1000), // 同一消息流式追加，取最后一行
        assistant("m2", 20, 7, 0, 0),
        r#"{"type":"assistant","message":{"id":"m3","usage":{"input_tokens":"oops"}}}"#.to_string(), // 类型不符 → 按 0
        r#"{"type":"ai-title","aiTitle":"x"}"#.to_string(),
        "not json at all".to_string(),
        "".to_string(),
    ]
    .join("\n");
    let p = f.project_file("C--a", "s.jsonl", &text);
    let s = parse_transcript(&p).unwrap();
    assert_eq!(s.message_count, 3);
    assert_eq!((s.input, s.output, s.cache_creation, s.cache_read), (30, 12, 100, 1000));
    assert_eq!(s.skipped_lines, 1);
    assert_eq!(s.session_count, 0);
}

#[test]
fn assistant_row_without_message_id_falls_back_to_uuid() {
    let f = FakeRoot::new();
    let text = [
        r#"{"type":"assistant","uuid":"a","message":{"usage":{"input_tokens":1,"output_tokens":1}}}"#,
        r#"{"type":"assistant","uuid":"b","message":{"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    ]
    .join("\n");
    let p = f.project_file("C--a", "s.jsonl", &text);
    assert_eq!(parse_transcript(&p).unwrap().message_count, 2);
}

#[test]
fn project_usage_is_incremental_and_counts_sessions() {
    let f = FakeRoot::new();
    f.project_file("C--a", "s1.jsonl", &assistant("m1", 1, 2, 3, 4));
    f.project_file("C--a", "s2.jsonl", &assistant("m2", 10, 20, 30, 40));
    f.project_file("C--a", "subagents/x/agent.jsonl", &assistant("m3", 100, 200, 300, 400));
    f.project_file("C--a", "memory/MEMORY.md", "not a transcript");
    let dir = f.project_dir("C--a");

    let mut cache = UsageCache::default();
    let s = update_project_usage(&dir, &mut cache);
    assert_eq!((s.input, s.output, s.cache_creation, s.cache_read), (111, 222, 333, 444));
    assert_eq!(s.session_count, 2, "subagents 不算会话");
    assert_eq!(s.message_count, 3);
    assert_eq!(cache.files.len(), 3);

    // 篡改缓存值、不改文件 → 复用缓存
    for e in cache.files.values_mut() {
        e.stat.output = 0;
    }
    let s2 = update_project_usage(&dir, &mut cache);
    assert_eq!(s2.output, 0);

    // 删除一个文件 → 缓存条目被移除
    std::fs::remove_file(dir.join("s2.jsonl")).unwrap();
    let s3 = update_project_usage(&dir, &mut cache);
    assert_eq!(s3.session_count, 1);
    assert_eq!(cache.files.len(), 2);
}
