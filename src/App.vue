<script setup lang="ts">
import { onMounted, ref } from "vue";
import { NConfigProvider, NMessageProvider, NTabs, NTabPane, NAlert, darkTheme, zhCN, dateZhCN, useOsTheme } from "naive-ui";
import StatusBar from "./components/StatusBar.vue";
import ProjectsView from "./views/ProjectsView.vue";
import GlobalView from "./views/GlobalView.vue";
import DiagnosticsView from "./views/DiagnosticsView.vue";
import { useOverviewStore } from "./stores/overview";

const store = useOverviewStore();
const osTheme = useOsTheme();
const tab = ref("projects");

onMounted(async () => {
  await store.bindEvents();
  await store.load();
  if (!store.overview?.scan) await store.refresh(); // 首次启动没有缓存：自动扫一次
});
</script>

<template>
  <NConfigProvider :theme="osTheme === 'dark' ? darkTheme : null" :locale="zhCN" :date-locale="dateZhCN">
    <NMessageProvider>
      <div class="app">
        <StatusBar />
        <NAlert v-if="store.error" type="error" closable style="margin: 8px 16px" @close="store.error = ''">{{ store.error }}</NAlert>
        <NAlert v-if="store.overview?.config_error" type="warning" style="margin: 8px 16px" title="配置文件不可用">
          {{ store.overview.config_error }}。当前只能显示无主数据目录。
        </NAlert>
        <NTabs v-model:value="tab" type="line" animated style="padding: 0 16px">
          <NTabPane name="projects" tab="项目"><ProjectsView /></NTabPane>
          <NTabPane name="global" tab="全局"><GlobalView /></NTabPane>
          <NTabPane name="diagnostics" tab="诊断"><DiagnosticsView /></NTabPane>
        </NTabs>
      </div>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
html, body, #app { margin: 0; height: 100%; font-family: system-ui, "Microsoft YaHei", sans-serif; }
.app { display: flex; flex-direction: column; height: 100%; }
</style>
