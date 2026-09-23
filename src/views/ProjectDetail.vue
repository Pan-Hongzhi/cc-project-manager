<script setup lang="ts">
import { computed } from "vue";
import { NButton, NSpace, NTag, NDescriptions, NDescriptionsItem, NTable, NAlert, NText, NDivider } from "naive-ui";
import { api, type Project, type Retention } from "../api";
import { useOverviewStore } from "../stores/overview";
import { stateLabel, stateTagType, categoryLabel, retentionLabel, consequenceLabel } from "../labels";
import { formatBytes, formatDateTime, formatTokens, categoryKey } from "../utils/format";
import { t, translateError } from "../i18n";

const props = defineProps<{ project: Project }>();
const emit = defineEmits<{ (e: "error", msg: string): void }>();
const store = useOverviewStore();

const p = computed(() => props.project);
const canRun = computed(() => p.value.state === "normal" || p.value.state === "config_only");
const canOpenData = computed(() => p.value.encoded_dir !== null);
const dataDir = computed(() => (p.value.encoded_dir ? `${store.overview?.root.root}\\projects\\${p.value.encoded_dir}` : null));
const retentionOf = (key: string): Retention | null => store.overview?.category_meta.find((m) => categoryKey(m.category) === key)?.retention ?? null;

async function guard(fn: () => Promise<void>) {
  try { await fn(); } catch (e) { emit("error", translateError(e)); }
}
</script>

<template>
  <div>
    <NSpace align="center">
      <NTag :type="stateTagType(p.state)">{{ stateLabel(p.state) }}</NTag>
      <NTag v-if="p.running" type="success" :bordered="false">{{ t("detail.runningPid", { pid: p.running.pid }) }}</NTag>
    </NSpace>
    <h3 style="word-break: break-all; margin: 8px 0" class="mono">{{ p.real_path ?? p.encoded_dir }}</h3>
    <NAlert v-if="p.state === 'legacy_encoded'" type="info" style="margin-bottom: 8px">
      {{ t("detail.legacyNote1") }} <NText code>{{ p.legacy_of }}</NText>{{ t("detail.legacyNote2") }}
    </NAlert>
    <NAlert v-if="p.state === 'orphan'" type="error" style="margin-bottom: 8px">{{ t("detail.orphanNote") }}</NAlert>

    <NSpace style="margin-bottom: 12px" wrap>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.runClaude(p.real_path!, false))">{{ t("detail.btn.newSession") }}</NButton>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.runClaude(p.real_path!, true))">{{ t("detail.btn.resume") }}</NButton>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.openPath(p.real_path!))">{{ t("detail.btn.openPath") }}</NButton>
      <NButton size="small" :disabled="!canOpenData" @click="guard(() => api.openPath(dataDir!))">{{ t("detail.btn.openData") }}</NButton>
    </NSpace>

    <NDescriptions :column="2" size="small" bordered label-placement="left">
      <NDescriptionsItem :label="t('detail.lastActive')">{{ formatDateTime(p.last_active_ms) }}</NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.sessions')"><span class="mono">{{ p.usage?.session_count ?? "—" }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.dataDir')"><span class="mono">{{ p.encoded_dir ?? t("detail.none") }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.trust')">{{ p.config_hint ? (p.config_hint.trust_accepted ? t("detail.trusted") : t("detail.untrusted")) : t("detail.noConfig") }}</NDescriptionsItem>
    </NDescriptions>

    <NDivider title-placement="left">{{ t("detail.space") }}</NDivider>
    <NTable v-if="p.size" size="small" :single-line="false">
      <thead><tr><th>{{ t("detail.col.category") }}</th><th>{{ t("detail.col.size") }}</th><th>{{ t("detail.col.files") }}</th><th>{{ t("detail.col.policy") }}</th><th>{{ t("detail.col.consequence") }}</th></tr></thead>
      <tbody>
        <tr v-for="e in p.size.by_category" :key="categoryKey(e.category)">
          <td>{{ categoryLabel(e.category) }}</td>
          <td class="mono">{{ formatBytes(e.bytes) }}</td>
          <td class="mono">{{ e.file_count }}</td>
          <td>{{ retentionOf(categoryKey(e.category)) ? retentionLabel(retentionOf(categoryKey(e.category))!) : t("retention.unknown") }}</td>
          <td>{{ consequenceLabel(e.category) }}</td>
        </tr>
      </tbody>
    </NTable>
    <NText v-else depth="3">{{ t("detail.notScanned") }}</NText>
    <NAlert type="warning" style="margin-top: 8px" :show-icon="false">{{ t("detail.plaintext") }}</NAlert>

    <NDivider title-placement="left">{{ t("detail.tokensTitle") }}</NDivider>
    <NDescriptions v-if="p.usage" :column="2" size="small" bordered label-placement="left">
      <NDescriptionsItem :label="t('detail.input')"><span class="mono">{{ formatTokens(p.usage.input) }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.output')"><span class="mono">{{ formatTokens(p.usage.output) }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.cacheWrite')"><span class="mono">{{ formatTokens(p.usage.cache_creation) }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.cacheRead')"><span class="mono">{{ formatTokens(p.usage.cache_read) }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.messages')"><span class="mono">{{ p.usage.message_count }}</span></NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.skipped')"><span class="mono">{{ p.usage.skipped_lines }}</span></NDescriptionsItem>
    </NDescriptions>
    <NText depth="3" style="font-size: 12px; display: block; margin-top: 4px">
      {{ t("detail.tokenNote", { days: store.overview?.scan?.cleanup_days ?? 30 }) }}
    </NText>

    <NDivider title-placement="left">{{ t("detail.memory") }}</NDivider>
    <NDescriptions :column="1" size="small" bordered label-placement="left">
      <NDescriptionsItem :label="t('detail.autoMemory')">
        {{ p.memory_summary ? t("detail.autoMemoryValue", { files: p.memory_summary.file_count, lines: p.memory_summary.memory_md_lines ?? 0 }) : "—" }}
      </NDescriptionsItem>
      <NDescriptionsItem :label="t('detail.userMemory')">
        <template v-if="p.state === 'normal' && p.memory_summary">
          <span class="mono">CLAUDE.md {{ p.memory_summary.user_claude_md ? "✓" : "✗" }} ·
          CLAUDE.local.md {{ p.memory_summary.user_claude_local_md ? "✓" : "✗" }} ·
          .claude/rules/ {{ p.memory_summary.user_rules_dir ? "✓" : "✗" }}</span>
        </template>
        <template v-else>{{ t("detail.pathUnavailable") }}</template>
      </NDescriptionsItem>
    </NDescriptions>
  </div>
</template>
