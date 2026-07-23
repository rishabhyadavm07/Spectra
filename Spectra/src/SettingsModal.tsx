import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Moon, Sun, ShieldOff, Clock, Monitor, Droplet } from "lucide-react";
import type { AppSettings } from "./types";
import { applyTheme } from "./theme";

interface Props {
  onClose: () => void;
}

export function SettingsModal({ onClose }: Props) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"general" | "mcp">("general");

  useEffect(() => {
    invoke<AppSettings>("get_settings").then(setSettings).catch(setError);
  }, []);

  async function handleSave() {
    if (!settings) return;
    setSaving(true);
    try {
      await invoke("save_settings", { settings });
      // Apply theme immediately
      applyTheme(settings.theme || "system");
      onClose();
    } catch (err: any) {
      setError(err.toString());
    } finally {
      setSaving(false);
    }
  }

  if (!settings) {
    return (
      <div className="env-editor-backdrop" onClick={onClose}>
        <div
          className="env-editor"
          onClick={(e) => e.stopPropagation()}
          style={{ width: 600, minHeight: 400 }}
        >
          <div className="env-editor-header">
            <h3>Settings</h3>
            <button className="env-editor-close" onClick={onClose}>
              <X size={16} />
            </button>
          </div>
          <div className="env-editor-body">Loading...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="env-editor-backdrop" onClick={onClose}>
      <div
        className="env-editor"
        onClick={(e) => e.stopPropagation()}
        style={{ width: 600, minHeight: 400 }}
      >
        <div className="env-editor-header">
          <h3>Settings</h3>
          <button className="env-editor-close" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="tab-bar">
          <button
            className={`tab ${activeTab === "general" ? "active" : ""}`}
            onClick={() => setActiveTab("general")}
          >
            General
          </button>
          <button
            className={`tab ${activeTab === "mcp" ? "active" : ""}`}
            onClick={() => setActiveTab("mcp")}
          >
            MCP Server
          </button>
        </div>
        <div
          className="tab-content"
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "20px",
            minHeight: "300px",
          }}
        >
          {error && <div className="error-box">{error}</div>}

          {activeTab === "general" && (
            <div
              style={{ display: "flex", flexDirection: "column", gap: "24px" }}
            >
              <div className="settings-field">
                <label
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  {settings.theme === "dark" ? (
                    <Moon size={16} />
                  ) : settings.theme === "crimson" ? (
                    <Droplet size={16} />
                  ) : settings.theme === "light" ? (
                    <Sun size={16} />
                  ) : (
                    <Monitor size={16} />
                  )}
                  Theme
                </label>
                <select
                  style={{
                    padding: "8px",
                    borderRadius: "6px",
                    border: "1px solid var(--border)",
                    background: "var(--bg-panel)",
                    color: "var(--text-main)",
                  }}
                  value={settings.theme}
                  onChange={(e) =>
                    setSettings({ ...settings, theme: e.target.value })
                  }
                >
                  <option value="system">System</option>
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                  <option value="crimson">Crimson</option>
                </select>
              </div>

              <div className="settings-field stacked-field">
                <label
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  <ShieldOff size={16} />
                  SSL Verification
                </label>
                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
                  }}
                >
                  <label
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                      cursor: "pointer",
                      color: "var(--text-main)",
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={settings.ssl_verification}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          ssl_verification: e.target.checked,
                        })
                      }
                    />
                    Enable SSL Certificate Verification
                  </label>
                  <p style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                    Disable this if you are developing against a local server
                    with self-signed certificates. Requires app restart to take
                    effect.
                  </p>
                </div>
              </div>

              <div className="settings-field">
                <label
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  <Clock size={16} />
                  Request Timeout (ms)
                </label>
                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
                  }}
                >
                  <input
                    type="number"
                    style={{
                      padding: "8px",
                      borderRadius: "6px",
                      border: "1px solid var(--border)",
                      background: "var(--bg-panel)",
                      color: "var(--text-main)",
                    }}
                    value={settings.request_timeout_ms}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        request_timeout_ms: parseInt(e.target.value) || 30000,
                      })
                    }
                  />
                  <p style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                    Requires app restart to take effect.
                  </p>
                </div>
              </div>
            </div>
          )}

          {activeTab === "mcp" && (
            <div
              style={{ display: "flex", flexDirection: "column", gap: "20px" }}
            >
              <div>
                <h4 style={{ margin: "0 0 8px 0", color: "var(--text-main)" }}>
                  Model Context Protocol (MCP) Integration
                </h4>
                <p
                  style={{
                    color: "var(--text-muted)",
                    fontSize: "14px",
                    lineHeight: "1.5",
                    margin: 0,
                  }}
                >
                  Spectra runs a built-in MCP server that enables AI agents
                  (like Claude Desktop or Cursor) to natively interact with your
                  APIs. Once connected, your agent can read your request
                  history, execute requests, and even take screenshots of the
                  GUI!
                </p>
              </div>

              <div
                style={{
                  background: "var(--bg-panel)",
                  border: "1px solid var(--border)",
                  borderRadius: "8px",
                  padding: "16px",
                }}
              >
                <h5 style={{ margin: "0 0 12px 0", color: "var(--text-main)" }}>
                  Claude Configuration
                </h5>
                <p
                  style={{
                    color: "var(--text-muted)",
                    fontSize: "13px",
                    margin: "0 0 8px 0",
                  }}
                >
                  Add the following to your Claude MCP settings:
                </p>
                <p
                  style={{
                    color: "var(--text-main)",
                    fontSize: "13px",
                    margin: "12px 0 4px 0",
                    fontWeight: "bold",
                  }}
                >
                  If running the packaged macOS App:
                </p>
                <pre
                  style={{
                    background: "#1e1e1e",
                    color: "#d4d4d4",
                    padding: "12px",
                    borderRadius: "6px",
                    fontSize: "12px",
                    overflowX: "auto",
                    margin: 0,
                  }}
                >
                  {`{
  "mcpServers": {
    "spectra": {
      "command": "/Applications/Spectra.app/Contents/MacOS/spectra-mcp",
      "args": []
    }
  }
}`}
                </pre>

                <p
                  style={{
                    color: "var(--text-main)",
                    fontSize: "13px",
                    margin: "16px 0 4px 0",
                    fontWeight: "bold",
                  }}
                >
                  If running from source:
                </p>
                <pre
                  style={{
                    background: "#1e1e1e",
                    color: "#d4d4d4",
                    padding: "12px",
                    borderRadius: "6px",
                    fontSize: "12px",
                    overflowX: "auto",
                    margin: 0,
                  }}
                >
                  {`{
  "mcpServers": {
    "spectra": {
      "command": "cargo",
      "args": ["run", "-p", "spectra-mcp"]
    }
  }
}`}
                </pre>
              </div>

              <div>
                <h5 style={{ margin: "0 0 12px 0", color: "var(--text-main)" }}>
                  Available Agent Tools
                </h5>
                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "12px",
                  }}
                >
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Workspaces</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>list_workspaces</code>,{" "}
                      <code>create_workspace</code>, <code>open_workspace</code>
                      , <code>set_workspace_auth</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Requests</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>list_requests</code>, <code>open_request</code>,{" "}
                      <code>create_request</code>, <code>delete_request</code>,{" "}
                      <code>move_request</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Request Config</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>set_method</code>, <code>set_url</code>,{" "}
                      <code>set_name</code>, <code>set_notes</code>,{" "}
                      <code>set_headers</code>, <code>set_params</code>,{" "}
                      <code>set_body</code>, <code>set_auth</code>,{" "}
                      <code>clear_auth</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Execution</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>send_request</code>,{" "}
                      <code>send_request_with_lines</code>,{" "}
                      <code>analyze_response</code>, <code>quick_test</code>,{" "}
                      <code>preview_headers</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Environments</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>list_environments</code>,{" "}
                      <code>create_environment</code>,{" "}
                      <code>update_environment</code>,{" "}
                      <code>delete_environment</code>,{" "}
                      <code>set_active_environment</code>,{" "}
                      <code>check_secrets_health</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Folders</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>list_folders</code>, <code>create_folder</code>,{" "}
                      <code>set_folder_auth</code>, <code>rename_folder</code>,{" "}
                      <code>move_folder</code>, <code>delete_folder</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>OAuth2</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>oauth_start_flow</code>,{" "}
                      <code>oauth_poll_flow</code>,{" "}
                      <code>oauth_cancel_flow</code>,{" "}
                      <code>oauth_save_token</code>,{" "}
                      <code>oauth_list_saved_tokens</code>,{" "}
                      <code>oauth_use_saved_token</code>,{" "}
                      <code>oauth_delete_saved_token</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>History & Saves</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>list_history</code>, <code>open_history_entry</code>
                      , <code>delete_history_entry</code>,{" "}
                      <code>replay_history_entry</code>,{" "}
                      <code>convert_history_to_request</code>,{" "}
                      <code>list_saved_responses</code>,{" "}
                      <code>save_response</code>,{" "}
                      <code>delete_saved_response</code>,{" "}
                      <code>open_saved_response</code>,{" "}
                      <code>rename_saved_response</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>GUI Automation</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>automation_screenshot_request</code>,{" "}
                      <code>automation_focus_request</code>,{" "}
                      <code>send_and_screenshot</code>,{" "}
                      <code>search_response</code>,{" "}
                      <code>set_ui_line_numbers</code>
                    </div>
                  </div>
                  <div
                    className="settings-field"
                    style={{ alignItems: "flex-start" }}
                  >
                    <label>Import/Export</label>
                    <div
                      style={{
                        fontSize: "13px",
                        color: "var(--text-muted)",
                        lineHeight: "1.6",
                      }}
                    >
                      <code>import_collection</code>,{" "}
                      <code>export_collection</code>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
        <div className="env-editor-footer">
          <span />
          <div style={{ display: "flex", gap: "8px" }}>
            <button onClick={onClose} disabled={saving}>
              Cancel
            </button>
            <button className="primary" onClick={handleSave} disabled={saving}>
              {saving ? "Saving..." : "Save Settings"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
