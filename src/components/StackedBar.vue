<script setup lang="ts">
// 单条 100% 堆叠横条：段间 2px 底色间隙、两端 4px 圆角、宽度足够时段内直接标注，配合图例与悬浮提示。
import { computed } from "vue";
import { NTooltip } from "naive-ui";
import { formatBytes } from "../utils/format";

export interface Segment {
  key: string;
  label: string;
  value: number;
  color: string;
  /** 图例里的补充说明（如清扫策略） */
  note?: string;
  /** 使用斜纹纹理（用于「未知」等需要二次编码的段） */
  hatched?: boolean;
}

const props = withDefaults(defineProps<{ segments: Segment[]; height?: number; showLegend?: boolean }>(), {
  height: 22,
  showLegend: true,
});

const total = computed(() => props.segments.reduce((s, x) => s + x.value, 0));
const visible = computed(() => props.segments.filter((s) => s.value > 0));
const pct = (v: number) => (total.value > 0 ? (v / total.value) * 100 : 0);
const pctText = (v: number) => {
  const p = pct(v);
  return p >= 10 ? `${p.toFixed(0)}%` : p >= 1 ? `${p.toFixed(1)}%` : "<1%";
};
</script>

<template>
  <div class="stacked">
    <div class="bar" :style="{ height: height + 'px' }" role="img" :aria-label="visible.map((s) => `${s.label} ${formatBytes(s.value)}`).join('，')">
      <NTooltip v-for="s in visible" :key="s.key" trigger="hover" placement="top">
        <template #trigger>
          <div class="seg" :class="{ hatched: s.hatched }" :style="{ width: pct(s.value) + '%', '--seg': s.color }">
            <span v-if="pct(s.value) >= 9" class="seg-label mono">{{ pctText(s.value) }}</span>
          </div>
        </template>
        <div class="tip">
          <b>{{ s.label }}</b>
          <span class="mono">{{ formatBytes(s.value) }} · {{ pctText(s.value) }}</span>
          <span v-if="s.note" class="tip-note">{{ s.note }}</span>
        </div>
      </NTooltip>
    </div>
    <ul v-if="showLegend" class="legend">
      <li v-for="s in visible" :key="s.key">
        <i class="swatch" :class="{ hatched: s.hatched }" :style="{ '--seg': s.color }" />
        <span class="lg-label">{{ s.label }}</span>
        <span class="lg-value mono">{{ formatBytes(s.value) }}</span>
        <span class="lg-pct mono">{{ pctText(s.value) }}</span>
        <span v-if="s.note" class="lg-note">{{ s.note }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  width: 100%;
  border-radius: 4px;
  overflow: hidden;
  background: var(--chrome);
  gap: 2px; /* 段间 2px 底色间隙 */
}
.seg {
  position: relative;
  background: var(--seg);
  min-width: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: filter 0.15s ease;
}
.seg:hover { filter: brightness(1.15); }
.seg.hatched, .swatch.hatched {
  background-image: repeating-linear-gradient(135deg, transparent 0 4px, rgba(0, 0, 0, 0.35) 4px 6px);
}
.seg-label { font-size: 11px; color: rgba(255, 255, 255, 0.92); text-shadow: 0 1px 1px rgba(0, 0, 0, 0.5); }
.legend { list-style: none; margin: 10px 0 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 6px 20px; }
.legend li { display: grid; grid-template-columns: 10px 1fr auto auto; align-items: center; column-gap: 8px; font-size: 12px; color: var(--text); }
.legend li .lg-note { grid-column: 2 / -1; color: var(--muted); font-size: 11px; }
.swatch { width: 10px; height: 10px; border-radius: 2px; background: var(--seg); display: inline-block; }
.lg-value { color: var(--text); }
.lg-pct { color: var(--muted); min-width: 40px; text-align: right; }
.tip { display: flex; flex-direction: column; gap: 2px; font-size: 12px; }
.tip-note { color: var(--muted); }
</style>
