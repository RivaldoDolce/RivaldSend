import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

interface SettingsState {
  darkMode: boolean;
  downloadDir: string;
  language: "fr" | "en" | "es";
  autoAcceptKnownPeers: boolean;
  notifications: boolean;
  toggleDarkMode: () => void;
  setDownloadDir: (d: string) => void;
  setLanguage: (l: SettingsState["language"]) => void;
  setAutoAcceptKnownPeers: (v: boolean) => void;
  setNotifications: (v: boolean) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      darkMode:
        typeof window !== "undefined" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches,
      downloadDir: "~/Téléchargements",
      language: "fr",
      autoAcceptKnownPeers: true,
      notifications: true,
      toggleDarkMode: () => set((s) => ({ darkMode: !s.darkMode })),
      setDownloadDir: (downloadDir) => set({ downloadDir }),
      setLanguage: (language) => set({ language }),
      setAutoAcceptKnownPeers: (autoAcceptKnownPeers) =>
        set({ autoAcceptKnownPeers }),
      setNotifications: (notifications) => set({ notifications }),
    }),
    {
      name: "rivaldsend-settings",
      storage: createJSONStorage(() => localStorage),
    }
  )
);
