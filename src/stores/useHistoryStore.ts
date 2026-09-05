import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import type { HistoryEntry } from "../types";

interface HistoryState {
  entries: HistoryEntry[];
  addEntry: (e: HistoryEntry) => void;
  removeEntry: (id: string) => void;
  clearAll: () => void;
  getSent: () => HistoryEntry[];
  getReceived: () => HistoryEntry[];
}

export const useHistoryStore = create<HistoryState>()(
  persist(
    (set, get) => ({
      entries: [],
      addEntry: (e) =>
        set((s) => ({
          entries: [e, ...s.entries].slice(0, 500),
        })),
      removeEntry: (id) =>
        set((s) => ({
          entries: s.entries.filter((e) => e.id !== id),
        })),
      clearAll: () => set({ entries: [] }),
      getSent: () => get().entries.filter((e) => e.direction === "sent"),
      getReceived: () => get().entries.filter((e) => e.direction === "received"),
    }),
    {
      name: "rivaldsend-history",
      storage: createJSONStorage(() => localStorage),
    }
  )
);
