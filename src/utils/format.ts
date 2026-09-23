import type { Category } from "../api";
import { t } from "../i18n";

export function formatBytes(n: number): string {
  if (n < 1024) return `${Math.round(n)} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(1)} ${units[i]}`;
}

function pad(n: number): string {
  return n.toString().padStart(2, "0");
}

export function formatDateTime(ms: number | null): string {
  if (ms === null || ms === undefined) return "—";
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function formatRelative(ms: number | null, now: number = Date.now()): string {
  if (ms === null || ms === undefined) return "—";
  const diff = Math.max(0, now - ms);
  const min = Math.floor(diff / 60_000);
  if (min < 1) return t("time.now");
  if (min < 60) return t("time.minutes", { n: min });
  const h = Math.floor(min / 60);
  if (h < 24) return t("time.hours", { n: h });
  const d = Math.floor(h / 24);
  if (d < 30) return t("time.days", { n: d });
  return t("time.months", { n: Math.floor(d / 30) });
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  return n.toLocaleString("en-US");
}

export function categoryKey(c: Category): string {
  return c.name ? `${c.kind}:${c.name}` : c.kind;
}
