import { useState } from "react";
import { api } from "./api";
import type { ImportFormat } from "./types";

interface Props {
  workspaceId: string;
  onClose: () => void;
  onImported: () => void;
}

const FORMAT_OPTIONS: { value: ImportFormat | ""; label: string }[] = [
  { value: "", label: "Auto-detect" },
  { value: "curl", label: "cURL command" },
  { value: "postman", label: "Postman Collection (v2.1)" },
  { value: "openapi", label: "OpenAPI / Swagger" },
  { value: "har", label: "HAR (HTTP Archive)" },
];

export function ImportModal({ workspaceId, onClose, onImported }: Props) {
  const [format, setFormat] = useState<ImportFormat | "">("");
  const [content, setContent] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<{
    imported_count: number;
    saved_response_count: number;
  } | null>(null);

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const text = await file.text();
    setContent(text);
    setError(null);
    if (!format) {
      if (file.name.endsWith(".curl") || file.name.endsWith(".sh"))
        setFormat("curl");
      else if (file.name.endsWith(".yaml") || file.name.endsWith(".yml"))
        setFormat("openapi");
      else if (file.name.endsWith(".har")) setFormat("har");
    }
  }

  async function handleImport() {
    if (!content.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const res = await api.importCollection(
        workspaceId,
        content,
        format || undefined,
      );
      setResult({
        imported_count: res.imported_count,
        saved_response_count: res.saved_response_count,
      });
      onImported();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="env-editor-backdrop" onClick={onClose}>
      <div
        className="env-editor import-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="env-editor-header">
          <span className="env-name-input import-title">Import</span>
        </div>

        <div className="env-editor-body">
          <div className="kv-row">
            <select
              value={format}
              onChange={(e) => setFormat(e.target.value as ImportFormat | "")}
            >
              {FORMAT_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            <input
              type="file"
              accept=".json,.yaml,.yml,.curl,.sh,.txt,.har"
              onChange={handleFileChange}
            />
          </div>

          <textarea
            className="body-editor import-textarea"
            spellCheck={false}
            placeholder={
              "Paste a curl command, a Postman collection export (JSON), an OpenAPI/Swagger spec (JSON or YAML), or a HAR file (JSON)…"
            }
            value={content}
            onChange={(e) => setContent(e.target.value)}
          />

          {error && <div className="error-box">{error}</div>}
          {result && (
            <div className="import-success">
              Imported {result.imported_count} request
              {result.imported_count === 1 ? "" : "s"}
              {result.saved_response_count > 0 &&
                ` with ${result.saved_response_count} saved response${result.saved_response_count === 1 ? "" : "s"}`}
              .
            </div>
          )}
        </div>

        <div className="env-editor-footer">
          <span />
          <div className="env-editor-footer-right">
            <button onClick={onClose}>{result ? "Close" : "Cancel"}</button>
            <button onClick={handleImport} disabled={busy || !content.trim()}>
              {busy ? "Importing…" : "Import"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
