import { create } from "zustand";

interface ServerState {
  tls: boolean;
  empreinte: string | null;
  port: number | null;
  setReady: (info: { tls: boolean; empreinte?: string; port?: number }) => void;
}

export const useServerStore = create<ServerState>()((set) => ({
  tls: false,
  empreinte: null,
  port: null,
  setReady: (info) =>
    set({
      tls: info.tls,
      empreinte: info.empreinte ?? null,
      port: info.port ?? null,
    }),
}));
