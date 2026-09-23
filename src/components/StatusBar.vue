<script setup lang="ts">
import { computed } from "vue";
import { NButton, NTag, NProgress } from "naive-ui";
import Mascot from "./Mascot.vue";
import { useOverviewStore } from "../stores/overview";
import { formatBytes, formatRelative } from "../utils/format";

const store = useOverviewStore();
const ov = computed(() => store.overview);
const pct = computed(() => (store.progress && store.progress.total > 0 ? Math.round((store.progress.done / store.progress.total) * 100) : 0));
</script>

<template>
  <!-- 仿图标里的终端标题栏：三个圆点 + 像素小人 + 标题 + 光标 -->
  <header class="chrome">
    <div class="chrome-left">
      <span class="dots" aria-hidden="true"><i /><i /><i /></span>
      <Mascot :size="22" />
      <span class="title mono">cc-project-manager<span class="cursor" :class="{ busy: store.scanning }" /></span>
      <span class="root mono" :title="ov?.root.root">{{ ov?.root.root ?? "…" }}</span>
      <NTag size="small" round :bordered="false" :type="ov?.root.source === 'env_var' ? 'info' : 'default'">
        {{ ov?.root.source === "env_var" ? "CLAUDE_CONFIG_DIR" : "默认位置" }}
      </NTag>
    </div>
    <div class="chrome-right">
      <NTag size="small" round :bordered="false" :type="ov?.cli ? 'success' : 'error'">
        {{ ov?.cli ? `CLI ${ov.cli.version ?? "版本未知"}` : "未找到 claude CLI" }}
      </NTag>
      <NTag size="small" round :bordered="false" :type="ov?.self_check.migration_enabled ? 'success' : 'warning'">
        {{ ov?.self_check.migration_enabled ? "编码自校验通过" : "编码自校验未通过" }}
      </NTag>
      <span v-if="ov?.scan" class="scan-info">
        {{ ov.scan_is_cached ? "上次扫描" : "本次扫描" }} {{ formatRelative(ov.scan.scanned_at_ms) }} · 合计
        <b class="mono">{{ formatBytes(ov.scan.root_total_bytes) }}</b>
      </span>
      <span v-else class="scan-info">尚未扫描</span>
      <NButton size="small" type="primary" :loading="store.scanning" @click="store.refresh()">
        {{ store.scanning ? "扫描中…" : "重新扫描" }}
      </NButton>
    </div>
    <div v-if="store.scanning" class="progress">
      <NProgress type="line" :percentage="pct" :show-indicator="false" :height="3" :border-radius="0" />
      <span v-if="store.progress" class="progress-text mono">
        scanning {{ store.progress.current }} ({{ store.progress.done }}/{{ store.progress.total }})
      </span>
    </div>
  </header>
</template>

<style scoped>
.chrome {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  padding: 10px 16px;
  background: linear-gradient(180deg, var(--chrome-raised), var(--chrome));
  border-bottom: 1px solid var(--edge);
}
.chrome-left, .chrome-right { display: flex; align-items: center; gap: 12px; min-width: 0; flex-wrap: wrap; }
.chrome-left { flex: 1 1 420px; }
.chrome-right { flex: 0 1 auto; justify-content: flex-end; margin-left: auto; }
.dots { display: inline-flex; gap: 6px; margin-right: 2px; }
.dots i { width: 10px; height: 10px; border-radius: 50%; background: var(--muted); display: inline-block; }
.title { color: #fafafa; font-weight: 600; letter-spacing: 0.2px; display: inline-flex; align-items: center; }
.cursor {
  display: inline-block; width: 9px; height: 15px; margin-left: 6px; border-radius: 2px;
  background: var(--green); animation: blink 1.1s steps(1) infinite;
}
.cursor.busy { animation-duration: 0.35s; }
@keyframes blink { 50% { opacity: 0; } }
.root {
  color: var(--text); opacity: 0.8; font-size: 12px;
  max-width: 22vw; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  padding: 2px 8px; border-radius: 4px; background: rgba(0, 0, 0, 0.35); border: 1px solid var(--edge);
}
.scan-info { color: var(--text); opacity: 0.75; font-size: 12px; white-space: nowrap; }
.scan-info b { color: #fafafa; font-weight: 600; }
.progress { position: absolute; left: 0; right: 0; bottom: -1px; }
.progress-text { position: absolute; right: 16px; bottom: 6px; font-size: 11px; color: var(--muted); }
</style>
