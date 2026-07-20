import { useEffect, useRef, useState } from "react";
import { ContextMenu } from "./ContextMenu";
import type { ContextMenuItem } from "./ContextMenu";
import type { HttpMethod } from "./types";
import { X, ChevronsRight, Settings, Cookie } from "lucide-react";
import { api } from "./api";

export interface OpenTab {
  /** Stable key for this tab; the request's own id, or a saved-response id
   * prefixed so it can't collide with a request id. */
  tabId: string;
  requestId: string;
  name: string;
  method: HttpMethod;
  dirty: boolean;
}

interface Props {
  tabs: OpenTab[];
  activeTabId: string | null;
  onSelect: (tabId: string) => void;
  onClose: (tabId: string) => void;
  onForceClose: (tabId: string) => void;
  onCloseOthers: (tabId: string) => void;
  onCloseAll: () => void;
  onForceCloseAll: () => void;
  onNewRequest: () => void;
  onDuplicateTab: (requestId: string) => void;
  onRevealInSidebar: (requestId: string) => void;
  onOpenSettings?: () => void;
}

// Tabs shrink toward this width (in px) as more open — Chrome-style — before
// any of them get pushed into the "»" overflow dropdown. Chosen to still fit
// a method badge, a few characters of the request name, and the close
// button (see App.css's `.top-tab` rules for the exact layout).
const TAB_MIN_WIDTH = 100;
const TAB_MAX_WIDTH = 220;
// Reserved width for the trailing "»" overflow button, so the fit
// calculation below doesn't measure against the bar's full width when an
// overflow button is about to appear.
const OVERFLOW_BUTTON_WIDTH = 36;

