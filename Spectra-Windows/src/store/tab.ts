import { create } from "zustand";
import { TabState } from "../types";

interface TabStoreState {
  tabs: TabState[];
  setTabs: (tabs: TabState[] | ((prev: TabState[]) => TabState[])) => void;

  activeTabId: string | null;
  setActiveTabId: (id: string | null) => void;

  tabPendingClose: string | null;
  setTabPendingClose: (id: string | null) => void;

  warningCount: number;
  setWarningCount: (count: number | ((prev: number) => number)) => void;
}

export const useTabStore = create<TabStoreState>((set) => ({
  tabs: [],
  setTabs: (v) =>
    set((state) => ({
      tabs: typeof v === "function" ? v(state.tabs) : v,
    })),

  activeTabId: null,
  setActiveTabId: (id) => set({ activeTabId: id }),

  tabPendingClose: null,
  setTabPendingClose: (id) => set({ tabPendingClose: id }),

  warningCount: 0,
  setWarningCount: (v) =>
    set((state) => ({
      warningCount: typeof v === "function" ? v(state.warningCount) : v,
    })),
}));
