import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { MonitorInfo, OutputTarget } from "../types";

interface OpenOutputInfo {
  outputId: string;
  monitorIndex: number;
}

interface OutputStore {
  // モニター情報
  monitors: MonitorInfo[];
  isLoadingMonitors: boolean;

  // 開いている出力ウィンドウ（IDとモニター情報のマップ）
  openOutputs: Map<string, OpenOutputInfo>;

  // アクション
  fetchMonitors: () => Promise<void>;
  openOutput: (
    output: OutputTarget,
    monitor: MonitorInfo | null,
  ) => Promise<void>;
  closeOutput: (id: string) => Promise<void>;
  closeAllOutputs: () => Promise<void>;
  isOutputOpen: (id: string) => boolean;
  getOutputMonitor: (id: string) => MonitorInfo | undefined;
}

export const useOutputStore = create<OutputStore>((set, get) => ({
  monitors: [],
  isLoadingMonitors: false,
  openOutputs: new Map(),

  fetchMonitors: async () => {
    set({ isLoadingMonitors: true });
    try {
      const monitors = await invoke<MonitorInfo[]>("get_monitors");
      set({ monitors, isLoadingMonitors: false });
    } catch (e) {
      console.error("Failed to fetch monitors:", e);
      set({ isLoadingMonitors: false });
    }
  },

  openOutput: async (output: OutputTarget, monitor: MonitorInfo | null) => {
    try {
      // モニター情報をバックエンドに渡す（nullの場合はウィンドウモード）
      await invoke("open_output_window", { config: output, monitor });
      set((state) => {
        const newMap = new Map(state.openOutputs);
        newMap.set(output.id, {
          outputId: output.id,
          monitorIndex: monitor?.index ?? -1, // -1 for windowed mode
        });
        return { openOutputs: newMap };
      });
    } catch (e) {
      console.error("Failed to open output:", e);
      throw e;
    }
  },

  closeOutput: async (id: string) => {
    try {
      await invoke("close_output_window", { id });
      set((state) => {
        const newMap = new Map(state.openOutputs);
        newMap.delete(id);
        return { openOutputs: newMap };
      });
    } catch (e) {
      console.error("Failed to close output:", e);
    }
  },

  closeAllOutputs: async () => {
    try {
      await invoke("close_all_outputs");
      set({ openOutputs: new Map() });
    } catch (e) {
      console.error("Failed to close all outputs:", e);
    }
  },

  isOutputOpen: (id: string) => {
    return get().openOutputs.has(id);
  },

  getOutputMonitor: (id: string) => {
    const info = get().openOutputs.get(id);
    if (!info) return undefined;
    return get().monitors.find((m) => m.index === info.monitorIndex);
  },
}));
