import { useState, useEffect } from "react";
import { api } from "./api";
import { AuthPanel } from "./AuthPanel";
import type {
  AuthConfig,
  HeaderEntry,
  ParamEntry,
  RequestBody,
  SpectraRequest,
  HistoryEntry,
} from "./types";
import { VarInput } from "./VarInput";
import { VarTextarea } from "./VarTextarea";
import { Plus } from "lucide-react";

type Tab = "params" | "headers" | "auth" | "body" | "notes" | "history";

const MAX_NOTES_WORDS = 50;

function countWords(text: string): number {
  const trimmed = text.trim();
  if (trimmed === "") return 0;
  return trimmed.split(/\s+/).length;
}

/** Soft-prevents typing past the word cap, mirroring how Twitter/X limits
 * character count: once at the cap, further plain typing (adding a new word)
 * is rejected, but the user can still delete/edit existing words freely.
 * The backend (`set_notes`) also truncates defensively for any caller that
 * bypasses this UI (e.g. an MCP client), so this is a UX nicety, not the
 * only enforcement. */
function clampNotesInput(current: string, next: string): string {
  if (countWords(next) <= MAX_NOTES_WORDS) return next;
  // Already over the cap and getting longer — likely more typing; only
  // allow edits that don't increase the word count (e.g. editing a word in
  // place), otherwise keep the previous value.
  return countWords(next) > countWords(current) ? current : next;
}

const BODY_KINDS: { value: RequestBody["kind"]; label: string }[] = [
  { value: "None", label: "None" },
  { value: "Json", label: "JSON" },
  { value: "Text", label: "Text" },
  { value: "Xml", label: "XML" },
  { value: "FormUrlEncoded", label: "Form URL Encoded" },
];

function defaultBody(kind: RequestBody["kind"]): RequestBody {
  switch (kind) {
    case "Json":
      return { kind, content: "" };
    case "Text":
      return { kind, content: "" };
    case "Xml":
      return { kind, content: "" };
    case "FormUrlEncoded":
      return { kind, fields: [] };
    default:
      return { kind: "None" };
  }
}

function enabledCount(entries: { enabled: boolean }[]): number {
  return entries.filter((e) => e.enabled).length;
}

interface Props {
  request: SpectraRequest;
  autoHeaders: [string, string][];
  variableNames: string[];
  onHeadersChange: (headers: HeaderEntry[]) => void;
  onHeadersCommit: (headers: HeaderEntry[]) => void;
  onParamsChange: (params: ParamEntry[]) => void;
  onParamsCommit: (params: ParamEntry[]) => void;
  onBodyChange: (body: RequestBody) => void;
  onBodyCommit: (body: RequestBody) => void;
  onAuthChange: (auth: AuthConfig) => void;
  onAuthCommit: (auth: AuthConfig) => void;
  onNotesChange: (notes: string) => void;
  onNotesCommit: (notes: string) => void;
  onViewHistory: (entry: HistoryEntry) => void;
}

