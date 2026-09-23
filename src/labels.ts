import type { Category, ProjectState, Retention, Verified } from "./api";

export function stateLabel(s: ProjectState): string {
  return { normal: "正常", config_only: "仅配置", orphan: "孤儿项目", unreachable: "路径不可达", unowned: "无主数据", legacy_encoded: "旧编码残留" }[s];
}

export type TagType = "default" | "success" | "warning" | "error" | "info";
export function stateTagType(s: ProjectState): TagType {
  // 绿色留给「运行中」（呼应图标里的绿色光标块），正常项目用中性色
  return { normal: "default", config_only: "default", orphan: "error", unreachable: "warning", unowned: "warning", legacy_encoded: "info" }[s] as TagType;
}

export function verifiedLabel(v: Verified): string {
  return { alive: "运行中", stale: "僵尸文件", unknown: "无法确认（按运行中处理）" }[v];
}

export function categoryLabel(c: Category): string {
  const base: Record<string, string> = {
    transcripts: "会话转录", auto_memory: "自动记忆", file_history: "文件快照", paste_cache: "粘贴缓存", uploads: "上传文件",
    debug: "调试日志", plans: "计划", tasks: "任务", session_env: "会话环境", history_log: "prompt 历史", stats_cache: "用量统计缓存",
    legacy: "旧版遗留", protected: "受保护", unknown: "未知数据",
  };
  const label = base[c.kind] ?? c.kind;
  return c.name ? `${label}（${c.name}）` : label;
}

/** 按 kind 聚合时的类别名（受保护 / 未知 / 旧版遗留等不带具体目录名） */
export function kindLabel(kind: string): string {
  const base: Record<string, string> = {
    transcripts: "会话转录", auto_memory: "自动记忆", file_history: "文件快照", paste_cache: "粘贴缓存", uploads: "上传文件",
    debug: "调试日志", plans: "计划", tasks: "任务", session_env: "会话环境", history_log: "prompt 历史", stats_cache: "用量统计缓存",
    legacy: "旧版遗留", protected: "受保护", unknown: "未知数据",
  };
  return base[kind] ?? kind;
}

/** 清扫策略的标签色：会自动消失=绿，永久/记忆规则=中性，受保护=蓝，遗留=橙，未知=红 */
export function retentionTagType(r: Retention): TagType {
  switch (r.kind) {
    case "auto_cleanup": return "success";
    case "permanent": return "default";
    case "memory_rule": return "default";
    case "protected": return "info";
    case "legacy_removed": return "warning";
    default: return "error";
  }
}

export function retentionLabel(r: Retention): string {
  switch (r.kind) {
    case "auto_cleanup": return `${r.days} 天自动清扫`;
    case "permanent": return "永久保留";
    case "memory_rule": return "不参与清扫（记忆特殊规则）";
    case "legacy_removed": return "清扫时移除";
    case "protected": return "受保护，工具不触碰";
    default: return "未知";
  }
}
