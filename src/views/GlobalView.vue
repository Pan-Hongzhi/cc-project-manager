<script setup lang="ts">
import { computed } from "vue";
import { NTable, NStatistic, NSpace, NButton, NAlert, NText, NDivider, NGrid, NGi, NTag, useMessage } from "naive-ui";
import { api, type CategoryEntry, type Project, type Retention } from "../api";
import { useOverviewStore } from "../stores/overview";
import { categoryLabel, kindLabel, retentionLabel, retentionTagType } from "../labels";
import { formatBytes, formatTokens, categoryKey } from "../utils/format";
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
  if (other > 0) segs.push({ key: "other", label: "其他（上传、调试、计划、任务、会话环境、统计缓存）", value: other, color: OTHER_COLOR, note: "30 天自动清扫 / 永久保留" });
  return segs;
});

// —— 占比图 2：按清扫策略（状态色：会自动消失=绿，永久=灰，受保护=蓝，未知=红）——
const POLICY: { key: string; label: string; color: string; note: string; match: (r: Retention) => boolean; hatched?: boolean }[] = [
  { key: "auto", label: "会自动清扫", color: "#22c55e", note: "超过清扫周期后 CC 自动删除，无需处理", match: (r) => r.kind === "auto_cleanup" || r.kind === "legacy_removed" },
  { key: "memory", label: "自动记忆", color: "#d95926", note: "不参与清扫，是 CC 积累的项目知识", match: (r) => r.kind === "memory_rule" },
  { key: "permanent", label: "永久保留", color: "#71717a", note: "prompt 历史与用量统计，体积很小", match: (r) => r.kind === "permanent" },
  { key: "protected", label: "受保护", color: "#9085e9", note: "凭据、配置、插件、技能、全局记忆，工具永不触碰", match: (r) => r.kind === "protected" },
  { key: "unknown", label: "未知", color: "#e66767", note: "不认识的数据，建议人工查看", match: (r) => r.kind === "unknown", hatched: true },
];
const policySegments = computed<Segment[]>(() => {
  const sums = new Map<string, number>();
  for (const e of scan.value?.global ?? []) {
    const r = retentionOf(e);
    const p = POLICY.find((x) => x.match(r)) ?? POLICY[POLICY.length - 1];
    sums.set(p.key, (sums.get(p.key) ?? 0) + e.bytes);
  }
  return POLICY.map((p) => ({ key: p.key, label: p.label, value: sums.get(p.key) ?? 0, color: p.color, note: p.note, hatched: p.hatched }));
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

  const pv = o.cleanup_preview;
  const autoTotal = policySegments.value.find((s) => s.key === "auto")?.value ?? 0;
  if (pv) {
    list.push({
      level: "success",
      title: "会自动清扫的数据不必动手",
      detail: `转录、文件快照、粘贴缓存等共 ${formatBytes(autoTotal)} 属于 ${scan.value.cleanup_days} 天自动清扫类别，其中 ${pv.files} 个文件（${formatBytes(pv.bytes)}）已超期，CC 下次启动清扫时会自动删除。`,
      action: "无需操作。若想立刻释放空间，可在项目页打开对应数据目录手动删除。",
      bytes: pv.bytes,
    });
  }

  const idleBig = projects.filter((p) => sizeOf(p) >= BIG_BYTES && (p.last_active_ms === null || now - p.last_active_ms > IDLE_DAYS * 86_400_000));
  if (idleBig.length) {
    const top = [...idleBig].sort((a, b) => sizeOf(b) - sizeOf(a)).slice(0, 3).map((p) => `${p.real_path ?? p.encoded_dir}（${formatBytes(sizeOf(p))}）`).join("；");
    list.push({
      level: "warning",
      title: `${idleBig.length} 个项目超过 ${IDLE_DAYS} 天未用且占用 ≥ 200 MB`,
      detail: `共 ${formatBytes(idleBig.reduce((s, p) => s + sizeOf(p), 0))}。最大的几个：${top}。`,
      action: "建议优先清理。项目页筛选「长期未用且占用大」可逐个查看；确认不再需要的项目，可在终端执行 claude project purge <路径> 删除其全部状态。",
      bytes: idleBig.reduce((s, p) => s + sizeOf(p), 0),
    });
  }

  const orphans = projects.filter((p) => p.state === "orphan");
  if (orphans.length) {
    const withData = orphans.filter((p) => sizeOf(p) > 0);
    list.push({
      level: "warning",
      title: `${orphans.length} 个孤儿项目，路径已不存在`,
      detail: withData.length
        ? `其中 ${withData.length} 个仍有数据，共 ${formatBytes(withData.reduce((s, p) => s + sizeOf(p), 0))}：${withData.map((p) => p.real_path).join("；")}。`
        : "都没有数据目录，只剩配置条目。",
      action: "若项目文件夹只是移动了位置，在新位置运行一次 claude 即可生成新的项目记录，旧数据可随后清理；若确实不再需要，可执行 claude project purge <路径> 连配置条目一起清除。",
      bytes: withData.reduce((s, p) => s + sizeOf(p), 0),
    });
  }

  const legacy = projects.filter((p) => p.state === "legacy_encoded" || p.state === "unowned");
  if (legacy.length) {
    list.push({
      level: "info",
      title: `${legacy.length} 个旧编码残留 / 无主数据目录`,
      detail: `共 ${formatBytes(legacy.reduce((s, p) => s + sizeOf(p), 0))}，CC 当前不会再读取它们：${legacy.map((p) => p.encoded_dir).join("；")}。`,
      action: "这些目录不影响使用，只占用空间。如需释放，可先备份其中的 memory 目录，再手动删除。",
      bytes: legacy.reduce((s, p) => s + sizeOf(p), 0),
    });
  }

  const unknown = policySegments.value.find((s) => s.key === "unknown")?.value ?? 0;
  if (unknown > 0) {
    list.push({
      level: "error",
      title: `${formatBytes(unknown)} 未知数据`,
      detail: "数据根下出现本工具不认识的目录或文件，可能是 CC 新版本新增的数据类别。",
      action: "在诊断页查看具体名字后人工判断，不要盲目删除；工具对未知数据只展示大小。",
      bytes: 0,
    });
  }

  const protectedBytes = policySegments.value.find((s) => s.key === "protected")?.value ?? 0;
  const memoryBytes = policySegments.value.find((s) => s.key === "memory")?.value ?? 0;
  list.push({
    level: "info",
    title: `${formatBytes(protectedBytes + memoryBytes)} 属于受保护与记忆数据，不建议清理`,
    detail: `受保护 ${formatBytes(protectedBytes)}（凭据、配置、插件、技能、全局记忆），自动记忆 ${formatBytes(memoryBytes)}（CC 积累的项目知识）。`,
    action: "这部分体积通常很小且不会增长失控，任何清理操作都不会触碰它们。",
    bytes: 0,
  });
  return list;
});
const reclaimable = computed(() => advice.value.reduce((s, a) => s + a.bytes, 0));

