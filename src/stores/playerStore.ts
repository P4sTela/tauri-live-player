import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { PlayerState, PlayerStatus } from '../types';

interface PlayerStore {
  status: PlayerStatus;
  currentCueIndex: number;
  currentTime: number;
  duration: number;
  error: string | null;

  // Actions
  loadCue: (index: number) => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (time: number) => Promise<void>;
  next: () => Promise<void>;
  prev: () => Promise<void>;

  // State sync
  syncState: () => Promise<void>;
  setError: (error: string | null) => void;
}

export const usePlayerStore = create<PlayerStore>((set, get) => ({
  status: 'idle',
  currentCueIndex: -1,
  currentTime: 0,
  duration: 0,
  error: null,

  loadCue: async (index) => {
    try {
      set({ status: 'loading', error: null });
      await invoke('load_cue', { cueIndex: index });
      set({ status: 'ready', currentCueIndex: index });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },

  play: async () => {
    try {
      await invoke('play');
      set({ status: 'playing', error: null });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },

  pause: async () => {
    try {
      await invoke('pause');
      set({ status: 'paused', error: null });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },

  stop: async () => {
    try {
      await invoke('stop');
      set({ status: 'idle', currentTime: 0, error: null });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },

  seek: async (time) => {
    try {
      await invoke('seek', { position: time });
      set({ currentTime: time, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  next: async () => {
    const { currentCueIndex, loadCue } = get();
    await loadCue(currentCueIndex + 1);
  },

  prev: async () => {
    const { currentCueIndex, loadCue } = get();
    if (currentCueIndex > 0) {
      await loadCue(currentCueIndex - 1);
    }
  },

  syncState: async () => {
    try {
      const state = await invoke<PlayerState>('get_player_state');
      set({
        status: state.status,
        currentCueIndex: state.currentCueIndex,
        currentTime: state.currentTime,
        duration: state.duration,
        error: state.error ?? null,
      });
    } catch (_e) {
      // ignore sync errors
    }
  },

  setError: (error) => {
    set({ error });
  },
}));
