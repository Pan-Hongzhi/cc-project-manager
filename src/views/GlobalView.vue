<script setup lang="ts">
import { computed } from "vue";
import { NTable, NStatistic, NSpace, NButton, NAlert, NText, NDivider, NGrid, NGi, NTag, useMessage } from "naive-ui";
import { api, type CategoryEntry, type Project, type Retention } from "../api";
import { useOverviewStore } from "../stores/overview";
import { categoryLabel, consequenceLabel, kindLabel, retentionLabel, retentionTagType } from "../labels";
import { formatBytes, formatTokens, categoryKey } from "../utils/format";
import { t, translateError } from "../i18n";
import StackedBar, { type Segment } from "../components/StackedBar.vue";

const store = useOverviewStore();
const message = useMessage();
const ov = computed(() => store.overview);
const scan = computed(() => ov.value?.scan ?? null);
const globalSorted = computed(() => [...(scan.value?.global ?? [])].sort((a, b) => b.bytes - a.bytes));
const metaOf = (key: string) => ov.value?.category_meta.find((m) => categoryKey(m.category) === key);

/** 每个类别（按 kind）的清扫策略；带名字的类别按 kind 推断 */
function retentionOf(e: CategoryEntry): Retention {
  const m = metaOf(categoryKey(e.category));
  if (m) return m.retention;
  if (e.category.kind === "protected") return { kind: "protected" };
  if (e.category.kind === "legacy") return { kind: "legacy_removed" };
  return { kind: "unknown" };
}

// —— 占比图 1：按类别（同一类别永远同一颜色；带名字的类别按 kind 聚合）——
// 颜色来自经 CVD 校验的 8 色分类调色板（深色表面），按类别身份固定分配，不按排名
const KIND_COLOR: Record<string, string> = {
  transcripts: "#3987e5",
  auto_memory: "#d95926",
  file_history: "#199e70",
  paste_cache: "#c98500",
  history_log: "#d55181",
  protected: "#9085e9",
  unknown: "#e66767",
  legacy: "#008300",
};
const OTHER_COLOR = "#71717a";
const categorySegments = computed<Segment[]>(() => {
  const byKind = new Map<string, number>();
  for (const e of scan.value?.global ?? []) byKind.set(e.category.kind, (byKind.get(e.category.kind) ?? 0) + e.bytes);
  const segs: Segment[] = [];
  let other = 0;
  for (const [kind, bytes] of byKind) {
    const color = KIND_COLOR[kind];
    if (color) {
      const r = retentionOf({ category: { kind }, bytes, file_count: 0, oldest_mtime_ms: null, newest_mtime_ms: null });
      segs.push({ key: kind, label: kindLabel(kind), value: bytes, color, note: retentionLabel(r), hatched: kind === "unknown" });
    } else other += bytes; // uploads / debug / plans / tasks / session_env / stats_cache 合并为「其他」
  }
  segs.sort((a, b) => b.value - a.value);
  if (other > 0) segs.push({ key: "other", label: t("global.other.label"), value: other, color: OTHER_COLOR, note: t("global.other.note") });
  return segs;
});

// —— 占比图 2：按清扫策略（状态色：会自动消失=绿，永久=灰，受保护=蓝，未知=红）——
const POLICY: { key: string; color: string; match: (r: Retention) => boolean; hatched?: boolean }[] = [
  { key: "auto", color: "#22c55e", match: (r) => r.kind === "auto_cleanup" || r.kind === "legacy_removed" },
  { key: "memory", color: "#d95926", match: (r) => r.kind === "memory_rule" },
  { key: "permanent", color: "#71717a", match: (r) => r.kind === "permanent" },
  { key: "protected", color: "#9085e9", match: (r) => r.kind === "protected" },
  { key: "unknown", color: "#e66767", match: (r) => r.kind === "unknown", hatched: true },
];
const policySegments = computed<Segment[]>(() => {
  const sums = new Map<string, number>();
  for (const e of scan.value?.global ?? []) {
    const r = retentionOf(e);
    const p = POLICY.find((x) => x.match(r)) ?? POLICY[POLICY.length - 1];
    sums.set(p.key, (sums.get(p.key) ?? 0) + e.bytes);
  }
  return POLICY.map((p) => ({
    key: p.key,
    label: t(`global.policy.${p.key}`),
    value: sums.get(p.key) ?? 0,
    color: p.color,
    note: t(`global.policy.${p.key}.note`),
    hatched: p.hatched,
  }));
});

