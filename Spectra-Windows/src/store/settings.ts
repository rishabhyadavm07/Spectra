import { create } from "zustand";

interface SettingsState {
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (
    collapsed: boolean | ((prev: boolean) => boolean),
  ) => void;

  sidebarWidth: number;
  setSidebarWidth: (width: number) => void;

  responsePanelHeight: number;
  setResponsePanelHeight: (height: number) => void;

  consoleOpen: boolean;
  setConsoleOpen: (open: boolean) => void;

  importModalOpen: boolean;
  setImportModalOpen: (open: boolean) => void;

  exportModalOpen: boolean;
  setExportModalOpen: (open: boolean) => void;

  settingsModalOpen: boolean;
  setSettingsModalOpen: (open: boolean) => void;

  collectionsCollapsed: boolean;
  setCollectionsCollapsed: (
    collapsed: boolean | ((prev: boolean) => boolean),
  ) => void;

  showLineNumbers: boolean;
  setShowLineNumbers: (show: boolean) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  sidebarCollapsed: false,
  setSidebarCollapsed: (v) =>
    set((state) => ({
      sidebarCollapsed: typeof v === "function" ? v(state.sidebarCollapsed) : v,
    })),

  sidebarWidth: 300,
  setSidebarWidth: (width) => set({ sidebarWidth: width }),

  responsePanelHeight: 320,
  setResponsePanelHeight: (height) => set({ responsePanelHeight: height }),

  consoleOpen: false,
  setConsoleOpen: (open) => set({ consoleOpen: open }),

  importModalOpen: false,
  setImportModalOpen: (open) => set({ importModalOpen: open }),

  exportModalOpen: false,
  setExportModalOpen: (open) => set({ exportModalOpen: open }),

  settingsModalOpen: false,
  setSettingsModalOpen: (open) => set({ settingsModalOpen: open }),

  collectionsCollapsed: false,
  setCollectionsCollapsed: (v) =>
    set((state) => ({
      collectionsCollapsed:
        typeof v === "function" ? v(state.collectionsCollapsed) : v,
    })),

  showLineNumbers: true,
  setShowLineNumbers: (show) => set({ showLineNumbers: show }),
}));
