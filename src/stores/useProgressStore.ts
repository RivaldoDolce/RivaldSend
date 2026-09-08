import { create } from "zustand";

interface ProgressData {
  bytesDone: number;
  speedBps: number;
  etaSecs: number;
}

interface ProgressState {
  byId: Record<string, ProgressData>;
}

export const useProgressStore = create<ProgressState>()(() => ({
  byId: {},
}));

let pending: Record<string, ProgressData> = {};
let raf = 0;

export function pushProgress(id: string, p: ProgressData) {
  pending[id] = p;
  if (!raf) {
    raf = requestAnimationFrame(() => {
      const batch = pending;
      pending = {};
      raf = 0;
      useProgressStore.setState((s) => ({
        byId: { ...s.byId, ...batch },
      }));
    });
  }
}

export function clearProgress(id: string) {
  useProgressStore.setState((s) => {
    const next = { ...s.byId };
    delete next[id];
    return { byId: next };
  });
}
