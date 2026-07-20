import { useEffect, useState, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "./api";
import type { HistoryEntry } from "./types";
import { Save, Trash2 } from "lucide-react";

interface Props {
  workspaceId: string;
  /** The request currently open in the active tab, if any — lets this panel
   * offer a "This Request" scope showing just its last 5 sends. `null` when
   * no tab is open (the toggle still renders but "This Request" is disabled). */
  activeRequestId: string | null;
  onView: (entry: HistoryEntry) => void;
  onConvertedToRequest: () => void;
  refreshSignal: number;
}

type Scope = "request" | "all";

function timeAgo(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  const s = Math.floor(diffMs / 1000);
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

export function HistoryPanel({
  workspaceId,
  activeRequestId,
  onView,
  onConvertedToRequest,
  refreshSignal,
}: Props) {
  // Default to "This Request" whenever a request is open — that's almost
  // always what the user wants ("what did I just send from here") — and
  // fall back to "All" when there's no active tab to scope to.
  const [scope, setScope] = useState<Scope>(
    activeRequestId ? "request" : "all",
  );
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const scrollParentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollParentRef.current,
    estimateSize: () => 45, // approx height of .history-row
    overscan: 10,
  });

  // If the active request changes (user switches tabs) while scoped to
  // "This Request", re-scope to the new request rather than silently
  // continuing to show the previous request's history.
  useEffect(() => {
    if (!activeRequestId && scope === "request") setScope("all");
  }, [activeRequestId, scope]);

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceId, refreshSignal, scope, activeRequestId]);

  async function refresh() {
    if (scope === "request" && activeRequestId) {
      setEntries(await api.listHistoryForRequest(workspaceId, activeRequestId));
    } else {
      setEntries(await api.listHistory(workspaceId));
    }
  }

  function handleView(entry: HistoryEntry) {
    onView(entry);
  }

  async function handleDelete(id: string) {
    await api.deleteHistoryEntry(workspaceId, id);
    refresh();
  }

  async function handleSaveAsRequest(id: string) {
    await api.convertHistoryToRequest(workspaceId, id, null);
    onConvertedToRequest();
  }

  return (
    <div className="history-panel">
      <div className="history-scope-toggle">
        <button
          className={
            scope === "request"
              ? "history-scope-btn active"
              : "history-scope-btn"
          }
          disabled={!activeRequestId}
          title={
            activeRequestId
              ? "Last 5 sends for the currently open request"
              : "Open a request to see its history"
          }
          onClick={() => setScope("request")}
        >
          This Request
        </button>
        <button
          className={
            scope === "all" ? "history-scope-btn active" : "history-scope-btn"
          }
          onClick={() => setScope("all")}
        >
          All
        </button>
      </div>

      {entries.length === 0 ? (
        <div className="empty-state history-empty">
          {scope === "request"
            ? "No history yet for this request."
            : "No requests sent yet."}
        </div>
      ) : (
        <div
          ref={scrollParentRef}
          style={{ height: "100%", overflowY: "auto", overflowX: "hidden" }}
        >
          <ul
            className="history-list"
            style={{
              position: "relative",
              height: `${rowVirtualizer.getTotalSize()}px`,
              display: "block",
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const entry = entries[virtualRow.index];
              return (
                <li
                  key={entry.id}
                  className="history-row"
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <div
                    className="history-row-main"
                    onClick={() => handleView(entry)}
                  >
                    <span
                      className={`method-badge method-${entry.request_snapshot.method}`}
                    >
                      {entry.request_snapshot.method}
                    </span>
                    <div className="history-row-info">
                      <span className="history-url">
                        {entry.request_snapshot.url}
                      </span>
                      <span className="history-meta">
                        {entry.error ? (
                          <span className="status-err">Failed</span>
                        ) : (
                          <span
                            className={
                              entry.response && entry.response.status < 400
                                ? "status-ok"
                                : "status-err"
                            }
                          >
                            {entry.response?.status}
                          </span>
                        )}
                        {" · "}
                        {timeAgo(entry.executed_at)}
                      </span>
                    </div>
                  </div>
                  <span className="history-row-actions">
                    <button
                      title="Save as request"
                      onClick={() => handleSaveAsRequest(entry.id)}
                    >
                      <Save size={14} />
                    </button>
                    <button
                      title="Delete"
                      className="danger-btn"
                      onClick={() => handleDelete(entry.id)}
                    >
                      <Trash2 size={14} />
                    </button>
                  </span>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}
