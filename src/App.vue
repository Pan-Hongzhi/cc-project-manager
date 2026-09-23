<script setup lang="ts">
import { onMounted, ref } from "vue";
import { NConfigProvider, NMessageProvider, NTabs, NTabPane, NAlert, darkTheme, zhCN, dateZhCN } from "naive-ui";
import StatusBar from "./components/StatusBar.vue";
import ProjectsView from "./views/ProjectsView.vue";
import GlobalView from "./views/GlobalView.vue";
import DiagnosticsView from "./views/DiagnosticsView.vue";
import { useOverviewStore } from "./stores/overview";
import { themeOverrides } from "./theme";

const store = useOverviewStore();
const tab = ref("projects");

onMounted(async () => {
  await store.bindEvents();
  await store.load();
  if (!store.overview?.scan) await store.refresh(); // 首次启动没有缓存：自动扫一次
});
</script>

<template>
  <!-- 与图标一致：始终使用深色「终端」主题 -->
  <NConfigProvider :theme="darkTheme" :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN">
    <NMessageProvider>
      <div class="app">
        <StatusBar />
        <NAlert v-if="store.error" type="error" closable class="banner" @close="store.error = ''">{{ store.error }}</NAlert>
        <NAlert v-if="store.overview?.config_error" type="warning" class="banner" title="配置文件不可用">
          {{ store.overview.config_error }}。当前只能显示无主数据目录。
        </NAlert>
        <NTabs v-model:value="tab" type="line" animated class="tabs">
          <NTabPane name="projects" tab="项目"><ProjectsView /></NTabPane>
          <NTabPane name="global" tab="全局"><GlobalView /></NTabPane>
          <NTabPane name="diagnostics" tab="诊断"><DiagnosticsView /></NTabPane>
        </NTabs>
      </div>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
:root {
  --ink: #0c0c0f;
  --panel: #131316;
  --panel-raised: #18181b;
  --chrome: #27272a;
  --chrome-raised: #3f3f46;
  --edge: #52525b;
  --muted: #71717a;
  --text: #e4e4e7;
  --coral: #e07a5f;
  --green: #22c55e;
  --font-ui: "Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", system-ui, sans-serif;
  --font-mono: "Cascadia Code", "Cascadia Mono", Consolas, "Courier New", monospace;
}

html, body, #app { margin: 0; height: 100%; overflow: hidden; } /* 页面本身不滚动，只有表格与标签页内容区滚动 */
body {
  color: var(--text);
  font-family: var(--font-ui);
  /* 终端底色 + 极淡的点阵网格，像一张暗色的图纸 */
  background-color: var(--ink);
  background-image:
    radial-gradient(ellipse 60% 40% at 50% -10%, rgba(224, 122, 95, 0.10), transparent 70%),
    radial-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px);
  background-size: 100% 100%, 22px 22px;
  background-attachment: fixed;
}
::selection { background: rgba(224, 122, 95, 0.45); color: #fff; }

.app { display: flex; flex-direction: column; height: 100%; }
.banner { margin: 8px 16px; }
/* 标签页填满剩余高度：窗口放大时表格跟着长高，不留底部空白 */
.tabs { padding: 0 16px; flex: 1; display: flex; flex-direction: column; min-height: 0; }
.tabs > .n-tabs-pane-wrapper { flex: 1; min-height: 0; overflow: auto; }

/* 等宽字体：路径、数字、编码目录名 */
.mono, .n-data-table .mono, code, .n-text.n-text--code {
  font-family: var(--font-mono);
  font-variant-ligatures: none;
}
.n-text.n-text--code {
  background: var(--panel-raised) !important;
  border: 1px solid var(--chrome) !important;
  color: var(--text) !important;
}

/* 表格行首次出现时逐行淡入，像终端逐行输出 */
@keyframes row-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: none; }
}
.n-data-table .n-data-table-tr { animation: row-in 0.28s ease-out both; }
.n-data-table .n-data-table-tr:nth-child(1)  { animation-delay: 0.02s; }
.n-data-table .n-data-table-tr:nth-child(2)  { animation-delay: 0.05s; }
.n-data-table .n-data-table-tr:nth-child(3)  { animation-delay: 0.08s; }
.n-data-table .n-data-table-tr:nth-child(4)  { animation-delay: 0.11s; }
.n-data-table .n-data-table-tr:nth-child(5)  { animation-delay: 0.14s; }
.n-data-table .n-data-table-tr:nth-child(6)  { animation-delay: 0.17s; }
.n-data-table .n-data-table-tr:nth-child(7)  { animation-delay: 0.20s; }
.n-data-table .n-data-table-tr:nth-child(8)  { animation-delay: 0.23s; }
.n-data-table .n-data-table-tr:nth-child(9)  { animation-delay: 0.26s; }
.n-data-table .n-data-table-tr:nth-child(10) { animation-delay: 0.29s; }
.n-data-table .n-data-table-tr:nth-child(n+11) { animation-delay: 0.32s; }

/* 选中行：左侧一道珊瑚橙光标条，呼应图标里的提示符 */
.n-data-table .n-data-table-tr.is-selected .n-data-table-td:first-child { box-shadow: inset 3px 0 0 var(--coral); }

/* 细滚动条 */
*::-webkit-scrollbar { width: 8px; height: 8px; }
*::-webkit-scrollbar-thumb { background: var(--chrome-raised); border-radius: 4px; }
*::-webkit-scrollbar-thumb:hover { background: var(--edge); }
*::-webkit-scrollbar-track { background: transparent; }
</style>
