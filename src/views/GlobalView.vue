<script setup lang="ts">
import { computed } from "vue";
import { NTable, NStatistic, NSpace, NButton, NAlert, NText, NDivider, NGrid, NGi, useMessage } from "naive-ui";
import { api } from "../api";
import { useOverviewStore } from "../stores/overview";
import { categoryLabel, retentionLabel } from "../labels";
import { formatBytes, formatTokens, categoryKey } from "../utils/format";

const store = useOverviewStore();
const message = useMessage();
const ov = computed(() => store.overview);
const scan = computed(() => ov.value?.scan ?? null);
const globalSorted = computed(() => [...(scan.value?.global ?? [])].sort((a, b) => b.bytes - a.bytes));
const metaOf = (key: string) => ov.value?.category_meta.find((m) => categoryKey(m.category) === key);
const retentionText = (key: string, kind: string) => {
  const m = metaOf(key);
  if (m) return retentionLabel(m.retention);
  if (kind === "protected") return "受保护，工具不触碰";
  if (kind === "legacy") return "清扫时移除";
  return "未知";
};
async function openInsights() {
  try { await api.openInsights(); } catch (e) { message.error(String(e)); }
}
async function openDataRoot() {
  try { await api.openPath(ov.value!.root.root); } catch (e) { message.error(String(e)); }
}
</script>

<template>
  <div style="padding-top: 8px">
    <NGrid :cols="4" :x-gap="16">
      <NGi><NStatistic label="数据根合计" :value="scan ? formatBytes(scan.root_total_bytes) : '—'" /></NGi>
      <NGi><NStatistic label="项目数" :value="ov?.projects.length ?? 0" /></NGi>
      <NGi>
        <NStatistic label="清扫周期" :value="scan ? `${scan.cleanup_days} 天` : '—'">
          <template #suffix><NText depth="3" style="font-size: 12px">{{ scan?.cleanup_days_source === "explicit" ? "（settings.json 显式设置）" : "（默认值）" }}</NText></template>
        </NStatistic>
      </NGi>
      <NGi><NStatistic label="无法访问的条目" :value="scan?.inaccessible ?? 0" /></NGi>
    </NGrid>

    <NDivider title-placement="left">按类别</NDivider>
    <NTable size="small" :single-line="false">
      <thead><tr><th>类别</th><th>大小</th><th>文件数</th><th>清扫策略</th><th>说明</th></tr></thead>
      <tbody>
        <tr v-for="e in globalSorted" :key="categoryKey(e.category)">
          <td>{{ categoryLabel(e.category) }}</td>
          <td>{{ formatBytes(e.bytes) }}</td>
          <td>{{ e.file_count }}</td>
          <td>{{ retentionText(categoryKey(e.category), e.category.kind) }}</td>
          <td>{{ metaOf(categoryKey(e.category))?.consequence ?? (e.category.kind === "protected" ? "凭据、配置、插件或运行时状态" : "未识别数据，只展示大小") }}</td>
        </tr>
      </tbody>
    </NTable>

    <NDivider title-placement="left">下次清扫模拟</NDivider>
    <NAlert v-if="ov?.cleanup_preview" type="info" :show-icon="false">
      按 {{ ov.cleanup_preview.cleanup_days }} 天口径，下次清扫将删除 {{ ov.cleanup_preview.files }} 个文件，释放 {{ formatBytes(ov.cleanup_preview.bytes) }}：
      <span v-for="e in ov.cleanup_preview.by_category" :key="categoryKey(e.category)" style="margin-right: 12px">{{ categoryLabel(e.category) }} {{ formatBytes(e.bytes) }}</span>
    </NAlert>
    <NText v-else depth="3">重新扫描后可用</NText>

    <NDivider title-placement="left">token 总量（stats-cache.json，/usage 口径）</NDivider>
    <template v-if="ov?.stats.available">
      <NSpace style="margin-bottom: 8px">
        <NStatistic label="会话总数" :value="ov.stats.total_sessions" />
        <NStatistic label="消息总数" :value="ov.stats.total_messages" />
        <NStatistic label="统计截至" :value="ov.stats.last_computed_date ?? '—'" />
      </NSpace>
      <NTable size="small" :single-line="false">
        <thead><tr><th>模型</th><th>输入</th><th>输出</th><th>缓存写入</th><th>缓存读取</th></tr></thead>
        <tbody>
          <tr v-for="m in ov.stats.models" :key="m.model">
            <td>{{ m.model }}</td><td>{{ formatTokens(m.input) }}</td><td>{{ formatTokens(m.output) }}</td>
            <td>{{ formatTokens(m.cache_creation) }}</td><td>{{ formatTokens(m.cache_read) }}</td>
          </tr>
        </tbody>
      </NTable>
    </template>
    <NAlert v-else type="warning">{{ ov?.stats.reason ?? "stats-cache.json 不可用" }}</NAlert>
    <NText depth="3" style="font-size: 12px; display: block; margin: 4px 0 8px">第一版只展示 token 数，不换算金额。</NText>
    <NSpace>
      <NButton size="small" @click="openInsights">打开 /insights 报告</NButton>
      <NButton size="small" @click="openDataRoot" :disabled="!ov">打开数据根目录</NButton>
    </NSpace>
  </div>
</template>
