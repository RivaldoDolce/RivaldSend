import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";

export interface IncomingRequest {
  requestId: string;
  peerId: string;
  peerName: string;
  files: Array<{ name: string; size: number }>;
  totalBytes: number;
}

interface IncomingState {
  pending: IncomingRequest | null;
  setPending: (r: IncomingRequest) => void;
  clear: () => void;
}

export const useIncomingStore = create<IncomingState>()(
  subscribeWithSelector((set) => ({
    pending: null,
    setPending: (pending) => set({ pending }),
    clear: () => set({ pending: null }),
  }))
);
