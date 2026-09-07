import { create } from "zustand";
import { createJSONStorage, persist, subscribeWithSelector } from "zustand/middleware";
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
  persist(
    subscribeWithSelector((set) => ({
    peers: [],
    selectedPeerId: null,
    isDiscovering: false,
    showSendModal: false,
    pendingFiles: [],
    setPeers: (peers) => set({ peers }),
    addPeer: (peer) =>
      set((s) => {
        const dupIdx = s.peers.findIndex(
          (p) => p.id === peer.id || (p.ip === peer.ip && p.port === peer.port)
        );
        if (dupIdx !== -1) {
          const existing = s.peers[dupIdx]!;
          const isGeneric = peer.name === `Appareil ${peer.ip}` || peer.name.startsWith("iface-");
          const merged: Peer = {
            ...existing,
            ...peer,
            name: !isGeneric && existing.name.startsWith("Appareil ") ? peer.name : existing.name || peer.name,
            fingerprint: peer.fingerprint || existing.fingerprint,
            fingerprintShort: peer.fingerprintShort || existing.fingerprintShort,
            trusted: peer.trusted || existing.trusted,
            status: peer.trusted || existing.trusted ? "paired" : peer.status,
            latencyMs: peer.latencyMs ?? existing.latencyMs,
          };
          if (JSON.stringify(merged) === JSON.stringify(existing)) return s;
          const next = [...s.peers];
          next[dupIdx] = merged;
          return { peers: next };
        }
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
    })),
    {
      name: "rivaldsend-peers",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        peers: state.peers.filter((peer) => peer.trusted),
      }),
    },
  ),
);
