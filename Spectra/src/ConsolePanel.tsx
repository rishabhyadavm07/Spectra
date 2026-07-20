import { X } from "lucide-react";

export interface ConsoleLogEntry {
  id: string;
  method: string;
  url: string;
  status: number | null;
  error: string | null;
  durationMs: number | null;
  timestamp: string;
}

interface Props {
  entries: ConsoleLogEntry[];
  onClear: () => void;
  onClose: () => void;
}

export function ConsolePanel({ entries, onClear, onClose }: Props) {
  return (
    <div className="console-panel">
      <div className="console-header">
        <span className="console-title">Console</span>
        <span className="console-header-spacer" />
        <button
          className="toolbar-btn"
          onClick={onClear}
          disabled={entries.length === 0}
        >
          Clear
        </button>
        <button className="icon-btn" title="Close console" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="console-log">
        {entries.length === 0 && (
          <div className="empty-state console-empty">
            No requests logged yet.
          </div>
        )}
        {entries.map((e) => (
          <div className="console-row" key={e.id}>
            <span className="console-time">
              {new Date(e.timestamp).toLocaleTimeString()}
            </span>
            <span className={`method-badge method-${e.method}`}>
              {e.method}
            </span>
            <span className="console-url">{e.url}</span>
            {e.error ? (
              <span className="status-err console-status">Error</span>
            ) : (
              <span
                className={
                  e.status && e.status < 400
                    ? "status-ok console-status"
                    : "status-err console-status"
                }
              >
                {e.status}
              </span>
            )}
            {e.durationMs !== null && (
              <span className="console-duration">{e.durationMs} ms</span>
            )}
            {e.error && <span className="console-error-text">{e.error}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
