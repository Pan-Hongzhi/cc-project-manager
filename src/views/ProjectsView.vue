<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref } from "vue";
import { NDataTable, NTag, NSpace, NSelect, NInputNumber, NText, useMessage, type DataTableColumns } from "naive-ui";
import type { Project, ProjectState } from "../api";
import { useOverviewStore } from "../stores/overview";
import { stateLabel, stateTagType } from "../labels";
import { t, translateError } from "../i18n";
import { formatBytes, formatRelative, formatTokens } from "../utils/format";
import ProjectDetail from "./ProjectDetail.vue";

const store = useOverviewStore();
const message = useMessage();

type Filter = "all" | ProjectState | "running" | "stale_big";
const filter = ref<Filter>("all");
const sortBy = ref<"active" | "size">("active");
const staleDays = ref(90);
const bigMb = ref(200);
const selectedId = ref<string | null>(null);

const filterOptions = computed(() => [
  { label: t("projects.filter.all"), value: "all" },
  { label: t("projects.filter.running"), value: "running" },
  { label: t("projects.filter.stale_big"), value: "stale_big" },
  { label: stateLabel("orphan"), value: "orphan" },
  { label: stateLabel("unreachable"), value: "unreachable" },
  { label: stateLabel("unowned"), value: "unowned" },
  { label: stateLabel("legacy_encoded"), value: "legacy_encoded" },
  { label: stateLabel("config_only"), value: "config_only" },
]);
const sortOptions = computed(() => [
  { label: t("projects.sort.active"), value: "active" },
  { label: t("projects.sort.size"), value: "size" },
]);

const rows = computed<Project[]>(() => {
  const now = Date.now();
  let list = store.overview?.projects ?? [];
  list = list.filter((p) => {
    switch (filter.value) {
      case "all": return true;
      case "running": return p.running !== null;
      case "stale_big": {
        const idle = p.last_active_ms === null || now - p.last_active_ms > staleDays.value * 86_400_000;
        return idle && (p.size?.total_bytes ?? 0) >= bigMb.value * 1024 * 1024;
      }
      default: return p.state === filter.value;
    }
  });
  return [...list].sort((a, b) =>
    sortBy.value === "size" ? (b.size?.total_bytes ?? 0) - (a.size?.total_bytes ?? 0) : (b.last_active_ms ?? 0) - (a.last_active_ms ?? 0),
  );
});

const columns = computed<DataTableColumns<Project>>(() => [
  {
    title: t("projects.col.state"), key: "state", width: 170,
    render: (p) => h(NSpace, { size: 4 }, () => [
      h(NTag, { size: "small", type: stateTagType(p.state) }, () => stateLabel(p.state)),
      p.running ? h(NTag, { size: "small", type: "success", bordered: false }, () => t("running")) : null,
    ]),
  },
  {
    title: t("projects.col.path"), key: "path", ellipsis: { tooltip: true },
    render: (p) => p.real_path ? h("span", { class: "mono" }, p.real_path) : h(NText, { depth: 3, class: "mono" }, () => `${t("projects.dataDirPrefix")}${p.encoded_dir}`),
  },
  { title: t("projects.col.active"), key: "last_active_ms", width: 120, render: (p) => formatRelative(p.last_active_ms) },
  { title: t("projects.col.size"), key: "size", width: 100, render: (p) => h("span", { class: "mono" }, p.size ? formatBytes(p.size.total_bytes) : "—") },
  { title: t("projects.col.tokens"), key: "usage", width: 190, render: (p) => h("span", { class: "mono" }, p.usage ? formatTokens(p.usage.input + p.usage.output + p.usage.cache_creation + p.usage.cache_read) : "—") },
  { title: t("projects.col.sessions"), key: "sessions", width: 80, render: (p) => h("span", { class: "mono" }, String(p.usage?.session_count ?? "—")) },
]);

const selected = computed(() => rows.value.find((p) => p.id === selectedId.value) ?? store.overview?.projects.find((p) => p.id === selectedId.value) ?? null);
const rowProps = (p: Project) => ({
  style: "cursor: pointer",
  class: p.id === selectedId.value ? "is-selected" : "",
  onClick: () => (selectedId.value = p.id),
});

// 表格与详情面板的高度按窗口实际剩余空间计算，窗口放大时跟着长高，不留底部空白
const tableWrap = ref<HTMLElement | null>(null);
const detailWrap = ref<HTMLElement | null>(null);
const tableMaxHeight = ref(400);
const detailMaxHeight = ref(500);
const BOTTOM_GAP = 16;
function fit() {
  if (tableWrap.value) {
    const top = tableWrap.value.getBoundingClientRect().top;
    // NDataTable 的 max-height 只作用于表体，表头高度要另外扣掉，否则整页会多出一截可滚动区域
    const header = tableWrap.value.querySelector<HTMLElement>(".n-data-table-base-table-header");
    const headerHeight = header?.offsetHeight ?? 40;
    tableMaxHeight.value = Math.max(160, Math.floor(window.innerHeight - top - headerHeight - BOTTOM_GAP));
  }
  if (detailWrap.value) {
    const top = detailWrap.value.getBoundingClientRect().top;
    detailMaxHeight.value = Math.max(160, Math.floor(window.innerHeight - top - BOTTOM_GAP));
  }
}
let observer: ResizeObserver | null = null;
onMounted(() => {
  fit();
  observer = new ResizeObserver(() => fit());
  observer.observe(document.body);
});
onUnmounted(() => observer?.disconnect());
</script>

<template>
  <div class="projects">
    <div class="list">
      <NSpace align="center" style="margin-bottom: 8px" wrap>
        <NSelect v-model:value="filter" :options="filterOptions" size="small" style="width: 180px" />
        <NSelect v-model:value="sortBy" size="small" style="width: 150px" :options="sortOptions" />
        <template v-if="filter === 'stale_big'">
          <NText depth="3">{{ t("projects.idleOver") }}</NText><NInputNumber v-model:value="staleDays" size="small" :min="1" style="width: 90px" /><NText depth="3">{{ t("projects.daysAnd") }}</NText>
          <NInputNumber v-model:value="bigMb" size="small" :min="1" style="width: 100px" /><NText depth="3">MB</NText>
        </template>
        <NText depth="3">{{ t("projects.count", { n: rows.length }) }}</NText>
      </NSpace>
      <div ref="tableWrap">
        <NDataTable :columns="columns" :data="rows" :row-key="(p: Project) => p.id" :row-props="rowProps" size="small" :max-height="tableMaxHeight" :loading="store.loading" />
      </div>
    </div>
    <div ref="detailWrap" class="detail" :style="{ maxHeight: detailMaxHeight + 'px' }">
      <ProjectDetail v-if="selected" :project="selected" @error="(m: string) => message.error(translateError(m))" />
      <NText v-else depth="3">{{ t("projects.selectHint") }}</NText>
    </div>
  </div>
</template>

<style scoped>
.projects { display: grid; grid-template-columns: minmax(0, 1fr) 420px; gap: 16px; padding-top: 8px; align-items: start; }
.detail { border-left: 1px solid var(--chrome); padding-left: 16px; overflow: auto; }
</style>
