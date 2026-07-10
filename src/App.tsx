import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Breadcrumb } from "./Breadcrumb";
import { ConsolePanel } from "./ConsolePanel";
import type { ConsoleLogEntry } from "./ConsolePanel";
import { EnvironmentPanel } from "./EnvironmentPanel";
import { ExportModal } from "./ExportModal";
import { HistoryPanel } from "./HistoryPanel";
import { ImportModal } from "./ImportModal";
import { SettingsModal } from "./SettingsModal";
import { applyTheme } from "./theme";
import { RequestTabs } from "./RequestTabs";
import { RequestTree } from "./RequestTree";
import { ResponsePanel, type ResponsePanelRef } from "./ResponsePanel";
import { TopTabBar } from "./TopTabBar";
import type { OpenTab } from "./TopTabBar";
import { VarInput } from "./VarInput";
import { WorkspaceSwitcher } from "./WorkspaceSwitcher";
import { VariablePopoverContext } from "./VariablePopoverContext";
import { ErrorBoundary } from "./ErrorBoundary";
import {
  Package,
  Monitor,
  Clock,
  Search,
  Plus,
  Download,
  Upload,
  ChevronRight,
  ChevronDown,
  FolderPlus,
  GitBranch,
  Terminal,
  TerminalSquare,
  AlertTriangle,
  ChevronsRight,
  ChevronsLeft,
} from "lucide-react";
import type {
  AuthConfig,
  HeaderEntry,
  HistoryEntry,
  HttpMethod,
  ParamEntry,
  RequestBody,
  ResponseDto,
  SavedResponse,
  SpectraRequest,
  TabState,
} from "./types";
import "./App.css";

const METHODS: HttpMethod[] = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "OPTIONS",
  "HEAD",
];

type SidebarView = "collections" | "environments" | "history";

function isTabDirty(tab: TabState): boolean {
  return JSON.stringify(tab.request) !== JSON.stringify(tab.lastPersisted);
}

import { useWorkspaceStore } from "./store/workspace";
import { useTabStore } from "./store/tab";
import { useSettingsStore } from "./store/settings";

