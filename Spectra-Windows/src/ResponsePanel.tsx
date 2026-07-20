import {
  useEffect,
  useRef,
  useState,
  forwardRef,
  useImperativeHandle,
} from "react";
import Editor, { useMonaco } from "@monaco-editor/react";
import type { ResponseDto } from "./types";
import { useSettingsStore } from "./store/settings";

import {
  Braces,
  Code,
  AlignLeft,
  FileJson,
  ChevronDown,
  Check,
  WrapText,
  Copy,
  Play,
  Database,
} from "lucide-react";

type Tab = "body" | "headers";
type ViewMode = "raw" | "preview";
type BodyFormat = "json" | "xml" | "html" | "yaml" | "javascript" | "raw";

const FORMAT_OPTIONS: {
  value: BodyFormat;
  label: string;
  icon: React.ReactNode;
}[] = [
  { value: "json", label: "JSON", icon: <Braces size={14} /> },
  { value: "xml", label: "XML", icon: <Code size={14} /> },
  { value: "html", label: "HTML", icon: <Code size={14} /> },
  { value: "yaml", label: "YAML", icon: <AlignLeft size={14} /> },
  { value: "javascript", label: "JavaScript", icon: <FileJson size={14} /> },
  { value: "raw", label: "Raw", icon: <AlignLeft size={14} /> },
];

interface Props {
  response: ResponseDto | null;
  error: string | null;
  sending: boolean;
  onSaveResponse?: (name: string) => void;
}

function looksLikeHtml(body: string): boolean {
  return /<html[\s>]|<!doctype html/i.test(body.slice(0, 512));
}

function detectFormat(
  body: string,
  contentType: string | undefined,
): BodyFormat {
  const ct = (contentType ?? "").toLowerCase();
  if (ct.includes("json")) return "json";
  if (ct.includes("html")) return "html";
  if (ct.includes("xml")) return "xml";
  if (ct.includes("yaml")) return "yaml";
  if (ct.includes("javascript")) return "javascript";
  const trimmed = body.trimStart();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  if (looksLikeHtml(body)) return "html";
  if (trimmed.startsWith("<")) return "xml";
  return "raw";
}

// Remove synchronous tryFormatJson

export interface ResponsePanelRef {
  findMatches: (query: string) => any[];
  revealLine: (line: number) => void;
}