// —— 清扫建议 ——
interface Advice { level: "success" | "info" | "warning" | "error"; title: string; detail: string; action: string; bytes: number }
const IDLE_DAYS = 90;
const BIG_BYTES = 200 * 1024 * 1024;
const advice = computed<Advice[]>(() => {
  const o = ov.value;
  if (!o || !scan.value) return [];
  const list: Advice[] = [];
  const now = Date.now();
  const projects = o.projects;
  const sizeOf = (p: Project) => p.size?.total_bytes ?? 0;
  const sum = (ps: Project[]) => ps.reduce((s, p) => s + sizeOf(p), 0);

  const pv = o.cleanup_preview;
  const autoTotal = policySegments.value.find((s) => s.key === "auto")?.value ?? 0;
  if (pv) {
    list.push({
      level: "success",
      title: t("advice.auto.title"),
      detail: t("advice.auto.detail", { total: formatBytes(autoTotal), days: scan.value.cleanup_days, files: pv.files, size: formatBytes(pv.bytes) }),
      action: t("advice.auto.action"),
      bytes: pv.bytes,
    });
  }

  const idleBig = projects.filter((p) => sizeOf(p) >= BIG_BYTES && (p.last_active_ms === null || now - p.last_active_ms > IDLE_DAYS * 86_400_000));
  if (idleBig.length) {
    const top = [...idleBig].sort((a, b) => sizeOf(b) - sizeOf(a)).slice(0, 3).map((p) => `${p.real_path ?? p.encoded_dir} (${formatBytes(sizeOf(p))})`).join("; ");
    list.push({
      level: "warning",
      title: t("advice.idle.title", { n: idleBig.length, days: IDLE_DAYS }),
      detail: t("advice.idle.detail", { total: formatBytes(sum(idleBig)), top }),
      action: t("advice.idle.action"),
      bytes: sum(idleBig),
    });
  }

  const orphans = projects.filter((p) => p.state === "orphan");
  if (orphans.length) {
    const withData = orphans.filter((p) => sizeOf(p) > 0);
    list.push({
      level: "warning",
      title: t("advice.orphan.title", { n: orphans.length }),
      detail: withData.length
        ? t("advice.orphan.detailWithData", { n: withData.length, total: formatBytes(sum(withData)), list: withData.map((p) => p.real_path).join("; ") })
        : t("advice.orphan.detailNoData"),
      action: t("advice.orphan.action"),
      bytes: sum(withData),
    });
  }

  const legacy = projects.filter((p) => p.state === "legacy_encoded" || p.state === "unowned");
  if (legacy.length) {
    list.push({
      level: "info",
      title: t("advice.legacy.title", { n: legacy.length }),
      detail: t("advice.legacy.detail", { total: formatBytes(sum(legacy)), list: legacy.map((p) => p.encoded_dir).join("; ") }),
      action: t("advice.legacy.action"),
      bytes: sum(legacy),
    });
  }

  const unknown = policySegments.value.find((s) => s.key === "unknown")?.value ?? 0;
  if (unknown > 0) {
    list.push({ level: "error", title: t("advice.unknown.title", { size: formatBytes(unknown) }), detail: t("advice.unknown.detail"), action: t("advice.unknown.action"), bytes: 0 });
  }

  const protectedBytes = policySegments.value.find((s) => s.key === "protected")?.value ?? 0;
  const memoryBytes = policySegments.value.find((s) => s.key === "memory")?.value ?? 0;
  list.push({
    level: "info",
    title: t("advice.protected.title", { size: formatBytes(protectedBytes + memoryBytes) }),
    detail: t("advice.protected.detail", { p: formatBytes(protectedBytes), m: formatBytes(memoryBytes) }),
    action: t("advice.protected.action"),
    bytes: 0,
  });
  return list;
});
const reclaimable = computed(() => advice.value.reduce((s, a) => s + a.bytes, 0));

async function openInsights() {
  try { await api.openInsights(); } catch (e) { message.error(translateError(e)); }
}
async function openDataRoot() {
  try { await api.openPath(ov.value!.root.root); } catch (e) { message.error(translateError(e)); }
}
</script>

