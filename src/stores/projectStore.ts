import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Project, Cue, MediaItem, OutputTarget } from '../types';

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
  updateItem: (cueId: string, itemId: string, updates: Partial<MediaItem>) => void;
  removeItem: (cueId: string, itemId: string) => void;

  // Output actions
  addOutput: (output: OutputTarget) => void;
  updateOutput: (id: string, updates: Partial<OutputTarget>) => void;
  removeOutput: (id: string) => void;

  // Brightness
  setMasterBrightness: (value: number) => void;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  project: null,
  isDirty: false,
  projectPath: null,

  loadProject: async (path) => {
    const project = await invoke<Project>('load_project', { path });
    set({ project, isDirty: false, projectPath: path });
  },

  saveProject: async (path) => {
    const { project, projectPath } = get();
    if (!project) return;
    const savePath = path || projectPath;
    await invoke('save_project', { path: savePath });
    set({ isDirty: false, projectPath: savePath || null });
  },

  newProject: async (name) => {
    const project = await invoke<Project>('new_project', { name });
    set({ project, isDirty: false, projectPath: null });
  },

  setProject: (project) => {
    set({ project });
  },

  addCue: (cue) => {
    set((state) => ({
      project: state.project
        ? { ...state.project, cues: [...state.project.cues, cue] }
        : null,
      isDirty: true,
    }));
  },

  updateCue: (id, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === id ? { ...c, ...updates } : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },

  removeCue: (id) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.filter((c) => c.id !== id),
          }
        : null,
      isDirty: true,
    }));
  },

  reorderCues: (fromIndex, toIndex) => {
    set((state) => {
      if (!state.project) return state;
      const cues = [...state.project.cues];
      const [removed] = cues.splice(fromIndex, 1);
      cues.splice(toIndex, 0, removed);
      return {
        project: { ...state.project, cues },
        isDirty: true,
      };
    });
  },

  addItemToCue: (cueId, item) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId ? { ...c, items: [...c.items, item] } : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },

  updateItem: (cueId, itemId, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId
                ? {
                    ...c,
                    items: c.items.map((i) =>
                      i.id === itemId ? { ...i, ...updates } : i
                    ),
                  }
                : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },

  removeItem: (cueId, itemId) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId
                ? { ...c, items: c.items.filter((i) => i.id !== itemId) }
                : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },

  addOutput: (output) => {
    set((state) => ({
      project: state.project
        ? { ...state.project, outputs: [...state.project.outputs, output] }
        : null,
      isDirty: true,
    }));
  },

  updateOutput: (id, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            outputs: state.project.outputs.map((o) =>
              o.id === id ? { ...o, ...updates } : o
            ),
          }
        : null,
      isDirty: true,
    }));
  },

  removeOutput: (id) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            outputs: state.project.outputs.filter((o) => o.id !== id),
          }
        : null,
      isDirty: true,
    }));
  },

  setMasterBrightness: (value) => {
    invoke('set_master_brightness', { value }).catch(console.error);
    set((state) => ({
      project: state.project
        ? { ...state.project, masterBrightness: value }
        : null,
    }));
  },
}));