function App() {
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const setWorkspaces = useWorkspaceStore((s) => s.setWorkspaces);
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const setActiveWorkspace = useWorkspaceStore((s) => s.setActiveWorkspace);
  const requests = useWorkspaceStore((s) => s.requests);
  const setRequests = useWorkspaceStore((s) => s.setRequests);
  const folders = useWorkspaceStore((s) => s.folders);
  const setFolders = useWorkspaceStore((s) => s.setFolders);
  const autoHeaders = useWorkspaceStore((s) => s.autoHeaders);
  const setAutoHeaders = useWorkspaceStore((s) => s.setAutoHeaders);
  const treeFilter = useWorkspaceStore((s) => s.treeFilter);
  const setTreeFilter = useWorkspaceStore((s) => s.setTreeFilter);
  const historyRefreshSignal = useWorkspaceStore((s) => s.historyRefreshSignal);
  const setHistoryRefreshSignal = useWorkspaceStore(
    (s) => s.setHistoryRefreshSignal,
  );
  const savedResponsesRefreshSignal = useWorkspaceStore(
    (s) => s.savedResponsesRefreshSignal,
  );
  const setSavedResponsesRefreshSignal = useWorkspaceStore(
    (s) => s.setSavedResponsesRefreshSignal,
  );
  const newTopLevelFolderSignal = useWorkspaceStore(
    (s) => s.newTopLevelFolderSignal,
  );
  const setNewTopLevelFolderSignal = useWorkspaceStore(
    (s) => s.setNewTopLevelFolderSignal,
  );
  const variableNames = useWorkspaceStore((s) => s.variableNames);
  const setVariableNames = useWorkspaceStore((s) => s.setVariableNames);
  const activeEnvironment = useWorkspaceStore((s) => s.activeEnvironment);
  const setActiveEnvironment = useWorkspaceStore((s) => s.setActiveEnvironment);

  const tabs = useTabStore((s) => s.tabs);
  const setTabs = useTabStore((s) => s.setTabs);
  const activeTabId = useTabStore((s) => s.activeTabId);
  const setActiveTabId = useTabStore((s) => s.setActiveTabId);
  const tabPendingClose = useTabStore((s) => s.tabPendingClose);
  const setTabPendingClose = useTabStore((s) => s.setTabPendingClose);
  const warningCount = useTabStore((s) => s.warningCount);
  const setWarningCount = useTabStore((s) => s.setWarningCount);

  const sidebarCollapsed = useSettingsStore((s) => s.sidebarCollapsed);
  const setSidebarCollapsed = useSettingsStore((s) => s.setSidebarCollapsed);
  const sidebarWidth = useSettingsStore((s) => s.sidebarWidth);
  const setSidebarWidth = useSettingsStore((s) => s.setSidebarWidth);
  const responsePanelHeight = useSettingsStore((s) => s.responsePanelHeight);
  const setResponsePanelHeight = useSettingsStore(
    (s) => s.setResponsePanelHeight,
  );
  const consoleOpen = useSettingsStore((s) => s.consoleOpen);
  const setConsoleOpen = useSettingsStore((s) => s.setConsoleOpen);
  const importModalOpen = useSettingsStore((s) => s.importModalOpen);
  const setImportModalOpen = useSettingsStore((s) => s.setImportModalOpen);
  const exportModalOpen = useSettingsStore((s) => s.exportModalOpen);
  const setExportModalOpen = useSettingsStore((s) => s.setExportModalOpen);
  const settingsModalOpen = useSettingsStore((s) => s.settingsModalOpen);
  const setSettingsModalOpen = useSettingsStore((s) => s.setSettingsModalOpen);
  const collectionsCollapsed = useSettingsStore((s) => s.collectionsCollapsed);
  const setCollectionsCollapsed = useSettingsStore(
    (s) => s.setCollectionsCollapsed,
  );
  const setShowLineNumbers = useSettingsStore((s) => s.setShowLineNumbers);

  // We still need local state for sidebarView and consoleEntries which aren't in the stores yet
  const [sidebarView, setSidebarView] = useState<SidebarView>("collections");
  const [consoleEntries, setConsoleEntries] = useState<ConsoleLogEntry[]>([]);

  const resizingRef = useRef(false);
  // Enforced at render time (see the `<aside>` style below), not just as
  // the drag handler's floor — a floor on the drag handler alone only
  // guards the one code path that currently sets sidebarWidth, but doesn't
  // protect against a stale/narrow value reaching render through any other
  // path (a persisted setting added later, a bug, HMR preserving old state
  // across an edit). Clamping where it's actually rendered means the
  // sidebar physically cannot show narrower than this, ever, regardless of
  // how sidebarWidth got set — this is what "cuts down on the buttons when
  // I open the app" needs to be permanently fixed, not just harder to
  // trigger via dragging.
  const SIDEBAR_MIN_WIDTH = 260;

  // Height of the response panel, in pixels, measured from the bottom of
  // `.editor-content`. Drives a drag handle between the request builder
  // (RequestTabs) and the response panel — dragging up grows the response
  // panel; past the request builder's own height it overlaps/covers it
  // entirely (position: absolute + z-index, see .response-panel-resizing in
  // App.css) rather than being capped at "as tall as the remaining flex
  // space," so the user can see a full-height response the way they'd
  // expect from "extend to the full top."
  const editorContentRef = useRef<HTMLDivElement>(null);
  const responseResizingRef = useRef(false);

  // Optimization: DOM refs for resize handlers to prevent 60fps React re-renders
  const sidebarRef = useRef<HTMLElement>(null);
  const currentSidebarWidthRef = useRef<number>(sidebarWidth);
  const responsePanelRef = useRef<HTMLDivElement>(null);
  const responsePanelComponentRef = useRef<ResponsePanelRef>(null);
  const currentResponseHeightRef = useRef<number>(responsePanelHeight);

  const activeTab = tabs.find((t) => t.tabId === activeTabId) ?? null;

  // Kept in sync below so the automation event listener (registered once,
  // empty deps) can read current tabs/workspace without re-subscribing.
  const tabsRef = useRef(tabs);
  useEffect(() => {
    tabsRef.current = tabs;
  }, [tabs]);
  const activeWorkspaceRef = useRef(activeWorkspace);
  useEffect(() => {
    activeWorkspaceRef.current = activeWorkspace;
  }, [activeWorkspace]);

  function startSidebarResize(e: React.MouseEvent) {
    e.preventDefault();
    resizingRef.current = true;
    document.body.style.cursor = "col-resize";
    currentSidebarWidthRef.current = sidebarWidth;

    function handleMouseMove(ev: MouseEvent) {
      if (!resizingRef.current) return;
      const next = Math.min(560, Math.max(SIDEBAR_MIN_WIDTH, ev.clientX));
      currentSidebarWidthRef.current = next;
      if (sidebarRef.current) {
        sidebarRef.current.style.width = `${next}px`;
      }
      const iconRail = document.querySelector(".icon-rail") as HTMLElement;
      if (iconRail && !sidebarCollapsed) {
        iconRail.style.width = `calc(${next}px - var(--traffic-light-inset))`;
      }
    }
    function handleMouseUp() {
      resizingRef.current = false;
      document.body.style.cursor = "";
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      setSidebarWidth(currentSidebarWidthRef.current);
    }
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  function startResponsePanelResize(e: React.MouseEvent) {
    e.preventDefault();
    responseResizingRef.current = true;
    document.body.style.cursor = "row-resize";
    currentResponseHeightRef.current = responsePanelHeight;
    const containerTop =
      editorContentRef.current?.getBoundingClientRect().top ?? 0;
    const containerHeight =
      editorContentRef.current?.getBoundingClientRect().height ??
      window.innerHeight;

    function handleMouseMove(ev: MouseEvent) {
      if (!responseResizingRef.current) return;
      const next = Math.min(
        containerHeight,
        Math.max(120, containerTop + containerHeight - ev.clientY),
      );
      currentResponseHeightRef.current = next;
      if (responsePanelRef.current) {
        responsePanelRef.current.style.height = `${next}px`;
      }
    }
    function handleMouseUp() {
      responseResizingRef.current = false;
      document.body.style.cursor = "";
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      setResponsePanelHeight(currentResponseHeightRef.current);
    }
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  useEffect(() => {
    refreshWorkspaces();
    api.getSettings().then((settings) => {
      applyTheme(settings.theme || "system");
    });
  }, []);

  useEffect(() => {
    if (activeWorkspace) {
      refreshRequests(activeWorkspace.id);
      refreshFolders(activeWorkspace.id);
    }
    // Switching workspaces invalidates any open tabs from the previous one.
    setTabs([]);
    setActiveTabId(null);
  }, [activeWorkspace?.id]);

  useEffect(() => {
    refreshVariableNames();
  }, [activeWorkspace?.id, activeWorkspace?.active_environment_id]);

  // Automation: driven by spectra-mcp's automation_screenshot_request tool
  // via the Rust-side IPC server in crates/spectra-tauri/src/automation/.
  // The Rust side emits this event, waits (with a timeout) for us to call
  // automation_tab_ready back, then screenshots this window. We reuse the
  // exact same tab-opening (`openTabForRequest`) and sending (`api.sendRequest`)
  // logic the rest of the app uses — no parallel path — per HANDOFF.md's
  // "reuse the tab model" note.
  useEffect(() => {
    const unlistenPromise = listen<{
      request_id: string;
      environment_id?: string | null;
    }>("automation://prepare-request", async (event) => {
      const { request_id, environment_id } = event.payload;
      let workspaceId: string | null = null;
      let name: string | null = null;
      let url: string | null = null;
      try {
        const req = await api.openRequest(request_id);
        workspaceId = req.workspace_id;
        name = req.name;
        url = req.url;

        // The request may belong to a workspace that isn't currently active
        // in the GUI — opening a tab alone doesn't fix that, since the rest
        // of the UI (sidebar tree, request lookups) is scoped to whatever
        // workspace is active, and the pane renders blank otherwise. Switch
        // first, exactly like clicking the workspace in WorkspaceSwitcher.
        //
        // Switching workspaces triggers the effect keyed on `activeWorkspace?.id`
        // that clears `tabs`/`activeTabId` (switching workspaces normally
        // invalidates tabs from the previous one — see that effect above).
        // If we called `openTabForRequest` in the same tick, that effect's
        // `setTabs([])` could run after it and wipe the tab we just opened,
        // since both are separate state updates racing across renders. Wait
        // for a couple of animation frames after the switch so that effect
        // has actually flushed before opening the tab.
        if (activeWorkspaceRef.current?.id !== req.workspace_id) {
          const ws = await api.listWorkspaces();
          const targetWorkspace = ws.find((w) => w.id === req.workspace_id);
          if (targetWorkspace) {
            setWorkspaces(ws);
            setActiveWorkspace(targetWorkspace);
            await new Promise<void>((resolve) => setTimeout(resolve, 50));
          }
        }

        const tabId = openTabForRequest(req);

        // Reuse the tabs array to find out whether this tab already has a
        // response (e.g. it was already open and sent earlier) — if not,
        // send it now, exactly like the Send button does.
        const existingTab = tabsRef.current.find((t) => t.tabId === tabId);
        const needsSend =
          !existingTab || (!existingTab.response && !existingTab.error);

        let sendError: string | null = null;
        if (needsSend) {
          updateTab(tabId, {
            sending: true,
            error: null,
            response: null,
            savedResponseName: null,
          });
          try {
            const resp = await api.sendRequest(
              request_id,
              environment_id ??
                activeWorkspaceRef.current?.active_environment_id ??
                undefined,
            );
            updateTab(tabId, { response: resp });
          } catch (e) {
            sendError = String(e);
            updateTab(tabId, { error: sendError });
          } finally {
            updateTab(tabId, { sending: false });
          }
        } else {
          sendError = existingTab?.error ?? null;
        }

        // Give React two animation frames to actually paint the response
        // before screenshotting — a real (if approximate) readiness signal
        // rather than a blind fixed sleep, since response render time is
        // effectively instant once state is set (no async layout work),
        // so two rAFs is enough to guarantee the paint has happened.
        await new Promise<void>((resolve) => setTimeout(resolve, 50));

        await api.automationTabReady({
          request_id,
          workspace_id: workspaceId,
          name,
          url,
          rendered: true,
          send_error: sendError,
        });
      } catch (e) {
        // If anything above throws (e.g. the request_id doesn't resolve at
        // all), still tell the Rust side we're "done" so it doesn't hang
        // for the full timeout — but report `rendered: false` so the caller
        // (screenshot or focus tool) can tell this apart from a real,
        // if error-showing, render.
        try {
          await api.automationTabReady({
            request_id,
            workspace_id: workspaceId,
            name,
            url,
            rendered: false,
            send_error: String(e),
          });
        } catch {
          // Nothing more we can do from the frontend if even this fails.
        }
      }
    });

    const unlistenLineNumbersPromise = listen<{ show: boolean }>(
      "automation://set-line-numbers",
      (event) => {
        setShowLineNumbers(event.payload.show);
      },
    );

    const unlistenSearchResponsePromise = listen<{
      request_id: string;
      query: string;
      environment_id?: string | null;
    }>("automation://search-response", async (event) => {
      const { request_id, query, environment_id } = event.payload;
      try {
        // Open/send identical to prepare-request, but we don't need to report rendered back
        // since the search tool relies on the DOM state *after* rendering.
        const req = await api.openRequest(request_id);
        if (activeWorkspaceRef.current?.id !== req.workspace_id) {
          const ws = await api.listWorkspaces();
          const targetWorkspace = ws.find((w) => w.id === req.workspace_id);
          if (targetWorkspace) {
            setWorkspaces(ws);
            setActiveWorkspace(targetWorkspace);
            await new Promise<void>((resolve) =>
              requestAnimationFrame(() =>
                requestAnimationFrame(() => resolve()),
              ),
            );
          }
        }
        const tabId = openTabForRequest(req);
        const existingTab = tabsRef.current.find((t) => t.tabId === tabId);
        const needsSend =
          !existingTab || (!existingTab.response && !existingTab.error);

        if (needsSend) {
          updateTab(tabId, {
            sending: true,
            error: null,
            response: null,
            savedResponseName: null,
          });
          try {
            const resp = await api.sendRequest(
              request_id,
              environment_id ??
                activeWorkspaceRef.current?.active_environment_id ??
                undefined,
            );
            updateTab(tabId, { response: resp });
          } catch (e) {
            updateTab(tabId, { error: String(e) });
          } finally {
            updateTab(tabId, { sending: false });
          }
        }

        // Wait for response to render in Monaco
        await new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        );

        let matchCount = 0;
        let firstMatchLine = null;

        if (responsePanelComponentRef.current) {
          const matches = responsePanelComponentRef.current.findMatches(query);
          matchCount = matches.length;
          if (matches.length > 0) {
            firstMatchLine = matches[0].range.startLineNumber;
            responsePanelComponentRef.current.revealLine(firstMatchLine);

            // Wait for scroll to settle
            await new Promise<void>((resolve) =>
              requestAnimationFrame(() =>
                requestAnimationFrame(() => resolve()),
              ),
            );
          }
        }

        await api.automationSearchReady({
          request_id,
          match_count: matchCount,
          first_match_line: firstMatchLine,
        });
      } catch (e) {
        try {
          await api.automationSearchReady({
            request_id,
            match_count: 0,
            first_match_line: null,
          });
        } catch {}
      }
    });

    return () => {
      unlistenPromise.then((f) => f());
      unlistenLineNumbersPromise.then((f) => f());
      unlistenSearchResponsePromise.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (!activeTab) {
      setAutoHeaders([]);
      return;
    }
    const requestId = activeTab.request.id;
    const environmentId = activeWorkspace?.active_environment_id ?? undefined;
    const timer = window.setTimeout(async () => {
      try {
        setAutoHeaders(await api.previewHeaders(requestId, environmentId));
      } catch {
        setAutoHeaders([]);
      }
    }, 300);
    return () => window.clearTimeout(timer);
  }, [
    activeTab?.request.id,
    activeTab?.request.method,
    activeTab?.request.url,
    activeTab?.request.params,
    activeTab?.request.body,
    activeTab?.request.auth,
    activeWorkspace?.active_environment_id,
  ]);

  async function refreshWorkspaces() {
    const ws = await api.listWorkspaces();
    setWorkspaces(ws);
    if (!activeWorkspace && ws.length > 0) setActiveWorkspace(ws[0]);
  }

  async function refreshRequests(workspaceId: string) {
    setRequests(await api.listRequests(workspaceId));
  }

  async function refreshFolders(workspaceId: string) {
    setFolders(await api.listFolders(workspaceId));
  }

  async function refreshVariableNames() {
    if (!activeWorkspace) {
      setVariableNames([]);
      setActiveEnvironment(null);
      return;
    }
    const envs = await api.listEnvironments(activeWorkspace.id);
    const active = envs.find(
      (e) => e.id === activeWorkspace.active_environment_id,
    );
    setVariableNames(active ? Object.keys(active.variables) : []);
    setActiveEnvironment(active ?? null);
  }

  /** Persists a single variable's value from the variable-pill popover,
   * preserving every other variable (and this one's secret flag) untouched.
   * Scoped to whichever environment is currently active — see
   * VariablePopoverContext's doc comment for why there's no scope picker. */
  async function handleUpdateVariable(name: string, value: string) {
    if (!activeWorkspace || !activeEnvironment) return;
    const existing = activeEnvironment.variables[name];
    if (!existing) return;
    const variables = Object.fromEntries(
      Object.entries(activeEnvironment.variables).map(([k, v]) => [
        k,
        { value: v.value, secret: v.secret },
      ]),
    );
    variables[name] = { value, secret: existing.secret };
    await api.updateEnvironment(
      activeWorkspace.id,
      activeEnvironment.id,
      activeEnvironment.name,
      variables,
    );
    refreshVariableNames();
  }

  function updateTab(tabId: string, patch: Partial<TabState>) {
    setTabs((prev) =>
      prev.map((t) => (t.tabId === tabId ? { ...t, ...patch } : t)),
    );
  }

  function updateTabRequest(tabId: string, patch: Partial<SpectraRequest>) {
    setTabs((prev) =>
      prev.map((t) =>
        t.tabId === tabId ? { ...t, request: { ...t.request, ...patch } } : t,
      ),
    );
  }

  /** Applies `patch` to both `request` and `lastPersisted` — use after a
   * commitX call succeeds, so the tab's dirty-diff no longer flags the
   * just-saved fields. */
  function markPersisted(tabId: string, patch: Partial<SpectraRequest>) {
    setTabs((prev) =>
      prev.map((t) =>
        t.tabId === tabId
          ? {
              ...t,
              request: { ...t.request, ...patch },
              lastPersisted: { ...t.lastPersisted, ...patch },
            }
          : t,
      ),
    );
  }

  async function handleCreateWorkspace(name: string) {
    const ws = await api.createWorkspace(name);
    setWorkspaces((prev) => [...prev, ws]);
    setActiveWorkspace(ws);
  }

  async function handleCreateRequest(folderId: string | null = null) {
    if (!activeWorkspace) return;
    const req = await api.createRequest(
      activeWorkspace.id,
      "New Request",
      "GET",
      "https://httpbin.org/get",
    );
    if (folderId) await api.moveRequest(req.id, folderId);
    await refreshRequests(activeWorkspace.id);
    const fresh = await api.openRequest(req.id);
    openTabForRequest(fresh);
  }

  /** Opens a tab for `request` — switches to it if already open, otherwise
   * appends a new tab and makes it active (standard browser/IDE behavior).
   * Returns the resolved tabId synchronously so callers can immediately
   * target that tab (e.g. to attach a saved/replayed response) without
   * racing the state update. */
  function openTabForRequest(
    request: SpectraRequest,
    savedResponseName: string | null = null,
  ): string {
    // Reads tabsRef (not the closed-over `tabs`) so this is safe to call from
    // any stale closure — e.g. the automation event listener below is
    // registered once with an empty dependency array, so its captured
    // `openTabForRequest` reference is the one from the first render; if it
    // read `tabs` directly, it would always see `tabs` as `[]` and never
    // recognize an already-open tab as existing, creating a duplicate on
    // every automation call instead of reusing/reactivating the real one.
    const existing = tabsRef.current.find((t) => t.request.id === request.id);
    const tabId = existing?.tabId ?? crypto.randomUUID();

    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.request.id === request.id);
      if (idx !== -1) {
        const copy = [...prev];
        copy[idx] = {
          ...copy[idx],
          request,
          savedResponseName: savedResponseName ?? copy[idx].savedResponseName,
        };
        return copy;
      }
      return [
        ...prev,
        {
          tabId,
          request,
          response: null,
          error: null,
          sending: false,
          savedResponseName,
          lastPersisted: request,
        },
      ];
    });
    setActiveTabId(tabId);
    return tabId;
  }

  async function openRequest(id: string) {
    const req = await api.openRequest(id);
    openTabForRequest(req);
  }

  async function openSavedResponse(saved: SavedResponse) {
    const req = await api.openRequest(saved.request_id);
    const tabId = openTabForRequest(req, saved.name);
    updateTab(tabId, {
      response: saved.response,
      error: null,
      savedResponseName: saved.name,
    });
  }

  function closeTabImmediately(tabId: string) {
    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.tabId === tabId);
      const next = prev.filter((t) => t.tabId !== tabId);
      if (activeTabId === tabId) {
        const fallback =
          next[idx] ?? next[idx - 1] ?? next[next.length - 1] ?? null;
        setActiveTabId(fallback ? fallback.tabId : null);
      }
      return next;
    });
    setTabPendingClose(null);
  }

  // window.confirm() is a silent no-op in Tauri's WKWebView (see HANDOFF.md),
  // so a dirty tab's close confirmation is this small inline prompt instead
  // of a blocking native dialog.
  function requestCloseTab(tabId: string) {
    const tab = tabs.find((t) => t.tabId === tabId);
    if (tab && isTabDirty(tab)) {
      setTabPendingClose(tabId);
    } else {
      closeTabImmediately(tabId);
    }
  }

  function closeOtherTabs(tabId: string) {
    const dirtyOthers = tabs.filter((t) => t.tabId !== tabId && isTabDirty(t));
    if (dirtyOthers.length > 0) {
      // Multiple tabs could need confirmation; keep this simple and safe by
      // only auto-closing the clean ones, leaving dirty ones open rather
      // than silently discarding several unsaved edits at once.
      setTabs((prev) => prev.filter((t) => t.tabId === tabId || isTabDirty(t)));
    } else {
      setTabs((prev) => prev.filter((t) => t.tabId === tabId));
    }
    setActiveTabId(tabId);
  }

  function closeAllTabs() {
    const dirty = tabs.filter((t) => isTabDirty(t));
    if (dirty.length > 0) {
      setTabs(dirty);
      setActiveTabId(dirty[0].tabId);
    } else {
      setTabs([]);
      setActiveTabId(null);
    }
  }

  function forceCloseAllTabs() {
    setTabs([]);
    setActiveTabId(null);
    setTabPendingClose(null);
  }

  function revealInSidebar(requestId: string) {
    const req = requests.find((r) => r.id === requestId);
    setSidebarView("collections");
    setCollectionsCollapsed(false);
    if (req) setTreeFilter(req.name);
  }

  async function handleCreateFolder(
    parentFolderId: string | null,
    name: string,
  ) {
    if (!activeWorkspace) return;
    await api.createFolder(activeWorkspace.id, parentFolderId, name);
    refreshFolders(activeWorkspace.id);
  }

  async function handleRenameFolder(id: string, name: string) {
    if (!activeWorkspace) return;
    await api.renameFolder(activeWorkspace.id, id, name);
    refreshFolders(activeWorkspace.id);
  }

  async function handleRenameRequest(id: string, name: string) {
    if (!activeWorkspace) return;
    const updated = await api.setName(id, name);
    setTabs((prev) =>
      prev.map((t) =>
        t.request.id === id
          ? { ...t, request: updated, lastPersisted: updated }
          : t,
      ),
    );
    refreshRequests(activeWorkspace.id);
  }

  async function handleDeleteFolder(id: string) {
    if (!activeWorkspace) return;
    await api.deleteFolder(activeWorkspace.id, id);
    refreshFolders(activeWorkspace.id);
    refreshRequests(activeWorkspace.id);
  }

  async function handleSetFolderAuth(id: string, auth: AuthConfig) {
    if (!activeWorkspace) return;
    await api.setFolderAuth(activeWorkspace.id, id, auth);
    refreshFolders(activeWorkspace.id);
  }

  async function handleDeleteRequest(id: string) {
    if (!activeWorkspace) return;
    await api.deleteRequest(id);
    setTabs((prev) => {
      const closing = prev.find((t) => t.request.id === id);
      const next = prev.filter((t) => t.request.id !== id);
      if (closing && activeTabId === closing.tabId) {
        setActiveTabId(next.length > 0 ? next[0].tabId : null);
      }
      return next;
    });
    refreshRequests(activeWorkspace.id);
  }

  /** Creates a sibling copy of a request in the same folder, named "<name> Copy" —
   * built from the same create/set-field calls the rest of the app uses rather
   * than a dedicated backend command, since there's nothing a "duplicate"
   * needs that isn't already expressible as create + copy each field. */
  async function handleDuplicateRequest(id: string) {
    if (!activeWorkspace) return;
    const original = await api.openRequest(id);
    const copy = await api.createRequest(
      activeWorkspace.id,
      `${original.name} Copy`,
      original.method,
      original.url,
    );
    if (original.folder_id) await api.moveRequest(copy.id, original.folder_id);
    await api.setHeaders(copy.id, original.headers);
    await api.setParams(copy.id, original.params);
    await api.setBody(copy.id, original.body);
    await api.setAuth(copy.id, original.auth);
    if (original.notes) await api.setNotes(copy.id, original.notes);
    await refreshRequests(activeWorkspace.id);
    const fresh = await api.openRequest(copy.id);
    openTabForRequest(fresh);
  }

  /** Copies a request as a `curl` command to the clipboard — the same
   * export::curl serializer the Export modal uses, just routed to the
   * clipboard instead of a download, matching Postman's tab/request
   * "Copy" context-menu action. */
  async function handleCopyRequestAsCurl(id: string) {
    const curl = await api.exportRequest(id, "curl");
    await navigator.clipboard.writeText(curl);
  }

  async function handleMoveRequest(
    requestId: string,
    targetFolderId: string | null,
  ) {
    if (!activeWorkspace) return;
    await api.moveRequest(requestId, targetFolderId);
    refreshRequests(activeWorkspace.id);
    setTabs((prev) =>
      prev.map((t) =>
        t.request.id === requestId
          ? {
              ...t,
              request: { ...t.request, folder_id: targetFolderId },
              lastPersisted: { ...t.lastPersisted, folder_id: targetFolderId },
            }
          : t,
      ),
    );
  }

  async function handleMethodChange(method: HttpMethod) {
    if (!activeTab) return;
    const updated = await api.setMethod(activeTab.request.id, method);
    markPersisted(activeTab.tabId, updated);
    if (activeWorkspace) refreshRequests(activeWorkspace.id);
  }

  function handleUrlChange(url: string) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { url });
  }

  async function commitUrl() {
    if (!activeTab) return;
    await api.setUrl(activeTab.request.id, activeTab.request.url);
    markPersisted(activeTab.tabId, { url: activeTab.request.url });
    if (activeWorkspace) refreshRequests(activeWorkspace.id);
  }

  function handleHeadersChange(headers: HeaderEntry[]) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { headers });
  }

  async function commitHeaders(headers: HeaderEntry[]) {
    if (!activeTab) return;
    await api.setHeaders(activeTab.request.id, headers);
    markPersisted(activeTab.tabId, { headers });
  }

  function handleParamsChange(params: ParamEntry[]) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { params });
  }

  async function commitParams(params: ParamEntry[]) {
    if (!activeTab) return;
    await api.setParams(activeTab.request.id, params);
    markPersisted(activeTab.tabId, { params });
  }

  function handleBodyChange(body: RequestBody) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { body });
  }

  async function commitBody(body: RequestBody) {
    if (!activeTab) return;
    await api.setBody(activeTab.request.id, body);
    markPersisted(activeTab.tabId, { body });
  }

  function handleAuthChange(auth: AuthConfig) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { auth });
  }

  async function commitAuth(auth: AuthConfig) {
    if (!activeTab) return;
    await api.setAuth(activeTab.request.id, auth);
    markPersisted(activeTab.tabId, { auth });
  }

  function handleNotesChange(notes: string) {
    if (!activeTab) return;
    updateTabRequest(activeTab.tabId, { notes });
  }

  async function commitNotes(notes: string) {
    if (!activeTab) return;
    const updated = await api.setNotes(activeTab.request.id, notes);
    // The backend may have truncated past 50 words — reflect its authoritative
    // value back into tab state rather than trusting the client's local copy.
    markPersisted(activeTab.tabId, { notes: updated.notes });
    updateTabRequest(activeTab.tabId, { notes: updated.notes });
  }

  function logToConsole(
    method: string,
    url: string,
    status: number | null,
    durationMs: number | null,
    error: string | null,
  ) {
    setConsoleEntries((prev) => [
      {
        id: crypto.randomUUID(),
        method,
        url,
        status,
        durationMs,
        error,
        timestamp: new Date().toISOString(),
      },
      ...prev,
    ]);
  }

  async function handleSend() {
    if (!activeTab) return;
    const tabId = activeTab.tabId;
    const req = activeTab.request;
    updateTab(tabId, {
      sending: true,
      error: null,
      response: null,
      savedResponseName: null,
    });
    try {
      const resp = await api.sendRequest(
        req.id,
        activeWorkspace?.active_environment_id ?? undefined,
      );
      updateTab(tabId, { response: resp });
      logToConsole(req.method, req.url, resp.status, resp.duration_ms, null);
    } catch (e) {
      const message = String(e);
      updateTab(tabId, { error: message });
      setWarningCount((n) => n + 1);
      logToConsole(req.method, req.url, null, null, message);
    } finally {
      updateTab(tabId, { sending: false });
      setHistoryRefreshSignal((n) => n + 1);
    }
  }

  async function handleSaveResponse(name: string) {
    if (!activeWorkspace || !activeTab || !activeTab.response) return;
    await api.saveResponse(
      activeWorkspace.id,
      activeTab.request.id,
      name,
      activeTab.response,
    );
    setSavedResponsesRefreshSignal((n) => n + 1);
  }

  function handleHistoryReplay(
    entry: HistoryEntry,
    replayResponse: ResponseDto | null,
    replayError: string | null,
  ) {
    const tabId = openTabForRequest(entry.request_snapshot);
    updateTab(tabId, {
      response: replayResponse,
      error: replayError,
      savedResponseName: null,
    });
  }

  function handleHistoryConverted() {
    if (activeWorkspace) refreshRequests(activeWorkspace.id);
    setSidebarView("collections");
  }

  const openTabsForBar: OpenTab[] = tabs.map((t) => ({
    tabId: t.tabId,
    requestId: t.request.id,
    name: t.request.name,
    method: t.request.method,
    dirty: isTabDirty(t),
  }));

  return (
    <VariablePopoverContext.Provider
      value={{ activeEnvironment, onUpdateVariable: handleUpdateVariable }}
    >
      <div className="app-container">
        <header className="app-header">
          <div
            className="icon-rail"
            style={
              sidebarCollapsed
                ? undefined
                : {
                    width: `calc(${Math.max(SIDEBAR_MIN_WIDTH, sidebarWidth)}px - var(--traffic-light-inset))`,
                  }
            }
          >
            <button
              className="rail-icon rail-collapse-toggle"
              title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
              onClick={() => setSidebarCollapsed((v) => !v)}
            >
              {sidebarCollapsed ? (
                <ChevronsRight size={20} />
              ) : (
                <ChevronsLeft size={20} />
              )}
            </button>
            <button
              className={
                sidebarView === "collections" ? "rail-icon active" : "rail-icon"
              }
              title="Collections"
              onClick={() => {
                setSidebarView("collections");
                setSidebarCollapsed(false);
              }}
            >
              <Package size={20} />
            </button>
            <button
              className={
                sidebarView === "environments"
                  ? "rail-icon active"
                  : "rail-icon"
              }
              title="Environments"
              onClick={() => {
                setSidebarView("environments");
                setSidebarCollapsed(false);
              }}
            >
              <Monitor size={20} />
            </button>
            <button
              className={
                sidebarView === "history" ? "rail-icon active" : "rail-icon"
              }
              title="History"
              onClick={() => {
                setSidebarView("history");
                setSidebarCollapsed(false);
              }}
            >
              <Clock size={20} />
            </button>
            {!sidebarCollapsed && (
              <WorkspaceSwitcher
                workspaces={workspaces}
                activeWorkspace={activeWorkspace}
                onSelect={setActiveWorkspace}
                onCreate={handleCreateWorkspace}
                onWorkspaceUpdated={(ws) => {
                  setWorkspaces((prev) =>
                    prev.map((w) => (w.id === ws.id ? ws : w)),
                  );
                  if (activeWorkspace?.id === ws.id) setActiveWorkspace(ws);
                }}
              />
            )}
          </div>
          <TopTabBar
            tabs={openTabsForBar}
            activeTabId={activeTabId}
            onSelect={setActiveTabId}
            onClose={requestCloseTab}
            onForceClose={closeTabImmediately}
            onCloseOthers={closeOtherTabs}
            onCloseAll={closeAllTabs}
            onForceCloseAll={forceCloseAllTabs}
            onNewRequest={() => handleCreateRequest(null)}
            onDuplicateTab={handleDuplicateRequest}
            onRevealInSidebar={revealInSidebar}
            onOpenSettings={() => setSettingsModalOpen(true)}
          />
        </header>

        <div className="layout">
          <aside
            ref={sidebarRef}
            className={sidebarCollapsed ? "sidebar collapsed" : "sidebar"}
            style={
              sidebarCollapsed
                ? undefined
                : { width: Math.max(SIDEBAR_MIN_WIDTH, sidebarWidth) }
            }
          >
            {!sidebarCollapsed && sidebarView === "collections" && (
              <>
                <div className="sidebar-search-row">
                  <span className="search-icon">
                    <Search size={14} />
                  </span>
                  <input
                    className="sidebar-search-input"
                    placeholder="Filter"
                    value={treeFilter}
                    onChange={(e) => setTreeFilter(e.target.value)}
                  />
                  <button
                    onClick={() => handleCreateRequest(null)}
                    disabled={!activeWorkspace}
                    title="New request"
                  >
                    <Plus size={16} />
                  </button>
                  <button
                    onClick={() => setImportModalOpen(true)}
                    disabled={!activeWorkspace}
                    title="Import cURL / Postman / OpenAPI"
                  >
                    <Download size={16} />
                  </button>
                  <button
                    onClick={() => setExportModalOpen(true)}
                    disabled={!activeWorkspace}
                    title="Export to Postman / OpenAPI / cURL"
                  >
                    <Upload size={16} />
                  </button>
                </div>

                <div className="tree-scroll">
                  <div
                    className="section-header"
                    onClick={() => setCollectionsCollapsed((v) => !v)}
                  >
                    <span className="section-toggle">
                      {collectionsCollapsed ? (
                        <ChevronRight size={14} />
                      ) : (
                        <ChevronDown size={14} />
                      )}
                    </span>
                    <span>Collections</span>
                    <button
                      className="section-action"
                      onClick={(e) => {
                        e.stopPropagation();
                        setCollectionsCollapsed(false);
                        setNewTopLevelFolderSignal((n) => n + 1);
                      }}
                      disabled={!activeWorkspace}
                      title="New folder"
                    >
                      <FolderPlus size={14} /> Folder
                    </button>
                  </div>
                  {!collectionsCollapsed && activeWorkspace && (
                    <RequestTree
                      workspaceId={activeWorkspace.id}
                      folders={folders}
                      requests={requests}
                      activeRequestId={activeTab?.request.id ?? null}
                      filter={treeFilter}
                      onOpenRequest={openRequest}
                      onCreateFolder={handleCreateFolder}
                      onRenameFolder={handleRenameFolder}
                      onDeleteFolder={handleDeleteFolder}
                      onSetFolderAuth={handleSetFolderAuth}
                      onMoveRequest={handleMoveRequest}
                      onCreateRequest={handleCreateRequest}
                      onRenameRequest={handleRenameRequest}
                      onDeleteRequest={handleDeleteRequest}
                      onDuplicateRequest={handleDuplicateRequest}
                      onCopyRequestAsCurl={handleCopyRequestAsCurl}
                      onOpenSavedResponse={openSavedResponse}
                      savedResponsesRefreshSignal={savedResponsesRefreshSignal}
                      newTopLevelFolderSignal={newTopLevelFolderSignal}
                    />
                  )}
                </div>
              </>
            )}

            {!sidebarCollapsed &&
              sidebarView === "environments" &&
              activeWorkspace && (
                <div className="tree-scroll">
                  <div className="section-header">
                    <span>Environments</span>
                  </div>
                  <EnvironmentPanel
                    workspace={activeWorkspace}
                    onWorkspaceChange={(ws) => {
                      setActiveWorkspace(ws);
                      setWorkspaces((prev) =>
                        prev.map((w) => (w.id === ws.id ? ws : w)),
                      );
                    }}
                    onVariablesChanged={refreshVariableNames}
                  />
                </div>
              )}

            {!sidebarCollapsed &&
              sidebarView === "history" &&
              activeWorkspace && (
                <div className="tree-scroll">
                  <div className="section-header">
                    <span>History</span>
                  </div>
                  <HistoryPanel
                    workspaceId={activeWorkspace.id}
                    activeRequestId={activeTab?.request.id ?? null}
                    onReplay={handleHistoryReplay}
                    onConvertedToRequest={handleHistoryConverted}
                    refreshSignal={historyRefreshSignal}
                  />
                </div>
              )}

            {!sidebarCollapsed && (
              <div className="status-bar">
                <span className="status-bar-item">
                  <GitBranch size={14} style={{ marginRight: 4 }} /> Connect Git
                </span>
                <span className="status-bar-spacer" />
                <button
                  className={
                    consoleOpen ? "status-bar-btn active" : "status-bar-btn"
                  }
                  onClick={() => setConsoleOpen(!consoleOpen)}
                >
                  <Terminal size={14} style={{ marginRight: 4 }} /> Console
                  {consoleEntries.length > 0 && ` (${consoleEntries.length})`}
                </button>
                <button
                  className="status-bar-btn"
                  disabled
                  title="Terminal is not available in this build"
                >
                  <TerminalSquare size={14} style={{ marginRight: 4 }} />{" "}
                  Terminal
                </button>
                <span className="status-bar-item status-count">
                  <AlertTriangle size={14} style={{ marginRight: 4 }} />{" "}
                  {warningCount}
                </span>
              </div>
            )}
          </aside>

          {!sidebarCollapsed && (
            <div
              className="sidebar-resize-handle"
              onMouseDown={startSidebarResize}
            />
          )}

          {consoleOpen && (
            <ConsolePanel
              entries={consoleEntries}
              onClear={() => setConsoleEntries([])}
              onClose={() => setConsoleOpen(false)}
            />
          )}

          {importModalOpen && activeWorkspace && (
            <ImportModal
              workspaceId={activeWorkspace.id}
              onClose={() => setImportModalOpen(false)}
              onImported={() => {
                refreshRequests(activeWorkspace.id);
                refreshFolders(activeWorkspace.id);
              }}
            />
          )}

          {exportModalOpen && activeWorkspace && (
            <ExportModal
              workspaceId={activeWorkspace.id}
              workspaceName={activeWorkspace.name}
              activeRequestId={activeTab?.request.id ?? null}
              onClose={() => setExportModalOpen(false)}
            />
          )}

          {settingsModalOpen && (
            <SettingsModal onClose={() => setSettingsModalOpen(false)} />
          )}

          <main className="editor">
            {tabPendingClose && (
              <div
                className="env-editor-backdrop"
                onClick={() => setTabPendingClose(null)}
              >
                <div
                  className="env-editor close-confirm"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div className="env-editor-body">
                    <p>This tab has unsaved changes. Close it anyway?</p>
                  </div>
                  <div className="env-editor-footer">
                    <span />
                    <div className="env-editor-footer-right">
                      <button onClick={() => setTabPendingClose(null)}>
                        Cancel
                      </button>
                      <button
                        onClick={() => closeTabImmediately(tabPendingClose)}
                      >
                        Close without saving
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {activeTab && (
              <Breadcrumb
                workspaceName={activeWorkspace?.name ?? ""}
                folders={folders}
                currentFolderId={activeTab.request.folder_id}
                requestName={activeTab.request.name}
                savedResponseName={activeTab.savedResponseName ?? undefined}
              />
            )}

            {!activeTab && (
              <div className="empty-state">
                Select or create a request to get started.
              </div>
            )}

            {activeTab && (
              <div className="editor-content" ref={editorContentRef}>
                <div className="editor-content-top">
                  <div className="url-bar">
                    <select
                      className={`method-select method-${activeTab.request.method}`}
                      value={activeTab.request.method}
                      onChange={(e) =>
                        handleMethodChange(e.target.value as HttpMethod)
                      }
                    >
                      {METHODS.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))}
                    </select>
                    <VarInput
                      value={activeTab.request.url}
                      onChange={handleUrlChange}
                      onBlur={commitUrl}
                      placeholder="https://api.example.com/resource"
                      variableNames={variableNames}
                    />
                    <button
                      className="send-btn"
                      onClick={handleSend}
                      disabled={activeTab.sending}
                    >
                      {activeTab.sending ? "Sending…" : "Send"}
                    </button>
                  </div>

                  <RequestTabs
                    request={activeTab.request}
                    autoHeaders={autoHeaders}
                    variableNames={variableNames}
                    onHeadersChange={handleHeadersChange}
                    onHeadersCommit={commitHeaders}
                    onParamsChange={handleParamsChange}
                    onParamsCommit={commitParams}
                    onBodyChange={handleBodyChange}
                    onBodyCommit={commitBody}
                    onAuthChange={handleAuthChange}
                    onAuthCommit={commitAuth}
                    onNotesChange={handleNotesChange}
                    onNotesCommit={commitNotes}
                  />
                </div>

                {/* Height is user-controlled (see startResponsePanelResize) rather
                 * than plain flex space — dragging up past the request builder's
                 * own height overlaps/covers it (.response-panel-wrap's
                 * `position: absolute` + z-index in App.css) instead of being
                 * capped at "whatever flex space remains," matching the
                 * requested "extend to the full top, overlapping the section
                 * above" behavior. Dragging back down shrinks it and the
                 * request builder reappears underneath.
                 *
                 * The resize handle lives INSIDE .response-panel-wrap, pinned to
                 * its top edge via CSS, rather than as a normal-flow sibling
                 * between .editor-content-top and the wrap. The wrap is
                 * absolutely positioned from the bottom of the container, so its
                 * top edge does not line up with wherever a normal-flow sibling
                 * handle would render — at most panel heights the absolutely
                 * positioned wrap (z-index above everything) simply covered the
                 * handle, making it look present but be completely unclickable. */}
                <div
                  ref={responsePanelRef}
                  className="response-panel-wrap"
                  style={{ height: responsePanelHeight }}
                >
                  <div
                    className="response-resize-handle"
                    onMouseDown={startResponsePanelResize}
                    title="Drag to resize"
                  />
                  <ErrorBoundary fallback={<div style={{ padding: '2rem', textAlign: 'center', color: 'var(--danger)' }}>Failed to load response viewer. Please try again.</div>}>
                    <ResponsePanel
                      ref={responsePanelComponentRef}
                      response={activeTab.response}
                      error={activeTab.error}
                      sending={activeTab.sending}
                      onSaveResponse={handleSaveResponse}
                    />
                  </ErrorBoundary>
                </div>
              </div>
            )}
          </main>
        </div>
      </div>
    </VariablePopoverContext.Provider>
  );
}

export default App;
