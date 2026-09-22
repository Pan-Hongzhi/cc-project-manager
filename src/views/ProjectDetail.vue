<script setup lang="ts">
import { computed } from "vue";
import { NButton, NSpace, NTag, NDescriptions, NDescriptionsItem, NTable, NAlert, NText, NDivider } from "naive-ui";
import { api, type Project } from "../api";
import { useOverviewStore } from "../stores/overview";
import { stateLabel, stateTagType, categoryLabel, retentionLabel } from "../labels";
import { formatBytes, formatDateTime, formatTokens, categoryKey } from "../utils/format";

const props = defineProps<{ project: Project }>();
const emit = defineEmits<{ (e: "error", msg: string): void }>();
const store = useOverviewStore();

const p = computed(() => props.project);
const canRun = computed(() => p.value.state === "normal" || p.value.state === "config_only");
const canOpenData = computed(() => p.value.encoded_dir !== null);
const dataDir = computed(() => (p.value.encoded_dir ? `${store.overview?.root.root}\\projects\\${p.value.encoded_dir}` : null));
const metaOf = (key: string) => store.overview?.category_meta.find((m) => categoryKey(m.category) === key);

async function guard(fn: () => Promise<void>) {
  try { await fn(); } catch (e) { emit("error", String(e)); }
}
</script>

<template>
  <div>
    <NSpace align="center">
      <NTag :type="stateTagType(p.state)">{{ stateLabel(p.state) }}</NTag>
      <NTag v-if="p.running" type="info">运行中 PID {{ p.running.pid }}</NTag>
    </NSpace>
    <h3 style="word-break: break-all; margin: 8px 0">{{ p.real_path ?? p.encoded_dir }}</h3>
    <NAlert v-if="p.state === 'legacy_encoded'" type="info" style="margin-bottom: 8px">
      此目录按旧版编码规则对应项目 <NText code>{{ p.legacy_of }}</NText>，CC 当前不再读取它。第一版只展示，不提供合并或删除。
    </NAlert>
    <NAlert v-if="p.state === 'orphan'" type="error" style="margin-bottom: 8px">项目路径已不存在。迁移/重绑定功能将在后续版本提供。</NAlert>

    <NSpace style="margin-bottom: 12px" wrap>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.runClaude(p.real_path!, false))">新会话</NButton>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.runClaude(p.real_path!, true))">恢复会话</NButton>
      <NButton size="small" :disabled="!canRun" @click="guard(() => api.openPath(p.real_path!))">打开项目路径</NButton>
      <NButton size="small" :disabled="!canOpenData" @click="guard(() => api.openPath(dataDir!))">打开数据目录</NButton>
    </NSpace>

    <NDescriptions :column="2" size="small" bordered label-placement="left">
      <NDescriptionsItem label="最近活跃">{{ formatDateTime(p.last_active_ms) }}</NDescriptionsItem>
      <NDescriptionsItem label="会话数">{{ p.usage?.session_count ?? "—" }}</NDescriptionsItem>
      <NDescriptionsItem label="数据目录">{{ p.encoded_dir ?? "无" }}</NDescriptionsItem>
      <NDescriptionsItem label="信任状态">{{ p.config_hint ? (p.config_hint.trust_accepted ? "已信任" : "未信任") : "无配置项" }}</NDescriptionsItem>
    </NDescriptions>

    <NDivider title-placement="left">空间占用</NDivider>
    <NTable v-if="p.size" size="small" :single-line="false">
      <thead><tr><th>类别</th><th>大小</th><th>文件</th><th>清扫策略</th><th>删除后果</th></tr></thead>
      <tbody>
        <tr v-for="e in p.size.by_category" :key="categoryKey(e.category)">
          <td>{{ categoryLabel(e.category) }}</td>
          <td>{{ formatBytes(e.bytes) }}</td>
          <td>{{ e.file_count }}</td>
          <td>{{ metaOf(categoryKey(e.category)) ? retentionLabel(metaOf(categoryKey(e.category))!.retention) : "未知" }}</td>
          <td>{{ metaOf(categoryKey(e.category))?.consequence ?? "未识别数据，只展示大小" }}</td>
        </tr>
      </tbody>
    </NTable>
    <NText v-else depth="3">尚未扫描</NText>
    <NAlert type="warning" style="margin-top: 8px" :show-icon="false">
      转录为明文 JSONL：命令输出中的密钥、`.env` 内容会原样落盘。分类删除将在后续版本提供。
    </NAlert>

    <NDivider title-placement="left">token（现存转录）</NDivider>
    <NDescriptions v-if="p.usage" :column="2" size="small" bordered label-placement="left">
      <NDescriptionsItem label="输入">{{ formatTokens(p.usage.input) }}</NDescriptionsItem>
      <NDescriptionsItem label="输出">{{ formatTokens(p.usage.output) }}</NDescriptionsItem>
      <NDescriptionsItem label="缓存写入">{{ formatTokens(p.usage.cache_creation) }}</NDescriptionsItem>
      <NDescriptionsItem label="缓存读取">{{ formatTokens(p.usage.cache_read) }}</NDescriptionsItem>
      <NDescriptionsItem label="消息数">{{ p.usage.message_count }}</NDescriptionsItem>
      <NDescriptionsItem label="跳过的坏行">{{ p.usage.skipped_lines }}</NDescriptionsItem>
    </NDescriptions>
    <NText depth="3" style="font-size: 12px; display: block; margin-top: 4px">
      口径：现存转录的用量（转录受 {{ store.overview?.scan?.cleanup_days ?? 30 }} 天清扫），不是历史总量；全局总量见「全局」页。
    </NText>

    <NDivider title-placement="left">记忆</NDivider>
    <NDescriptions :column="1" size="small" bordered label-placement="left">
      <NDescriptionsItem label="自动记忆（CC 写）">
        {{ p.memory_summary ? `${p.memory_summary.file_count} 个文件，MEMORY.md ${p.memory_summary.memory_md_lines ?? 0} 行` : "—" }}
      </NDescriptionsItem>
      <NDescriptionsItem label="用户记忆（随项目文件夹）">
        <template v-if="p.state === 'normal' && p.memory_summary">
          CLAUDE.md {{ p.memory_summary.user_claude_md ? "✓" : "✗" }} ·
          CLAUDE.local.md {{ p.memory_summary.user_claude_local_md ? "✓" : "✗" }} ·
          .claude/rules/ {{ p.memory_summary.user_rules_dir ? "✓" : "✗" }}
        </template>
        <template v-else>项目路径不可用，无法检查</template>
      </NDescriptionsItem>
    </NDescriptions>
  </div>
</template>
