import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";

export type View = "transfer" | "history" | "settings" | "pairing";
export type MobileTab = "home" | "send" | "received" | "settings";

interface NavState {
  view: View;
  mobileTab: MobileTab;
  setView: (v: View) => void;
  setMobileTab: (t: MobileTab) => void;
}

export const useNavStore = create<NavState>()(
  subscribeWithSelector((set) => ({
    view: "transfer",
    mobileTab: "home",
    setView: (view) => set({ view }),
    setMobileTab: (mobileTab) => set({ mobileTab }),
  }))
);