export const ResponsePanel = forwardRef<ResponsePanelRef, Props>(
  function ResponsePanel({ response, error, sending, onSaveResponse }, ref) {
    const [tab, setTab] = useState<Tab>("body");
    const [viewMode, setViewMode] = useState<ViewMode>("raw");
    const [format, setFormat] = useState<BodyFormat>("json");
    const showLineNumbers = useSettingsStore((s) => s.showLineNumbers);

    const [isDark, setIsDark] = useState(
      () =>
        document.documentElement.classList.contains("dark") ||
        document.documentElement.classList.contains("crimson"),
    );
    useEffect(() => {
      const observer = new MutationObserver(() => {
        setIsDark(
          document.documentElement.classList.contains("dark") ||
            document.documentElement.classList.contains("crimson"),
        );
      });
      observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["class"],
      });
      return () => observer.disconnect();
    }, []);

    const [formatMenuOpen, setFormatMenuOpen] = useState(false);
    const [formatTouched, setFormatTouched] = useState(false);
    const [wrap, setWrap] = useState(false);
    const [copied, setCopied] = useState(false);
    const [saving, setSaving] = useState(false);
    const [saveName, setSaveName] = useState("");
    const editorInstanceRef = useRef<any>(null);

    useImperativeHandle(
      ref,
      () => ({
        findMatches: (query: string) => {
          if (!editorInstanceRef.current) return [];
          const model = editorInstanceRef.current.getModel();
          if (!model) return [];
          return model.findMatches(query, false, false, false, null, true);
        },
        revealLine: (line: number) => {
          if (!editorInstanceRef.current) return;
          editorInstanceRef.current.revealLineInCenter(line);
        },
      }),
      [],
    );

    const formatMenuRef = useRef<HTMLDivElement>(null);
    const [schemaUrl, setSchemaUrl] = useState("");
    const [schemaPopoverOpen, setSchemaPopoverOpen] = useState(false);
    const schemaMenuRef = useRef<HTMLDivElement>(null);

    const monaco = useMonaco();

    const [formattedBody, setFormattedBody] = useState<string>("");
    const [isFormatting, setIsFormatting] = useState(false);
    const workerRef = useRef<Worker | null>(null);

    useEffect(() => {
      workerRef.current = new Worker(
        new URL("./worker/formatter.ts", import.meta.url),
        { type: "module" },
      );
      workerRef.current.onmessage = (e) => {
        setFormattedBody(e.data.result);
        setIsFormatting(false);
      };
      return () => {
        workerRef.current?.terminate();
      };
    }, []);

    // Format asynchronously when relevant state changes
    useEffect(() => {
      if (response && tab === "body" && viewMode === "raw") {
        if (format === "json" || format === "xml" || format === "html") {
          setIsFormatting(true);
          // We only really format JSON in the worker currently, but we pass everything through it
          workerRef.current?.postMessage({ body: response.body, format });
        } else {
          setFormattedBody(response.body);
          setIsFormatting(false);
        }
      }
    }, [response?.body, format, tab, viewMode]);

    useEffect(() => {
      if (monaco && format === "json" && schemaUrl) {
        // @ts-ignore
        monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
          validate: true,
          schemas: [
            {
              uri: schemaUrl,
              fileMatch: ["*"], // validate all models that are opened with this language
              schema: {
                $ref: schemaUrl,
              },
            },
          ],
        });
      } else if (monaco) {
        // @ts-ignore
        monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
          validate: true,
          schemas: [],
        });
      }
    }, [monaco, format, schemaUrl]);

    // Re-detect the best format whenever a new response arrives, unless the
    // user already picked one explicitly for the response currently shown.
    useEffect(() => {
      if (response) {
        setFormat(
          detectFormat(
            response.body,
            response.headers["content-type"] ??
              response.headers["Content-Type"],
          ),
        );
        setFormatTouched(false);
        setViewMode("raw");
      }
    }, [response]);

    useEffect(() => {
      function handleClickOutside(e: MouseEvent) {
        if (
          formatMenuRef.current &&
          !formatMenuRef.current.contains(e.target as Node)
        ) {
          setFormatMenuOpen(false);
        }
        if (
          schemaMenuRef.current &&
          !schemaMenuRef.current.contains(e.target as Node)
        ) {
          setSchemaPopoverOpen(false);
        }
      }
      if (formatMenuOpen || schemaPopoverOpen)
        document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }, [formatMenuOpen, schemaPopoverOpen]);

    function chooseFormat(f: BodyFormat) {
      setFormat(f);
      setFormatTouched(true);
      setFormatMenuOpen(false);
    }

    function startSave() {
      setSaveName("");
      setSaving(true);
    }

    function commitSave() {
      const name = saveName.trim();
      if (name && onSaveResponse) onSaveResponse(name);
      setSaving(false);
    }

    async function handleCopy() {
      if (!response) return;
      await navigator.clipboard.writeText(response.body);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    }

    const canPreview =
      format === "html" && response ? looksLikeHtml(response.body) : false;
    const activeFormatOption =
      FORMAT_OPTIONS.find((f) => f.value === format) ?? FORMAT_OPTIONS[0];

    return (
      <section className="panel response-panel">
        <div className="response-header-row">
          <div className="tab-bar">
            <button
              className={tab === "body" ? "tab active" : "tab"}
              onClick={() => setTab("body")}
            >
              Body
            </button>
            <button
              className={tab === "headers" ? "tab active" : "tab"}
              onClick={() => setTab("headers")}
            >
              Headers
              {response && (
                <span className="tab-count">
                  {Object.keys(response.headers).length}
                </span>
              )}
            </button>
          </div>

          {response && (
            <div className="response-meta">
              <span
                className={
                  response.status < 400
                    ? "status-badge status-ok"
                    : "status-badge status-err"
                }
              >
                {response.status} {response.status_text}
              </span>
              <span className="response-meta-item">
                {response.duration_ms} ms
              </span>
              <span className="response-meta-item">
                {formatBytes(response.size_bytes)}
              </span>
              {onSaveResponse && !saving && (
                <button className="save-response-btn" onClick={startSave}>
                  Save Response
                </button>
              )}
              {saving && (
                <form
                  className="save-response-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    commitSave();
                  }}
                >
                  <input
                    autoFocus
                    placeholder="Response name"
                    value={saveName}
                    onChange={(e) => setSaveName(e.target.value)}
                    onKeyDown={(e) => e.key === "Escape" && setSaving(false)}
                  />
                  <button type="submit">Save</button>
                  <button type="button" onClick={() => setSaving(false)}>
                    Cancel
                  </button>
                </form>
              )}
            </div>
          )}
        </div>

        {tab === "body" && response && !sending && !error && (
          <div className="body-toolbar">
            <div className="body-toolbar-left">
              <div className="format-dropdown" ref={formatMenuRef}>
                <button
                  className="format-badge"
                  onClick={() => setFormatMenuOpen((v) => !v)}
                >
                  {activeFormatOption.icon} {activeFormatOption.label}
                  {!formatTouched && (
                    <span className="format-auto-hint">auto</span>
                  )}
                  <ChevronDown size={14} className="format-caret" />
                </button>
                {formatMenuOpen && (
                  <div className="format-menu">
                    {FORMAT_OPTIONS.map((opt) => (
                      <button
                        key={opt.value}
                        className={
                          opt.value === format
                            ? "format-menu-item active"
                            : "format-menu-item"
                        }
                        onClick={() => chooseFormat(opt.value)}
                      >
                        {opt.value === format && (
                          <Check size={14} className="format-check" />
                        )}
                        <span className="format-menu-icon">{opt.icon}</span>
                        {opt.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>
              {canPreview && (
                <button
                  className={
                    viewMode === "preview"
                      ? "toolbar-btn active"
                      : "toolbar-btn"
                  }
                  onClick={() =>
                    setViewMode((m) => (m === "preview" ? "raw" : "preview"))
                  }
                >
                  <Play size={14} style={{ marginRight: 4 }} /> Preview
                </button>
              )}
            </div>
            <div className="body-toolbar-right">
              <button
                className={wrap ? "icon-btn active" : "icon-btn"}
                title="Wrap lines"
                onClick={() => setWrap((v) => !v)}
              >
                <WrapText size={14} />
              </button>
              <button
                className="icon-btn"
                title="Copy response body"
                onClick={handleCopy}
              >
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
              <div className="format-dropdown" ref={schemaMenuRef}>
                <button
                  className={schemaUrl ? "icon-btn active" : "icon-btn"}
                  title="JSON Schema Validation"
                  onClick={() => setSchemaPopoverOpen((v) => !v)}
                >
                  <Database size={14} style={{ marginRight: 4 }} /> Schema
                </button>
                {schemaPopoverOpen && (
                  <div
                    className="format-menu"
                    style={{
                      padding: "8px",
                      width: "250px",
                      left: "auto",
                      right: 0,
                    }}
                  >
                    <label
                      style={{
                        display: "block",
                        marginBottom: "4px",
                        fontSize: "12px",
                        fontWeight: 600,
                      }}
                    >
                      Schema URL
                    </label>
                    <input
                      type="url"
                      placeholder="https://json-schema.org/..."
                      value={schemaUrl}
                      onChange={(e) => setSchemaUrl(e.target.value)}
                      style={{
                        width: "100%",
                        padding: "4px 6px",
                        fontSize: "12px",
                      }}
                    />
                    <p
                      style={{
                        fontSize: "10px",
                        color: "var(--text-muted)",
                        marginTop: "4px",
                        marginBottom: 0,
                      }}
                    >
                      Enter a URL to an OpenAPI or JSON schema to validate this
                      response payload.
                    </p>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        <div className="response-content">
          {sending && <div className="empty-state">Sending…</div>}
          {!sending && error && <div className="error-box">{error}</div>}
          {!sending && !error && !response && (
            <div className="empty-state">No response yet.</div>
          )}

          {!sending &&
            !error &&
            response &&
            tab === "body" &&
            viewMode === "preview" && (
              <iframe
                title="Response preview"
                className="response-preview-frame"
                sandbox=""
                srcDoc={response.body}
              />
            )}

          {!sending &&
            !error &&
            response &&
            tab === "body" &&
            viewMode === "raw" && (
              <>
                {isFormatting ? (
                  <div className="empty-state">Formatting payload...</div>
                ) : (
                  <Editor
                    value={formattedBody}
                    language={format === "raw" ? "text" : format}
                    theme={isDark ? "vs-dark" : "vs"}
                    onMount={(editor) => {
                      editorInstanceRef.current = editor;
                    }}
                    options={{
                      readOnly: true,
                      wordWrap: wrap ? "on" : "off",
                      lineNumbers: showLineNumbers ? "on" : "off",
                      minimap: { enabled: false },
                      scrollBeyondLastLine: false,
                      automaticLayout: true,
                      formatOnPaste: true,
                      formatOnType: true,
                    }}
                  />
                )}
              </>
            )}

          {!sending && !error && response && tab === "headers" && (
            <div className="response-headers-table">
              {Object.entries(response.headers).map(([key, value]) => (
                <div className="kv-row auto-header-row" key={key}>
                  <span className="auto-header-key">{key}</span>
                  <span className="auto-header-value">{value}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </section>
    );
  },
);

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