export function TopTabBar({
  tabs,
  activeTabId,
  onSelect,
  onClose,
  onForceClose,
  onCloseOthers,
  onCloseAll,
  onForceCloseAll,
  onNewRequest,
  onDuplicateTab,
  onRevealInSidebar,
  onOpenSettings,
}: Props) {
  const [menuFor, setMenuFor] = useState<{
    tab: OpenTab;
    x: number;
    y: number;
  } | null>(null);
  const [overflowMenuOpen, setOverflowMenuOpen] = useState(false);
  const [visibleCount, setVisibleCount] = useState<number>(tabs.length);
  const containerRef = useRef<HTMLDivElement>(null);
  const overflowBtnRef = useRef<HTMLButtonElement>(null);

  // Recompute how many tabs fit at their minimum width whenever the
  // container is resized or the tab list itself changes (new tab is why
  // overflow first appears; a tab closing is why it might disappear again).
  // This can't be done in pure CSS: flexbox can shrink tabs, but "move the
  // ones that don't fit into a dropdown" requires knowing the actual fitted
  // count, which needs a real width measurement.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    function recompute() {
      const containerWidth = el!.clientWidth;
      const n = tabs.length;
      if (n === 0) {
        setVisibleCount(0);
        return;
      }
      // Everything fits at (up to) max width with no overflow button needed.
      if (n * TAB_MIN_WIDTH <= containerWidth) {
        setVisibleCount(n);
        return;
      }
      // Some tabs must overflow — reserve space for the "»" button and fit
      // as many as possible at their minimum width.
      const usable = containerWidth - OVERFLOW_BUTTON_WIDTH;
      const fits = Math.max(1, Math.floor(usable / TAB_MIN_WIDTH));
      setVisibleCount(Math.min(fits, n));
    }

    recompute();
    const observer = new ResizeObserver(recompute);
    observer.observe(el);
    return () => observer.disconnect();
  }, [tabs.length]);

  // Remove early return so the bar still renders with the settings button
  // if (tabs.length === 0) return null;

  const overflowStart = Math.min(visibleCount, tabs.length);
  const visibleTabs = tabs.slice(0, overflowStart);
  const overflowedTabs = tabs.slice(overflowStart);
  const activeIsOverflowed = overflowedTabs.some(
    (t) => t.tabId === activeTabId,
  );

  // Tabs shrink toward TAB_MIN_WIDTH as more of them are visible, up to
  // TAB_MAX_WIDTH when there's only a few — real Chrome behavior.
  const tabBasis =
    visibleTabs.length > 0
      ? Math.max(
          TAB_MIN_WIDTH,
          TAB_MAX_WIDTH / Math.max(1, visibleTabs.length / 3),
        )
      : TAB_MAX_WIDTH;

  function menuItems(tab: OpenTab): ContextMenuItem[] {
    return [
      { label: "New Request", onClick: onNewRequest, shortcut: "⌘T" },
      { label: "Duplicate Tab", onClick: () => onDuplicateTab(tab.requestId) },
      {
        label: "Close Tab",
        onClick: () => onClose(tab.tabId),
        shortcut: "⌘W",
        separatorBefore: true,
      },
      {
        label: "Force Close Tab",
        onClick: () => onForceClose(tab.tabId),
        shortcut: "⌥⌘W",
      },
      { label: "Close Other Tabs", onClick: () => onCloseOthers(tab.tabId) },
      { label: "Close All Tabs", onClick: onCloseAll },
      { label: "Force Close All Tabs", onClick: onForceCloseAll },
      {
        label: "Reveal in Sidebar",
        onClick: () => onRevealInSidebar(tab.requestId),
        separatorBefore: true,
      },
    ];
  }

  function overflowMenuItems(): ContextMenuItem[] {
    return overflowedTabs.map((tab) => ({
      label: `${tab.method} ${tab.name}${tab.dirty ? " •" : ""}`,
      onClick: () => onSelect(tab.tabId),
    }));
  }

  function renderTab(tab: OpenTab) {
    return (
      <div
        key={tab.tabId}
        className={tab.tabId === activeTabId ? "top-tab active" : "top-tab"}
        style={{
          flexBasis: tabBasis,
          maxWidth: TAB_MAX_WIDTH,
          minWidth: TAB_MIN_WIDTH,
        }}
        onClick={() => onSelect(tab.tabId)}
        onContextMenu={(e) => {
          e.preventDefault();
          setMenuFor({ tab, x: e.clientX, y: e.clientY });
        }}
      >
        <span className={`method-badge method-${tab.method}`}>
          {tab.method}
        </span>
        <span className="top-tab-name">{tab.name}</span>
        {tab.dirty && (
          <span className="top-tab-dirty" title="Unsaved changes" />
        )}
        <button
          className="top-tab-close"
          title="Close tab"
          onClick={(e) => {
            e.stopPropagation();
            onClose(tab.tabId);
          }}
        >
          <X size={14} />
        </button>
      </div>
    );
  }

  return (
    <div className="top-tab-bar" ref={containerRef}>
      {visibleTabs.map(renderTab)}

      {overflowedTabs.length > 0 && (
        <button
          ref={overflowBtnRef}
          className={
            activeIsOverflowed
              ? "top-tab-overflow-btn has-active"
              : "top-tab-overflow-btn"
          }
          title={`${overflowedTabs.length} more tab${overflowedTabs.length === 1 ? "" : "s"}`}
          onClick={() => setOverflowMenuOpen((v) => !v)}
        >
          <ChevronsRight size={14} style={{ marginRight: 4 }} />{" "}
          {overflowedTabs.length}
          {activeIsOverflowed && (
            <span
              className="top-tab-overflow-dot"
              title="Active tab is hidden"
            />
          )}
        </button>
      )}

      {overflowMenuOpen && overflowBtnRef.current && (
        <ContextMenu
          x={overflowBtnRef.current.getBoundingClientRect().left}
          y={overflowBtnRef.current.getBoundingClientRect().bottom + 4}
          items={overflowMenuItems()}
          onClose={() => setOverflowMenuOpen(false)}
        />
      )}

      <div style={{ flex: 1 }} />
      <button
        className="top-tab-settings-btn"
        title="Clear Cookies"
        onClick={() => {
          api.clearCookies().then(() => alert("Cookies cleared."));
        }}
        style={{
          background: "none",
          border: "none",
          color: "var(--text-muted)",
          cursor: "pointer",
          padding: "0 12px",
          display: "flex",
          alignItems: "center",
        }}
      >
        <Cookie size={16} />
      </button>
      {onOpenSettings && (
        <button
          className="top-tab-settings-btn"
          title="Settings"
          onClick={onOpenSettings}
          style={{
            background: "none",
            border: "none",
            color: "var(--text-muted)",
            cursor: "pointer",
            padding: "0 12px",
            display: "flex",
            alignItems: "center",
          }}
        >
          <Settings size={16} />
        </button>
      )}

      {menuFor && (
        <ContextMenu
          x={menuFor.x}
          y={menuFor.y}
          items={menuItems(menuFor.tab)}
          onClose={() => setMenuFor(null)}
        />
      )}
    </div>
  );
}
