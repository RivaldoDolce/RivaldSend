import { create } from "zustand";
import type { Peer, Transfer, HistoryEntry } from "../types";

type View = "transfer" | "history" | "settings" | "pairing";

interface AppState {
  view: View;
  peers: Peer[];
  transfers: Transfer[];
  history: HistoryEntry[];
  selectedPeerId: string | null;
  darkMode: boolean;
  downloadDir: string;
  setView: (v: View) => void;
  setPeers: (p: Peer[]) => void;
  setTransfers: (t: Transfer[]) => void;
  addHistory: (h: HistoryEntry) => void;
  toggleDarkMode: () => void;
  selectPeer: (id: string | null) => void;
  setDownloadDir: (d: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  view: "transfer",
  peers: [
    { id: "1", name: "MacBook de Rivald", fingerprint: "a1b2c3d4", fingerprintShort: "a1b2", ip: "192.168.1.42", port: 7420, status: "paired", trusted: true },
    { id: "2", name: "iPhone 15", fingerprint: "e5f6g7h8", fingerprintShort: "e5f6", ip: "192.168.1.18", port: 7420, status: "discovered", trusted: false },
  ],
  transfers: [],
  history: [],
  selectedPeerId: null,
  darkMode: window.matchMedia("(prefers-color-scheme: dark)").matches,
  downloadDir: "~/Téléchargements",
  setView: (view) => set({ view }),
  setPeers: (peers) => set({ peers }),
  setTransfers: (transfers) => set({ transfers }),
  addHistory: (h) => set((s) => ({ history: [h, ...s.history] })),
  toggleDarkMode: () => set((s) => ({ darkMode: !s.darkMode })),
  selectPeer: (id) => set({ selectedPeerId: id }),
  setDownloadDir: (downloadDir) => set({ downloadDir }),
}));