export function RequestTabs({
  request,
  autoHeaders,
  variableNames,
  onHeadersChange,
  onHeadersCommit,
  onParamsChange,
  onParamsCommit,
  onBodyChange,
  onBodyCommit,
  onAuthChange,
  onAuthCommit,
  onNotesChange,
  onNotesCommit,
  onViewHistory,
}: Props) {
  const [tab, setTab] = useState<Tab>("params");
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);

  useEffect(() => {
    if (tab === "history") {
      api.listHistoryForRequest(request.workspace_id, request.id).then(setHistoryEntries);
    }
  }, [tab, request.workspace_id, request.id]);

  const visibleAutoHeaders = autoHeaders.filter(
    ([key]) =>
      !request.headers.some(
        (h) => h.enabled && h.key.toLowerCase() === key.toLowerCase(),
      ),
  );

  function updateHeaderRow(
    index: number,
    field: keyof HeaderEntry,
    value: string | boolean,
  ) {
    const headers = [...request.headers];
    headers[index] = { ...headers[index], [field]: value };
    onHeadersChange(headers);
  }

  function updateParamRow(
    index: number,
    field: keyof ParamEntry,
    value: string | boolean,
  ) {
    const params = [...request.params];
    params[index] = { ...params[index], [field]: value };
    onParamsChange(params);
  }

  function updateFormField(
    index: number,
    field: keyof ParamEntry,
    value: string | boolean,
  ) {
    if (request.body.kind !== "FormUrlEncoded") return;
    const fields = [...request.body.fields];
    fields[index] = { ...fields[index], [field]: value };
    onBodyChange({ kind: "FormUrlEncoded", fields });
  }

  return (
    <section className="panel tabbed-panel">
      <div className="tab-bar">
        <button
          className={tab === "params" ? "tab active" : "tab"}
          onClick={() => setTab("params")}
        >
          Params
          {request.params.length > 0 && (
            <span className="tab-count">{enabledCount(request.params)}</span>
          )}
        </button>
        <button
          className={tab === "headers" ? "tab active" : "tab"}
          onClick={() => setTab("headers")}
        >
          Headers
          {request.headers.length > 0 && (
            <span className="tab-count">{enabledCount(request.headers)}</span>
          )}
        </button>
        <button
          className={tab === "auth" ? "tab active" : "tab"}
          onClick={() => setTab("auth")}
        >
          Auth
          {request.auth.type !== "None" && <span className="tab-count">1</span>}
        </button>
        <button
          className={tab === "body" ? "tab active" : "tab"}
          onClick={() => setTab("body")}
        >
          Body
          {request.body.kind !== "None" && <span className="tab-count">•</span>}
        </button>
        <button
          className={tab === "notes" ? "tab active" : "tab"}
          onClick={() => setTab("notes")}
        >
          Notes
          {request.notes.trim() !== "" && <span className="tab-count">•</span>}
        </button>
        <button
          className={tab === "history" ? "tab active" : "tab"}
          onClick={() => setTab("history")}
        >
          History
        </button>
      </div>

      <div className="tab-content">
        {tab === "params" && (
          <div className="tab-pane">
            {request.params.map((p, i) => (
              <div className="kv-row" key={i}>
                <input
                  type="checkbox"
                  checked={p.enabled}
                  onChange={(e) => {
                    updateParamRow(i, "enabled", e.target.checked);
                    onParamsCommit(request.params);
                  }}
                />
                <VarInput
                  placeholder="Key"
                  value={p.key}
                  onChange={(v) => updateParamRow(i, "key", v)}
                  onBlur={() => onParamsCommit(request.params)}
                  variableNames={variableNames}
                />
                <VarInput
                  placeholder="Value"
                  value={p.value}
                  onChange={(v) => updateParamRow(i, "value", v)}
                  onBlur={() => onParamsCommit(request.params)}
                  variableNames={variableNames}
                />
              </div>
            ))}
            <button
              onClick={() =>
                onParamsChange([
                  ...request.params,
                  { key: "", value: "", enabled: true },
                ])
              }
            >
              <Plus size={14} style={{ marginRight: 4 }} /> Param
            </button>
          </div>
        )}

        {tab === "headers" && (
          <div className="tab-pane">
            {request.headers.map((h, i) => (
              <div className="kv-row" key={i}>
                <input
                  type="checkbox"
                  checked={h.enabled}
                  onChange={(e) => {
                    updateHeaderRow(i, "enabled", e.target.checked);
                    onHeadersCommit(request.headers);
                  }}
                />
                <VarInput
                  placeholder="Key"
                  value={h.key}
                  onChange={(v) => updateHeaderRow(i, "key", v)}
                  onBlur={() => onHeadersCommit(request.headers)}
                  variableNames={variableNames}
                />
                <VarInput
                  placeholder="Value"
                  value={h.value}
                  onChange={(v) => updateHeaderRow(i, "value", v)}
                  onBlur={() => onHeadersCommit(request.headers)}
                  variableNames={variableNames}
                />
              </div>
            ))}
            <button
              onClick={() =>
                onHeadersChange([
                  ...request.headers,
                  { key: "", value: "", enabled: true },
                ])
              }
            >
              <Plus size={14} style={{ marginRight: 4 }} /> Header
            </button>

            {visibleAutoHeaders.length > 0 && (
              <div className="auto-headers">
                <div className="auto-headers-label">Auto-generated</div>
                {visibleAutoHeaders.map(([key, value]) => (
                  <div className="kv-row auto-header-row" key={key}>
                    <span className="auto-header-key">{key}</span>
                    <span className="auto-header-value">{value}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {tab === "auth" && (
          <div className="tab-pane">
            <AuthPanel
              requestId={request.id}
              workspaceId={request.workspace_id}
              auth={request.auth}
              onChange={onAuthChange}
              onCommit={onAuthCommit}
              variableNames={variableNames}
            />
          </div>
        )}

        {tab === "body" && (
          <div className="tab-pane">
            <div className="body-kind-row">
              {BODY_KINDS.map((k) => (
                <label key={k.value} className="radio-label">
                  <input
                    type="radio"
                    name="body-kind"
                    checked={request.body.kind === k.value}
                    onChange={() => {
                      const next = defaultBody(k.value);
                      onBodyChange(next);
                      onBodyCommit(next);
                    }}
                  />
                  {k.label}
                </label>
              ))}
            </div>

            {(request.body.kind === "Json" ||
              request.body.kind === "Text" ||
              request.body.kind === "Xml") && (
              <VarTextarea
                className="body-editor"
                spellCheck={false}
                placeholder={request.body.kind === "Json" ? "{\n  \n}" : ""}
                value={request.body.content}
                onChange={(v) =>
                  onBodyChange({
                    kind: request.body.kind,
                    content: v,
                  } as RequestBody)
                }
                onBlur={() => onBodyCommit(request.body)}
                variableNames={variableNames}
              />
            )}

            {request.body.kind === "FormUrlEncoded" && (
              <div>
                {request.body.fields.map((f, i) => (
                  <div className="kv-row" key={i}>
                    <input
                      type="checkbox"
                      checked={f.enabled}
                      onChange={(e) => {
                        updateFormField(i, "enabled", e.target.checked);
                        onBodyCommit(request.body);
                      }}
                    />
                    <VarInput
                      placeholder="Key"
                      value={f.key}
                      onChange={(v) => updateFormField(i, "key", v)}
                      onBlur={() => onBodyCommit(request.body)}
                      variableNames={variableNames}
                    />
                    <VarInput
                      placeholder="Value"
                      value={f.value}
                      onChange={(v) => updateFormField(i, "value", v)}
                      onBlur={() => onBodyCommit(request.body)}
                      variableNames={variableNames}
                    />
                  </div>
                ))}
                <button
                  onClick={() => {
                    if (request.body.kind !== "FormUrlEncoded") return;
                    onBodyChange({
                      kind: "FormUrlEncoded",
                      fields: [
                        ...request.body.fields,
                        { key: "", value: "", enabled: true },
                      ],
                    });
                  }}
                >
                  <Plus size={14} style={{ marginRight: 4 }} /> Field
                </button>
              </div>
            )}

            {request.body.kind === "None" && (
              <div className="hint-text">
                This request does not have a body.
              </div>
            )}
          </div>
        )}

        {tab === "notes" && (
          <div className="tab-pane">
            <textarea
              className="notes-editor"
              placeholder="Notes about this request (max 50 words)…"
              spellCheck={false}
              value={request.notes}
              onChange={(e) =>
                onNotesChange(clampNotesInput(request.notes, e.target.value))
              }
              onBlur={() => onNotesCommit(request.notes)}
            />
            <div className="notes-footer">
              <span className="notes-count">
                {countWords(request.notes)} / {MAX_NOTES_WORDS} words
              </span>
            </div>
          </div>
        )}

        {tab === "history" && (
          <div className="tab-pane" style={{ padding: "0.5em 1em" }}>
            {historyEntries.length === 0 ? (
              <div className="empty-state">No history for this request yet.</div>
            ) : (
              <ul className="history-list" style={{ position: "relative", height: "auto" }}>
                {historyEntries.map((entry) => (
                  <li key={entry.id} className="history-row" style={{ position: "relative" }}>
                    <div className="history-row-main" onClick={() => onViewHistory(entry)}>
                      <span className={`method-badge method-${entry.request_snapshot.method}`}>
                        {entry.request_snapshot.method}
                      </span>
                      <div className="history-row-info">
                        <span className="history-url">{entry.request_snapshot.url}</span>
                        <span className="history-meta">
                          {entry.error ? (
                            <span className="status-err">Failed</span>
                          ) : (
                            <span className={entry.response && entry.response.status < 400 ? "status-ok" : "status-err"}>
                              {entry.response?.status}
                            </span>
                          )}
                          {" · "}
                          {new Date(entry.executed_at).toLocaleString()}
                        </span>
                      </div>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>
    </section>
  );
}
