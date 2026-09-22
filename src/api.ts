import { invoke } from "@tauri-apps/api/core";

export type ProjectState = "normal" | "config_only" | "orphan" | "unreachable" | "unowned" | "legacy_encoded";
export type Verified = "alive" | "stale" | "unknown";
export interface Category { kind: string; name?: string }
export type Retention =
  | { kind: "auto_cleanup"; days: number }
  | { kind: "permanent" } | { kind: "memory_rule" } | { kind: "legacy_removed" } | { kind: "protected" } | { kind: "unknown" };
export interface CategoryMeta { category: Category; retention: Retention; deletable_by_tool: boolean; consequence: string }
export interface CategoryEntry { category: Category; bytes: number; file_count: number; oldest_mtime_ms: number | null; newest_mtime_ms: number | null }
export interface SizeBreakdown { total_bytes: number; by_category: CategoryEntry[] }
export interface UsageStat { input: number; output: number; cache_creation: number; cache_read: number; message_count: number; session_count: number; skipped_lines: number }
export interface MemorySummary { file_count: number; memory_md_lines: number | null; user_claude_md: boolean; user_claude_local_md: boolean; user_rules_dir: boolean }
export interface RunningSession { pid: number; session_id: string; cwd: string; started_at_ms: number; status: string; verified: Verified }
export interface ConfigHint { last_session_id: string | null; trust_accepted: boolean; last_total_input_tokens: number; last_total_output_tokens: number }
export interface Project {
  id: string; real_path: string | null; encoded_dir: string | null; state: ProjectState; legacy_of: string | null;
  running: RunningSession | null; last_active_ms: number | null; config_hint: ConfigHint | null;
  size: SizeBreakdown | null; usage: UsageStat | null; memory_summary: MemorySummary | null;
}
export interface EncodingSelfCheck { total_entries: number; matched_by_current_rule: number; matched_by_legacy_rule: number; unmatched: string[]; migration_enabled: boolean }
export interface ProjectScan { size: SizeBreakdown; last_active_ms: number | null; session_count: number; memory_file_count: number; memory_md_lines: number | null }
export interface ScanResult { scanned_at_ms: number; root_total_bytes: number; global: CategoryEntry[]; per_project: Record<string, ProjectScan>; inaccessible: number; cleanup_days: number; cleanup_days_source: "explicit" | "default" }
export interface CleanupPreview { cleanup_days: number; files: number; bytes: number; by_category: CategoryEntry[] }
export interface ModelTotals { model: string; input: number; output: number; cache_creation: number; cache_read: number }
export interface GlobalStats { available: boolean; reason: string | null; version: number | null; last_computed_date: string | null; first_session_date: string | null; total_sessions: number; total_messages: number; models: ModelTotals[] }
export interface CliInfo { path: string; version: string | null; kind: "npm_cmd" | "native_exe" | "other" }
export interface DataRoot { root: string; config_file: string; source: "env_var" | "default" }
export interface ScanProgress { done: number; total: number; current: string; bytes_so_far: number }
export interface Overview {
  root: DataRoot; config_error: string | null; projects: Project[]; self_check: EncodingSelfCheck; sessions: RunningSession[];
  scan: ScanResult | null; cleanup_preview: CleanupPreview | null; category_meta: CategoryMeta[]; stats: GlobalStats;
  cli: CliInfo | null; tool_data_dir: string; generated_at_ms: number; scan_is_cached: boolean;
}

export const api = {
  getOverview: () => invoke<Overview>("get_overview"),
  refresh: () => invoke<void>("refresh"),
  openPath: (path: string) => invoke<void>("open_in_explorer", { path }),
  runClaude: (path: string, resume: boolean) => invoke<void>("run_claude", { path, resume }),
  openInsights: () => invoke<void>("open_insights_report"),
  openToolDataDir: () => invoke<void>("open_tool_data_dir"),
};
