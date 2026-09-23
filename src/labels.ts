import type { Category, ProjectState, Retention, Verified } from "./api";
import { t } from "./i18n";

export function stateLabel(s: ProjectState): string {
  return t(`state.${s}`);
}

export type TagType = "default" | "success" | "warning" | "error" | "info";
export function stateTagType(s: ProjectState): TagType {
  // 绿色留给「运行中」（呼应图标里的绿色光标块），正常项目用中性色
  return { normal: "default", config_only: "default", orphan: "error", unreachable: "warning", unowned: "warning", legacy_encoded: "info" }[s] as TagType;
}

export function verifiedLabel(v: Verified): string {
  return t(`verified.${v}`);
}

/** 按 kind 聚合时的类别名（受保护 / 未知 / 旧版遗留等不带具体目录名） */
export function kindLabel(kind: string): string {
  const key = `category.${kind}`;
  const s = t(key);
  return s === key ? kind : s;
}

export function categoryLabel(c: Category): string {
  if (c.kind === "unknown" && c.name === "projects/other") return t("category.projectsOther");
  const label = kindLabel(c.kind);
  return c.name ? `${label}（${c.name}）` : label;
}

/** 类别的删除后果 / 说明（不再依赖后端文案，便于随语言切换） */
export function consequenceLabel(c: Category): string {
  switch (c.kind) {
    case "paste_cache":
    case "uploads":
    case "debug":
    case "plans":
    case "tasks":
    case "session_env":
      return t("consequence.temp");
    case "transcripts":
    case "auto_memory":
    case "file_history":
    case "history_log":
    case "stats_cache":
    case "legacy":
    case "protected":
      return t(`consequence.${c.kind}`);
    default:
      return t("consequence.unknown");
  }
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
  return r.kind === "auto_cleanup" ? t("retention.auto_cleanup", { days: r.days }) : t(`retention.${r.kind}`);
}