<template>
  <div class="global">
    <NGrid :cols="4" :x-gap="16">
      <NGi><NStatistic :label="t('global.stat.total')" :value="scan ? formatBytes(scan.root_total_bytes) : '—'" /></NGi>
      <NGi>
        <NStatistic :label="t('global.stat.reclaimable')" :value="scan ? formatBytes(reclaimable) : '—'">
          <template #suffix><NText depth="3" style="font-size: 12px">{{ t("global.stat.reclaimableNote") }}</NText></template>
        </NStatistic>
      </NGi>
      <NGi>
        <NStatistic :label="t('global.stat.cleanupPeriod')" :value="scan ? t('global.stat.days', { days: scan.cleanup_days }) : '—'">
          <template #suffix><NText depth="3" style="font-size: 12px">{{ scan?.cleanup_days_source === "explicit" ? t("global.stat.explicit") : t("global.stat.default") }}</NText></template>
        </NStatistic>
      </NGi>
      <NGi><NStatistic :label="t('global.stat.projects')" :value="ov?.projects.length ?? 0" /></NGi>
    </NGrid>

    <template v-if="scan">
      <NDivider title-placement="left">{{ t("global.section.byCategory") }}</NDivider>
      <StackedBar :segments="categorySegments" />

      <NDivider title-placement="left">{{ t("global.section.byPolicy") }}</NDivider>
      <StackedBar :segments="policySegments" />

      <NDivider title-placement="left">{{ t("global.section.advice") }}</NDivider>
      <div class="advice">
        <NAlert v-for="(a, i) in advice" :key="i" :type="a.level" :title="a.title" :show-icon="true">
          <div class="advice-detail">{{ a.detail }}</div>
          <div class="advice-action"><span class="advice-arrow">→</span>{{ a.action }}</div>
        </NAlert>
      </div>

      <NDivider title-placement="left">{{ t("global.section.details") }}</NDivider>
      <div class="policy-legend">
        <NText depth="3" style="font-size: 12px; margin-right: 4px">{{ t("global.legend.title") }}</NText>
        <NTag size="small" round :bordered="false" type="success">{{ t("global.legend.auto") }}</NTag>
        <NTag size="small" round :bordered="false" type="default">{{ t("global.legend.permanent") }}</NTag>
        <NTag size="small" round :bordered="false" type="info">{{ t("global.legend.protected") }}</NTag>
        <NTag size="small" round :bordered="false" type="warning">{{ t("global.legend.legacy") }}</NTag>
        <NTag size="small" round :bordered="false" type="error">{{ t("global.legend.unknown") }}</NTag>
      </div>
      <NTable size="small" :single-line="false">
        <thead><tr><th>{{ t("global.col.category") }}</th><th class="num">{{ t("global.col.size") }}</th><th class="num">{{ t("global.col.share") }}</th><th class="num">{{ t("global.col.files") }}</th><th>{{ t("global.col.policy") }}</th><th>{{ t("global.col.consequence") }}</th></tr></thead>
        <tbody>
          <tr v-for="e in globalSorted" :key="categoryKey(e.category)">
            <td>
              <i class="dot" :style="{ background: KIND_COLOR[e.category.kind] ?? OTHER_COLOR }" />
              {{ categoryLabel(e.category) }}
            </td>
            <td class="num mono">{{ formatBytes(e.bytes) }}</td>
            <td class="num mono">{{ scan.root_total_bytes ? ((e.bytes / scan.root_total_bytes) * 100).toFixed(1) + "%" : "—" }}</td>
            <td class="num mono">{{ e.file_count }}</td>
            <td><NTag size="small" round :bordered="false" :type="retentionTagType(retentionOf(e))">{{ retentionLabel(retentionOf(e)) }}</NTag></td>
            <td class="muted">{{ consequenceLabel(e.category) }}</td>
          </tr>
        </tbody>
      </NTable>
    </template>
    <NText v-else depth="3">{{ t("global.notScanned") }}</NText>

    <NDivider title-placement="left">{{ t("global.section.tokens") }}</NDivider>
    <template v-if="ov?.stats.available">
      <NSpace style="margin-bottom: 8px" :size="32">
        <NStatistic :label="t('global.tokens.sessions')" :value="ov.stats.total_sessions" />
        <NStatistic :label="t('global.tokens.messages')" :value="ov.stats.total_messages" />
        <NStatistic :label="t('global.tokens.asOf')" :value="ov.stats.last_computed_date ?? '—'" />
      </NSpace>
      <NTable size="small" :single-line="false">
        <thead><tr><th>{{ t("global.tokens.model") }}</th><th class="num">{{ t("detail.input") }}</th><th class="num">{{ t("detail.output") }}</th><th class="num">{{ t("detail.cacheWrite") }}</th><th class="num">{{ t("detail.cacheRead") }}</th></tr></thead>
        <tbody>
          <tr v-for="m in ov.stats.models" :key="m.model">
            <td class="mono">{{ m.model }}</td><td class="num mono">{{ formatTokens(m.input) }}</td><td class="num mono">{{ formatTokens(m.output) }}</td>
            <td class="num mono">{{ formatTokens(m.cache_creation) }}</td><td class="num mono">{{ formatTokens(m.cache_read) }}</td>
          </tr>
        </tbody>
      </NTable>
    </template>
    <NAlert v-else type="warning">{{ ov?.stats.reason ?? t("global.tokens.unavailable") }}</NAlert>
    <NText depth="3" style="font-size: 12px; display: block; margin: 4px 0 8px">{{ t("global.tokens.note") }}</NText>
    <NSpace>
      <NButton size="small" @click="openInsights">{{ t("global.btn.insights") }}</NButton>
      <NButton size="small" @click="openDataRoot" :disabled="!ov">{{ t("global.btn.openRoot") }}</NButton>
    </NSpace>
  </div>
</template>

<style scoped>
.global { padding: 8px 0 16px; }
.advice { display: grid; gap: 10px; }
.advice-detail { color: var(--text); }
.advice-action { margin-top: 6px; color: var(--muted); }
.advice-arrow { color: var(--coral); margin-right: 6px; font-weight: 600; }
.policy-legend { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin-right: 8px; vertical-align: middle; }
.num { text-align: right; }
.muted { color: var(--muted); }
</style>
