import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { Transfer } from "../types";

interface TransfersState {
  transfers: Transfer[];
  selectedTransferId: string | null;
  addTransfer: (t: Transfer) => void;
  updateTransfer: (id: string, patch: Partial<Transfer>) => void;
  removeTransfer: (id: string) => void;
  selectTransfer: (id: string | null) => void;
  hasActiveTransfers: () => boolean;
}

export const useTransfersStore = create<TransfersState>()(
  subscribeWithSelector((set, get) => ({
    transfers: [],
    selectedTransferId: null,
    addTransfer: (t) => set((s) => ({ transfers: [t, ...s.transfers] })),
    updateTransfer: (id, patch) =>
      set((s) => ({
        transfers: s.transfers.map((t) => (t.id === id ? { ...t, ...patch } : t)),
      })),
    removeTransfer: (id) =>
      set((s) => ({
        transfers: s.transfers.filter((t) => t.id !== id),
        selectedTransferId: s.selectedTransferId === id ? null : s.selectedTransferId,
      })),
    selectTransfer: (id) => set({ selectedTransferId: id }),
    hasActiveTransfers: () =>
      get().transfers.some((t) => t.status === "running" || t.status === "queued"),
  }))
);
