<script setup lang="ts">
import { computed } from "vue";
import { NButton, NSpace, NTag, NProgress, NText } from "naive-ui";
import { useOverviewStore } from "../stores/overview";
import { formatBytes, formatRelative } from "../utils/format";

const store = useOverviewStore();
const ov = computed(() => store.overview);
const pct = computed(() => (store.progress && store.progress.total > 0 ? Math.round((store.progress.done / store.progress.total) * 100) : 0));
</script>

<template>
  <div class="statusbar">
    <NSpace align="center" :size="16" wrap>
      <NText depth="3">数据根</NText>
      <NText code>{{ ov?.root.root ?? "…" }}</NText>
      <NTag size="small" :type="ov?.root.source === 'env_var' ? 'info' : 'default'">{{ ov?.root.source === "env_var" ? "CLAUDE_CONFIG_DIR" : "默认位置" }}</NTag>
      <NTag size="small" :type="ov?.cli ? 'success' : 'error'">{{ ov?.cli ? `CLI ${ov.cli.version ?? "版本未知"}` : "未找到 claude CLI" }}</NTag>
      <NTag size="small" :type="ov?.self_check.migration_enabled ? 'success' : 'warning'">{{ ov?.self_check.migration_enabled ? "编码自校验通过" : "编码自校验未通过" }}</NTag>
      <NText v-if="ov?.scan" depth="3">
        {{ ov.scan_is_cached ? "上次扫描" : "本次扫描" }} {{ formatRelative(ov.scan.scanned_at_ms) }} · 合计 {{ formatBytes(ov.scan.root_total_bytes) }}
      </NText>
      <NText v-else depth="3">尚未扫描</NText>
      <NButton size="small" type="primary" :loading="store.scanning" @click="store.refresh()">{{ store.scanning ? "扫描中…" : "重新扫描" }}</NButton>
    </NSpace>
    <NProgress v-if="store.scanning" type="line" :percentage="pct" :show-indicator="false" style="margin-top: 6px" />
    <NText v-if="store.progress" depth="3" style="font-size: 12px">正在扫描 {{ store.progress.current }}（{{ store.progress.done }}/{{ store.progress.total }}）</NText>
  </div>
</template>

<style scoped>
.statusbar { padding: 8px 16px; border-bottom: 1px solid var(--n-border-color, #e5e5e5); }
</style>
