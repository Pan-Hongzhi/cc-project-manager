<script setup lang="ts">
import { computed } from "vue";
import { NDescriptions, NDescriptionsItem, NTable, NTag, NButton, NDivider, NText, NAlert, useMessage } from "naive-ui";
import { api } from "../api";
import { useOverviewStore } from "../stores/overview";
import { verifiedLabel } from "../labels";
import { formatDateTime } from "../utils/format";
import { t, translateError } from "../i18n";

const store = useOverviewStore();
const message = useMessage();
const ov = computed(() => store.overview);
async function openToolDataDir() {
  try { await api.openToolDataDir(); } catch (e) { message.error(translateError(e)); }
}
</script>

<template>
  <div v-if="ov" style="padding: 8px 0 16px">
    <NDescriptions :column="1" size="small" bordered label-placement="left">
      <NDescriptionsItem :label="t('diag.root')"><span class="mono">{{ ov.root.root }}</span>（{{ ov.root.source === "env_var" ? t("diag.fromEnv") : t("diag.defaultLoc") }}）</NDescriptionsItem>
      <NDescriptionsItem :label="t('diag.configFile')"><span class="mono">{{ ov.root.config_file }}</span> <NText v-if="ov.config_error" type="error">— {{ ov.config_error }}</NText></NDescriptionsItem>
      <NDescriptionsItem :label="t('diag.cli')">
        <template v-if="ov.cli"><span class="mono">{{ ov.cli.path }}</span>（{{ ov.cli.kind }}，{{ ov.cli.version ?? t("diag.versionUnknown") }}）</template>
        <NText v-else type="error">{{ t("diag.cliMissing") }}</NText>
      </NDescriptionsItem>
      <NDescriptionsItem :label="t('diag.toolDir')"><span class="mono">{{ ov.tool_data_dir }}</span> <NButton size="tiny" @click="openToolDataDir">{{ t("diag.open") }}</NButton></NDescriptionsItem>
      <NDescriptionsItem :label="t('diag.generatedAt')">{{ formatDateTime(ov.generated_at_ms) }}</NDescriptionsItem>
    </NDescriptions>

    <NDivider title-placement="left">{{ t("diag.selfcheck") }}</NDivider>
    <NAlert :type="ov.self_check.migration_enabled ? 'success' : 'warning'" :show-icon="false">
      {{ t("diag.selfcheckSummary", { entries: ov.self_check.total_entries, cur: ov.self_check.matched_by_current_rule, legacy: ov.self_check.matched_by_legacy_rule, unmatched: ov.self_check.unmatched.length }) }}
      <div v-if="!ov.self_check.migration_enabled" style="margin-top: 4px">{{ t("diag.selfcheckDisabled") }}</div>
      <ul v-if="ov.self_check.unmatched.length" style="margin: 4px 0 0"><li v-for="d in ov.self_check.unmatched" :key="d"><NText code>{{ d }}</NText></li></ul>
    </NAlert>

    <NDivider title-placement="left">{{ t("diag.sessionsTitle", { n: ov.sessions.length }) }}</NDivider>
    <NTable size="small" :single-line="false">
      <thead><tr><th>{{ t("diag.col.pid") }}</th><th>{{ t("diag.col.verdict") }}</th><th>{{ t("diag.col.cwd") }}</th><th>{{ t("diag.col.status") }}</th><th>{{ t("diag.col.started") }}</th></tr></thead>
      <tbody>
        <tr v-for="s in ov.sessions" :key="s.pid">
          <td class="mono">{{ s.pid }}</td>
          <td><NTag size="small" :bordered="false" :type="s.verified === 'alive' ? 'success' : s.verified === 'stale' ? 'default' : 'warning'">{{ verifiedLabel(s.verified) }}</NTag></td>
          <td class="mono" style="word-break: break-all">{{ s.cwd }}</td>
          <td class="mono">{{ s.status }}</td>
          <td>{{ formatDateTime(s.started_at_ms) }}</td>
        </tr>
      </tbody>
    </NTable>
    <NText depth="3" style="font-size: 12px">{{ t("diag.staleNote") }}</NText>
  </div>
</template>
