<script setup lang="ts">
import { computed } from "vue";
import { NDescriptions, NDescriptionsItem, NTable, NTag, NButton, NDivider, NText, NAlert } from "naive-ui";
import { api } from "../api";
import { useOverviewStore } from "../stores/overview";
import { verifiedLabel } from "../labels";
import { formatDateTime } from "../utils/format";

const store = useOverviewStore();
const ov = computed(() => store.overview);
</script>

<template>
  <div v-if="ov" style="padding-top: 8px">
    <NDescriptions :column="1" size="small" bordered label-placement="left">
      <NDescriptionsItem label="数据根目录">{{ ov.root.root }}（{{ ov.root.source === "env_var" ? "来自 CLAUDE_CONFIG_DIR" : "默认位置" }}）</NDescriptionsItem>
      <NDescriptionsItem label="配置文件">{{ ov.root.config_file }} <NText v-if="ov.config_error" type="error">— {{ ov.config_error }}</NText></NDescriptionsItem>
      <NDescriptionsItem label="claude CLI">
        <template v-if="ov.cli">{{ ov.cli.path }}（{{ ov.cli.kind }}，{{ ov.cli.version ?? "版本未知" }}）</template>
        <NText v-else type="error">未找到。删除功能（后续版本）将不可用。</NText>
      </NDescriptionsItem>
      <NDescriptionsItem label="工具自身数据目录">{{ ov.tool_data_dir }} <NButton size="tiny" @click="api.openToolDataDir()">打开</NButton></NDescriptionsItem>
      <NDescriptionsItem label="视图生成时间">{{ formatDateTime(ov.generated_at_ms) }}</NDescriptionsItem>
    </NDescriptions>

    <NDivider title-placement="left">编码自校验</NDivider>
    <NAlert :type="ov.self_check.migration_enabled ? 'success' : 'warning'" :show-icon="false">
      配置项 {{ ov.self_check.total_entries }} 条；当前规则匹配到数据目录 {{ ov.self_check.matched_by_current_rule }} 个；旧规则匹配 {{ ov.self_check.matched_by_legacy_rule }} 个；
      无法解释的目录 {{ ov.self_check.unmatched.length }} 个。
      <div v-if="!ov.self_check.migration_enabled" style="margin-top: 4px">存在无法解释的目录，迁移功能（后续版本）将被禁用，以防官方更改编码规则后挪错数据。</div>
      <ul v-if="ov.self_check.unmatched.length" style="margin: 4px 0 0"><li v-for="d in ov.self_check.unmatched" :key="d"><NText code>{{ d }}</NText></li></ul>
    </NAlert>

    <NDivider title-placement="left">sessions/ 会话文件（{{ ov.sessions.length }}）</NDivider>
    <NTable size="small" :single-line="false">
      <thead><tr><th>PID</th><th>判定</th><th>cwd</th><th>状态字段</th><th>启动时间</th></tr></thead>
      <tbody>
        <tr v-for="s in ov.sessions" :key="s.pid">
          <td>{{ s.pid }}</td>
          <td><NTag size="small" :type="s.verified === 'alive' ? 'success' : s.verified === 'stale' ? 'default' : 'warning'">{{ verifiedLabel(s.verified) }}</NTag></td>
          <td style="word-break: break-all">{{ s.cwd }}</td>
          <td>{{ s.status }}</td>
          <td>{{ formatDateTime(s.started_at_ms) }}</td>
        </tr>
      </tbody>
    </NTable>
    <NText depth="3" style="font-size: 12px">僵尸文件 = PID 已退出或被其他程序复用。工具不会删除它们（sessions/ 受保护）。</NText>
  </div>
</template>
