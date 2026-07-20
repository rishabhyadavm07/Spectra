import { create } from "zustand";
import { Workspace, RequestSummary, Folder, Environment } from "../types";

interface WorkspaceState {
  workspaces: Workspace[];
  setWorkspaces: (
    workspaces: Workspace[] | ((prev: Workspace[]) => Workspace[]),
  ) => void;

  activeWorkspace: Workspace | null;
  setActiveWorkspace: (ws: Workspace | null) => void;

  requests: RequestSummary[];
  setRequests: (requests: RequestSummary[]) => void;

  folders: Folder[];
  setFolders: (folders: Folder[]) => void;

  autoHeaders: [string, string][];
  setAutoHeaders: (headers: [string, string][]) => void;

  treeFilter: string;
  setTreeFilter: (filter: string) => void;

  historyRefreshSignal: number;
  setHistoryRefreshSignal: (
    signal: number | ((prev: number) => number),
  ) => void;

  savedResponsesRefreshSignal: number;
  setSavedResponsesRefreshSignal: (
    signal: number | ((prev: number) => number),
  ) => void;

  newTopLevelFolderSignal: number;
  setNewTopLevelFolderSignal: (
    signal: number | ((prev: number) => number),
  ) => void;

  variableNames: string[];
  setVariableNames: (names: string[]) => void;

  activeEnvironment: Environment | null;
  setActiveEnvironment: (env: Environment | null) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspaces: [],
  setWorkspaces: (v) =>
    set((state) => ({
      workspaces: typeof v === "function" ? v(state.workspaces) : v,
    })),

  activeWorkspace: null,
  setActiveWorkspace: (ws) => set({ activeWorkspace: ws }),

  requests: [],
  setRequests: (reqs) => set({ requests: reqs }),

  folders: [],
  setFolders: (folders) => set({ folders }),

  autoHeaders: [],
  setAutoHeaders: (headers) => set({ autoHeaders: headers }),

  treeFilter: "",
  setTreeFilter: (filter) => set({ treeFilter: filter }),

  historyRefreshSignal: 0,
  setHistoryRefreshSignal: (v) =>
    set((state) => ({
      historyRefreshSignal:
        typeof v === "function" ? v(state.historyRefreshSignal) : v,
    })),

  savedResponsesRefreshSignal: 0,
  setSavedResponsesRefreshSignal: (v) =>
    set((state) => ({
      savedResponsesRefreshSignal:
        typeof v === "function" ? v(state.savedResponsesRefreshSignal) : v,
    })),

  newTopLevelFolderSignal: 0,
  setNewTopLevelFolderSignal: (v) =>
    set((state) => ({
      newTopLevelFolderSignal:
        typeof v === "function" ? v(state.newTopLevelFolderSignal) : v,
    })),

  variableNames: [],
  setVariableNames: (names) => set({ variableNames: names }),

  activeEnvironment: null,
  setActiveEnvironment: (env) => set({ activeEnvironment: env }),
}));
