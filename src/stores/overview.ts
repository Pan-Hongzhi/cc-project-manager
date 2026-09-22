import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, type Overview, type Project, type ScanProgress } from "../api";

let unlisteners: UnlistenFn[] = [];

export const useOverviewStore = defineStore("overview", {
  state: () => ({
    overview: null as Overview | null,
    loading: false,
    scanning: false,
    progress: null as ScanProgress | null,
    error: "" as string,
  }),
  getters: {
    projectsSorted(state): Project[] {
      const list = state.overview?.projects ?? [];
      return [...list].sort((a, b) => (b.last_active_ms ?? 0) - (a.last_active_ms ?? 0));
    },
  },
  actions: {
    async bindEvents() {
      if (unlisteners.length) return;
      unlisteners.push(
        await listen<ScanProgress>("scan-progress", (e) => {
          this.progress = e.payload;
        }),
        await listen<Overview>("overview", (e) => {
          this.overview = e.payload;
          this.scanning = false;
          this.progress = null;
        }),
      );
    },
    async load() {
      this.loading = true;
      this.error = "";
      try {
        this.overview = await api.getOverview();
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async refresh() {
      if (this.scanning) return;
      this.scanning = true;
      this.error = "";
      try {
        await api.refresh();
      } catch (e) {
        this.scanning = false;
        this.error = String(e);
      }
    },
  },
});