async function openInsights() {
  try { await api.openInsights(); } catch (e) { message.error(String(e)); }
}
async function openDataRoot() {
  try { await api.openPath(ov.value!.root.root); } catch (e) { message.error(String(e)); }
}
</script>

<template>
  <div class="global">
    <NGrid :cols="4" :x-gap="16">
      <NGi><NStatistic label="数据根合计" :value="scan ? formatBytes(scan.root_total_bytes) : '—'" /></NGi>
      <NGi><NStatistic label="可回收空间估算" :value="scan ? formatBytes(reclaimable) : '—'"><template #suffix><NText depth="3" style="font-size: 12px">（自动清扫 + 建议清理）</NText></template></NStatistic></NGi>
      <NGi>
        <NStatistic label="清扫周期" :value="scan ? `${scan.cleanup_days} 天` : '—'">
          <template #suffix><NText depth="3" style="font-size: 12px">{{ scan?.cleanup_days_source === "explicit" ? "（settings.json 显式设置）" : "（默认值）" }}</NText></template>
        </NStatistic>
      </NGi>
      <NGi><NStatistic label="项目数" :value="ov?.projects.length ?? 0" /></NGi>
    </NGrid>

    <template v-if="scan">
      <NDivider title-placement="left">空间占比 · 按类别</NDivider>
      <StackedBar :segments="categorySegments" />

      <NDivider title-placement="left">空间占比 · 按清扫策略</NDivider>
      <StackedBar :segments="policySegments" />

      <NDivider title-placement="left">清扫建议</NDivider>
      <div class="advice">
        <NAlert v-for="(a, i) in advice" :key="i" :type="a.level" :title="a.title" :show-icon="true">
          <div class="advice-detail">{{ a.detail }}</div>
          <div class="advice-action"><span class="advice-arrow">→</span>{{ a.action }}</div>
        </NAlert>
      </div>

      <NDivider title-placement="left">明细列表</NDivider>
      <div class="policy-legend">
        <NText depth="3" style="font-size: 12px; margin-right: 4px">清扫策略：</NText>
        <NTag size="small" round :bordered="false" type="success">30 天自动清扫</NTag>
        <NTag size="small" round :bordered="false" type="default">永久保留 / 记忆规则</NTag>
        <NTag size="small" round :bordered="false" type="info">受保护，工具不触碰</NTag>
        <NTag size="small" round :bordered="false" type="warning">旧版遗留，清扫时移除</NTag>
        <NTag size="small" round :bordered="false" type="error">未知</NTag>
      </div>
      <NTable size="small" :single-line="false">
        <thead><tr><th>类别</th><th class="num">大小</th><th class="num">占比</th><th class="num">文件数</th><th>清扫策略</th><th>删除后果 / 说明</th></tr></thead>
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
            <td class="muted">{{ metaOf(categoryKey(e.category))?.consequence ?? (e.category.kind === "protected" ? "凭据、配置、插件或运行时状态" : e.category.kind === "legacy" ? "旧版本遗留目录，CC 清扫时会移除" : "未识别数据，只展示大小") }}</td>
          </tr>
        </tbody>
      </NTable>
    </template>
    <NText v-else depth="3">尚未扫描，点击右上角「重新扫描」。</NText>

    <NDivider title-placement="left">token 总量（stats-cache.json，/usage 口径）</NDivider>
    <template v-if="ov?.stats.available">
      <NSpace style="margin-bottom: 8px" :size="32">
        <NStatistic label="会话总数" :value="ov.stats.total_sessions" />
        <NStatistic label="消息总数" :value="ov.stats.total_messages" />
        <NStatistic label="统计截至" :value="ov.stats.last_computed_date ?? '—'" />
      </NSpace>
      <NTable size="small" :single-line="false">
        <thead><tr><th>模型</th><th class="num">输入</th><th class="num">输出</th><th class="num">缓存写入</th><th class="num">缓存读取</th></tr></thead>
        <tbody>
          <tr v-for="m in ov.stats.models" :key="m.model">
            <td class="mono">{{ m.model }}</td><td class="num mono">{{ formatTokens(m.input) }}</td><td class="num mono">{{ formatTokens(m.output) }}</td>
            <td class="num mono">{{ formatTokens(m.cache_creation) }}</td><td class="num mono">{{ formatTokens(m.cache_read) }}</td>
          </tr>
        </tbody>
      </NTable>
    </template>
    <NAlert v-else type="warning">{{ ov?.stats.reason ?? "stats-cache.json 不可用" }}</NAlert>
    <NText depth="3" style="font-size: 12px; display: block; margin: 4px 0 8px">仅统计 token 数量，不换算费用。</NText>
    <NSpace>
      <NButton size="small" @click="openInsights">打开 /insights 报告</NButton>
      <NButton size="small" @click="openDataRoot" :disabled="!ov">打开数据根目录</NButton>
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
