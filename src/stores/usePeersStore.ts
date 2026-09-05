import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { Peer } from "../types";

interface PeersState {
  peers: Peer[];
  selectedPeerId: string | null;
  isDiscovering: boolean;
  showSendModal: boolean;
  pendingFiles: Array<{ path: string; size: number }>;
  setPeers: (peers: Peer[]) => void;
  addPeer: (peer: Peer) => void;
  removePeer: (id: string) => void;
  updatePeer: (id: string, patch: Partial<Peer>) => void;
  selectPeer: (id: string | null) => void;
  setDiscovering: (v: boolean) => void;
  openSendModal: (files: Array<{ path: string; size: number }>) => void;
  closeSendModal: () => void;
}

export const usePeersStore = create<PeersState>()(
  subscribeWithSelector((set) => ({
    peers: [],
    selectedPeerId: null,
    isDiscovering: false,
    showSendModal: false,
    pendingFiles: [],
    setPeers: (peers) => set({ peers }),
    addPeer: (peer) =>
      set((s) => {
        if (s.peers.some((p) => p.id === peer.id)) return s;
        return { peers: [...s.peers, peer] };
      }),
    removePeer: (id) =>
      set((s) => ({
        peers: s.peers.filter((p) => p.id !== id),
        selectedPeerId: s.selectedPeerId === id ? null : s.selectedPeerId,
      })),
    updatePeer: (id, patch) =>
      set((s) => ({
        peers: s.peers.map((p) => (p.id === id ? { ...p, ...patch } : p)),
      })),
    selectPeer: (id) => set({ selectedPeerId: id }),
    setDiscovering: (isDiscovering) => set({ isDiscovering }),
    openSendModal: (files) => set({ showSendModal: true, pendingFiles: files }),
    closeSendModal: () => set({ showSendModal: false, pendingFiles: [] }),
  }))
);
