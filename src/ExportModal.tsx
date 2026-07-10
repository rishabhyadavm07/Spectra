import { useState } from "react";
import { api } from "./api";

type ExportFormat = "postman" | "openapi" | "curl";

interface Props {
  workspaceId: string;
  workspaceName: string;
  activeRequestId: string | null;
  onClose: () => void;
}

const WORKSPACE_FORMATS: { value: ExportFormat; label: string; ext: string }[] =
  [
    { value: "postman", label: "Postman Collection (v2.1)", ext: "json" },
    { value: "openapi", label: "OpenAPI 3.0", ext: "json" },
  ];

const REQUEST_FORMATS: { value: ExportFormat; label: string; ext: string }[] = [
  { value: "curl", label: "cURL command (this request only)", ext: "sh" },
];

export function ExportModal({
  workspaceId,
  workspaceName,
  activeRequestId,
  onClose,
}: Props) {
  const [format, setFormat] = useState<ExportFormat>("postman");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  async function handleExport() {
    setBusy(true);
    setError(null);
    setOutput(null);
    setCopied(false);
    try {
      const text =
        format === "curl"
          ? await api.exportRequest(activeRequestId as string, "curl")
          : await api.exportWorkspace(workspaceId, format);
      setOutput(text);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function handleCopy() {
    if (!output) return;
    navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  function handleDownload() {
    if (!output) return;
    const opt = [...WORKSPACE_FORMATS, ...REQUEST_FORMATS].find(
      (o) => o.value === format,
    );
    const ext = opt?.ext ?? "txt";
    const base =
      format === "curl"
        ? "request"
        : workspaceName.replace(/[^a-z0-9-_]+/gi, "-").toLowerCase();
    const blob = new Blob([output], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${base}.${ext}`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="env-editor-backdrop" onClick={onClose}>
      <div
        className="env-editor import-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="env-editor-header">
          <span className="env-name-input import-title">Export</span>
        </div>

        <div className="env-editor-body">
          <div className="kv-row">
            <select
              value={format}
              onChange={(e) => setFormat(e.target.value as ExportFormat)}
            >
              <optgroup label="Whole workspace">
                {WORKSPACE_FORMATS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </optgroup>
              <optgroup label="Active request">
                {REQUEST_FORMATS.map((opt) => (
                  <option
                    key={opt.value}
                    value={opt.value}
                    disabled={!activeRequestId}
                  >
                    {opt.label}
                  </option>
                ))}
              </optgroup>
            </select>
            <button
              onClick={handleExport}
              disabled={busy || (format === "curl" && !activeRequestId)}
            >
              {busy ? "Exporting…" : "Generate"}
            </button>
          </div>

          {error && <div className="error-box">{error}</div>}

          {output && (
            <>
              <textarea
                className="body-editor import-textarea"
                spellCheck={false}
                readOnly
                value={output}
              />
              <div className="import-success">
                Generated {output.length.toLocaleString()} characters.
              </div>
            </>
          )}
        </div>

        <div className="env-editor-footer">
          <span />
          <div className="env-editor-footer-right">
            <button onClick={onClose}>Close</button>
            <button onClick={handleCopy} disabled={!output}>
              {copied ? "Copied!" : "Copy"}
            </button>
            <button onClick={handleDownload} disabled={!output}>
              Download
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
