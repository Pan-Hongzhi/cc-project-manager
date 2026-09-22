<script setup lang="ts">
import { computed, h, ref } from "vue";
import { NDataTable, NTag, NSpace, NSelect, NInputNumber, NText, useMessage, type DataTableColumns } from "naive-ui";
import type { Project, ProjectState } from "../api";
import { useOverviewStore } from "../stores/overview";
import { stateLabel, stateTagType } from "../labels";
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

const filterOptions = [
  { label: "全部", value: "all" },
  { label: "运行中", value: "running" },
  { label: "长期未用且占用大", value: "stale_big" },
  { label: "孤儿项目", value: "orphan" },
  { label: "路径不可达", value: "unreachable" },
  { label: "无主数据", value: "unowned" },
  { label: "旧编码残留", value: "legacy_encoded" },
  { label: "仅配置", value: "config_only" },
];

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

const columns: DataTableColumns<Project> = [
  {
    title: "状态", key: "state", width: 120,
    render: (p) => h(NSpace, { size: 4 }, () => [
      h(NTag, { size: "small", type: stateTagType(p.state) }, () => stateLabel(p.state)),
      p.running ? h(NTag, { size: "small", type: "info" }, () => "运行中") : null,
    ]),
  },
  {
    title: "项目路径", key: "path", ellipsis: { tooltip: true },
    render: (p) => p.real_path ? h("span", p.real_path) : h(NText, { depth: 3 }, () => `（数据目录）${p.encoded_dir}`),
  },
  { title: "最近活跃", key: "last_active_ms", width: 120, render: (p) => formatRelative(p.last_active_ms) },
  { title: "空间", key: "size", width: 100, render: (p) => (p.size ? formatBytes(p.size.total_bytes) : "—") },
  { title: "token（现存转录）", key: "usage", width: 150, render: (p) => (p.usage ? formatTokens(p.usage.input + p.usage.output + p.usage.cache_creation + p.usage.cache_read) : "—") },
  { title: "会话数", key: "sessions", width: 80, render: (p) => p.usage?.session_count ?? "—" },
];

const selected = computed(() => rows.value.find((p) => p.id === selectedId.value) ?? store.overview?.projects.find((p) => p.id === selectedId.value) ?? null);
const rowProps = (p: Project) => ({ style: "cursor: pointer", onClick: () => (selectedId.value = p.id) });
</script>

<template>
  <div class="projects">
    <div class="list">
      <NSpace align="center" style="margin-bottom: 8px" wrap>
        <NSelect v-model:value="filter" :options="filterOptions" size="small" style="width: 180px" />
        <NSelect v-model:value="sortBy" size="small" style="width: 150px" :options="[{ label: '按最近活跃', value: 'active' }, { label: '按空间占用', value: 'size' }]" />
        <template v-if="filter === 'stale_big'">
          <NText depth="3">未用超过</NText><NInputNumber v-model:value="staleDays" size="small" :min="1" style="width: 90px" /><NText depth="3">天，且 ≥</NText>
          <NInputNumber v-model:value="bigMb" size="small" :min="1" style="width: 100px" /><NText depth="3">MB</NText>
        </template>
        <NText depth="3">共 {{ rows.length }} 项</NText>
      </NSpace>
      <NDataTable :columns="columns" :data="rows" :row-key="(p: Project) => p.id" :row-props="rowProps" size="small" :max-height="'calc(100vh - 260px)'" :loading="store.loading" />
    </div>
    <div class="detail">
      <ProjectDetail v-if="selected" :project="selected" @error="(m: string) => message.error(m)" />
      <NText v-else depth="3">点击左侧项目查看详情</NText>
    </div>
  </div>
</template>

<style scoped>
.projects { display: grid; grid-template-columns: minmax(0, 1fr) 420px; gap: 16px; padding-top: 8px; }
.detail { border-left: 1px solid rgba(128, 128, 128, 0.25); padding-left: 16px; overflow: auto; max-height: calc(100vh - 200px); }
</style>
