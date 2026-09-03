import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { Peer, Transfer, HistoryEntry } from "../types";

type View = "transfer" | "history" | "settings" | "pairing";
type MobileTab = "home" | "send" | "received" | "settings";

interface AppState {
  view: View;
  mobileTab: MobileTab;
  peers: Peer[];
  transfers: Transfer[];
  history: HistoryEntry[];
  selectedPeerId: string | null;
  selectedTransferId: string | null;
  darkMode: boolean;
  downloadDir: string;
  isDiscovering: boolean;
  setView: (v: View) => void;
  setMobileTab: (t: MobileTab) => void;
  setPeers: (p: Peer[]) => void;
  setTransfers: (t: Transfer[]) => void;
  addHistory: (h: HistoryEntry) => void;
  toggleDarkMode: () => void;
  selectPeer: (id: string | null) => void;
  selectTransfer: (id: string | null) => void;
  setDownloadDir: (d: string) => void;
  setDiscovering: (v: boolean) => void;
}

export const useAppStore = create<AppState>()(
  subscribeWithSelector((set) => ({
    view: "transfer",
    mobileTab: "home",
    peers: [
      { id: "1", name: "MacBook de Rivald", fingerprint: "a1b2c3d4", fingerprintShort: "a1b2", ip: "192.168.1.42", port: 7420, status: "paired", trusted: true },
      { id: "2", name: "PC Bureau", fingerprint: "e5f6g7h8", fingerprintShort: "e5f6", ip: "192.168.1.15", port: 7420, status: "discovered", trusted: false },
    ],
    transfers: [],
    history: [],
    selectedPeerId: null,
    selectedTransferId: null,
    darkMode: typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)").matches : false,
    downloadDir: "~/Téléchargements",
    isDiscovering: true,
    setView: (view) => set({ view }),
    setMobileTab: (mobileTab) => set({ mobileTab }),
    setPeers: (peers) => set({ peers }),
    setTransfers: (transfers) => set({ transfers }),
    addHistory: (h) => set((s) => ({ history: [h, ...s.history] })),
    toggleDarkMode: () => set((s) => ({ darkMode: !s.darkMode })),
    selectPeer: (id) => set({ selectedPeerId: id }),
    selectTransfer: (id) => set({ selectedTransferId: id }),
    setDownloadDir: (downloadDir) => set({ downloadDir }),
    setDiscovering: (isDiscovering) => set({ isDiscovering }),
  }))
);
