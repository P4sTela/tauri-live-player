import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import type { Project, Cue, MediaItem, OutputTarget } from "../types";

interface ProjectStore {
  project: Project | null;
  isDirty: boolean;
  projectPath: string | null;

  // Project actions
  loadProject: (path: string) => Promise<void>;
  saveProject: (path?: string) => Promise<void>;
  newProject: (name: string) => Promise<void>;
  setProject: (project: Project) => void;

  // Cue actions
  addCue: (cue: Cue) => void;
  updateCue: (id: string, updates: Partial<Cue>) => void;
  removeCue: (id: string) => void;
  reorderCues: (fromIndex: number, toIndex: number) => void;

  // Item actions
  addItemToCue: (cueId: string, item: MediaItem) => void;
  updateItem: (
    cueId: string,
    itemId: string,
    updates: Partial<MediaItem>,
  ) => void;
  removeItem: (cueId: string, itemId: string) => void;

  // Output actions
  addOutput: (output: OutputTarget) => void;
  updateOutput: (id: string, updates: Partial<OutputTarget>) => void;
  removeOutput: (id: string) => void;

  // Brightness
  setMasterBrightness: (value: number) => void;

  // Volume
  setMasterVolume: (value: number) => void;
}

// Helper to sync project to Rust backend
const syncToBackend = (project: Project | null) => {
  if (project) {
    invoke("update_project", { project }).catch((e) => {
      console.error("Failed to sync project to backend:", e);
    });
  }
};

export const useProjectStore = create<ProjectStore>()(
  subscribeWithSelector((set, get) => ({
    project: null,
    isDirty: false,
    projectPath: null,

    loadProject: async (path) => {
      const project = await invoke<Project>("load_project", { path });
      set({ project, isDirty: false, projectPath: path });
      // Remember last opened project
      localStorage.setItem("lastProjectPath", path);
    },

    saveProject: async (path) => {
      const { project, projectPath } = get();
      if (!project) return;
      const savePath = path || projectPath;
      await invoke("save_project", { path: savePath });
      set({ isDirty: false, projectPath: savePath || null });
      // Remember last saved project
      if (savePath) {
        localStorage.setItem("lastProjectPath", savePath);
      }
    },

    newProject: async (name) => {
      const project = await invoke<Project>("new_project", { name });
      set({ project, isDirty: false, projectPath: null });
    },

    setProject: (project) => {
      set({ project });
      syncToBackend(project);
    },

    addCue: (cue) => {
      set((state) => {
        const newProject = state.project
          ? { ...state.project, cues: [...state.project.cues, cue] }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    updateCue: (id, updates) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              cues: state.project.cues.map((c) =>
                c.id === id ? { ...c, ...updates } : c,
              ),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    removeCue: (id) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              cues: state.project.cues.filter((c) => c.id !== id),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    reorderCues: (fromIndex, toIndex) => {
      set((state) => {
        if (!state.project) return state;
        const cues = [...state.project.cues];
        const [removed] = cues.splice(fromIndex, 1);
        cues.splice(toIndex, 0, removed);
        const newProject = { ...state.project, cues };
        syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    addItemToCue: (cueId, item) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              cues: state.project.cues.map((c) =>
                c.id === cueId ? { ...c, items: [...c.items, item] } : c,
              ),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    updateItem: (cueId, itemId, updates) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              cues: state.project.cues.map((c) =>
                c.id === cueId
                  ? {
                      ...c,
                      items: c.items.map((i) =>
                        i.id === itemId ? { ...i, ...updates } : i,
                      ),
                    }
                  : c,
              ),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    removeItem: (cueId, itemId) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              cues: state.project.cues.map((c) =>
                c.id === cueId
                  ? { ...c, items: c.items.filter((i) => i.id !== itemId) }
                  : c,
              ),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    addOutput: (output) => {
      set((state) => {
        const newProject = state.project
          ? { ...state.project, outputs: [...state.project.outputs, output] }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    updateOutput: (id, updates) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              outputs: state.project.outputs.map((o) =>
                o.id === id ? { ...o, ...updates } : o,
              ),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    removeOutput: (id) => {
      set((state) => {
        const newProject = state.project
          ? {
              ...state.project,
              outputs: state.project.outputs.filter((o) => o.id !== id),
            }
          : null;
        if (newProject) syncToBackend(newProject);
        return { project: newProject, isDirty: true };
      });
    },

    setMasterBrightness: (value) => {
      invoke("set_master_brightness", { value }).catch(console.error);
      set((state) => {
        const newProject = state.project
          ? { ...state.project, masterBrightness: value }
          : null;
        // Note: set_master_brightness already updates backend, no need to sync full project
        return { project: newProject };
      });
    },

    setMasterVolume: (value) => {
      invoke("set_master_volume", { value }).catch(console.error);
      set((state) => {
        const newProject = state.project
          ? { ...state.project, masterVolume: value }
          : null;
        return { project: newProject };
      });
    },
  })),
);
