<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type Overview } from "./api";

const overview = ref<Overview | null>(null);
const error = ref("");
onMounted(async () => {
  try {
    overview.value = await api.getOverview();
  } catch (e) {
    error.value = String(e);
  }
});
</script>

<template>
  <main style="font-family: system-ui; padding: 16px">
    <h1>CC Project Manager</h1>
    <p v-if="error" style="color: red">{{ error }}</p>
    <p v-else-if="!overview">加载中…</p>
    <template v-else>
      <p>数据根：{{ overview.root.root }}（{{ overview.root.source }}）</p>
      <p>项目数：{{ overview.projects.length }}，会话文件：{{ overview.sessions.length }}</p>
      <pre style="max-height: 60vh; overflow: auto; font-size: 12px">{{ JSON.stringify(overview.self_check, null, 2) }}</pre>
    </template>
  </main>
</template>
