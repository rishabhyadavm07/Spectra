import { useEffect, useState } from "react";
import { api } from "./api";
import type {
  Environment,
  OrphanedSecret,
  VariableInput,
  Workspace,
} from "./types";
import { UNCHANGED_SECRET_SENTINEL } from "./types";
import { Plus, AlertTriangle } from "lucide-react";

interface Props {
  workspace: Workspace;
  onWorkspaceChange: (ws: Workspace) => void;
  onVariablesChanged?: () => void;
}

interface VarRow {
  key: string;
  value: string;
  secret: boolean;
}

function toRows(
  vars: Record<string, { value: string; secret: boolean }>,
): VarRow[] {
  return Object.entries(vars).map(([key, v]) => ({
    key,
    value: v.value,
    secret: v.secret,
  }));
}

function toVariableInputs(rows: VarRow[]): Record<string, VariableInput> {
  const out: Record<string, VariableInput> = {};
  for (const r of rows) {
    if (r.key.trim()) out[r.key] = { value: r.value, secret: r.secret };
  }
  return out;
}

export function EnvironmentPanel({
  workspace,
  onWorkspaceChange,
  onVariablesChanged,
}: Props) {
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editRows, setEditRows] = useState<VarRow[]>([]);
  const [orphanedSecrets, setOrphanedSecrets] = useState<OrphanedSecret[]>([]);

  useEffect(() => {
    refresh();
    checkSecretsHealth();
  }, [workspace.id]);

  async function refresh() {
    setEnvironments(await api.listEnvironments(workspace.id));
  }

  async function checkSecretsHealth() {
    try {
      setOrphanedSecrets(await api.checkSecretsHealth(workspace.id));
    } catch {
      // Non-critical UI hint — a failed check just means no warning shows.
      setOrphanedSecrets([]);
    }
  }

  async function handleSelect(id: string) {
    const updated = await api.setActiveEnvironment(
      workspace.id,
      id === "" ? null : id,
    );
    onWorkspaceChange(updated);
  }

  function openNewEnvironment() {
    setEditingId(null);
    setEditName("New Environment");
    setEditRows([{ key: "", value: "", secret: false }]);
    setEditorOpen(true);
  }

  function openEditEnvironment(env: Environment) {
    setEditingId(env.id);
    setEditName(env.name);
    const rows = toRows(env.variables);
    setEditRows(
      rows.length > 0 ? rows : [{ key: "", value: "", secret: false }],
    );
    setEditorOpen(true);
  }

  async function handleSave() {
    const variables = toVariableInputs(editRows);
    if (editingId) {
      await api.updateEnvironment(workspace.id, editingId, editName, variables);
    } else {
      const created = await api.createEnvironment(
        workspace.id,
        editName,
        variables,
      );
      const updated = await api.setActiveEnvironment(workspace.id, created.id);
      onWorkspaceChange(updated);
    }
    setEditorOpen(false);
    refresh();
    checkSecretsHealth();
    onVariablesChanged?.();
  }

  async function handleDelete() {
    if (!editingId) return;
    await api.deleteEnvironment(workspace.id, editingId);
    if (workspace.active_environment_id === editingId) {
      onWorkspaceChange({ ...workspace, active_environment_id: null });
    }
    setEditorOpen(false);
    refresh();
    checkSecretsHealth();
    onVariablesChanged?.();
  }

  function updateRow(
    index: number,
    field: keyof VarRow,
    value: string | boolean,
  ) {
    const rows = [...editRows];
    rows[index] = { ...rows[index], [field]: value };
    setEditRows(rows);
  }

  function toggleSecret(index: number) {
    const row = editRows[index];
    const nextSecret = !row.secret;
    // Flipping a currently-masked secret row back to plain would otherwise
    // leak the sentinel string as if it were a real value — clear it so the
    // user must type a real value instead of silently storing "••••••••".
    const nextValue =
      row.secret && row.value === UNCHANGED_SECRET_SENTINEL ? "" : row.value;
    updateRow(index, "value", nextValue);
    updateRow(index, "secret", nextSecret);
  }

  const activeEnv = environments.find(
    (e) => e.id === workspace.active_environment_id,
  );

  return (
    <div className="env-panel">
      <div className="env-picker-row">
        <select
          value={workspace.active_environment_id ?? ""}
          onChange={(e) => handleSelect(e.target.value)}
        >
          <option value="">No Environment</option>
          {environments.map((e) => (
            <option key={e.id} value={e.id}>
              {e.name}
            </option>
          ))}
        </select>
        {activeEnv ? (
          <button
            title="Edit environment"
            onClick={() => openEditEnvironment(activeEnv)}
          >
            Edit
          </button>
        ) : (
          <button title="New environment" onClick={openNewEnvironment}>
            <Plus size={14} style={{ marginRight: 4 }} /> Env
          </button>
        )}
      </div>

      {orphanedSecrets.length > 0 && (
        <div className="secrets-warning">
          <strong>
            <AlertTriangle
              size={14}
              style={{ marginRight: 4, verticalAlign: "middle" }}
            />{" "}
            {orphanedSecrets.length} secret
            {orphanedSecrets.length === 1 ? "" : "s"} missing from Keychain.
          </strong>
          <p>
            This usually means this workspace was copied from another machine or
            user account — Keychain entries never travel with{" "}
            <code>~/.spectra</code>. Affected variables will resolve to an empty
            value until re-entered:
          </p>
          <ul>
            {orphanedSecrets.map((s) => (
              <li key={`${s.environment_id}:${s.variable_name}`}>
                <strong>{s.variable_name}</strong> in {s.environment_name}
              </li>
            ))}
          </ul>
        </div>
      )}

      {editorOpen && (
        <div
          className="env-editor-backdrop"
          onClick={() => setEditorOpen(false)}
        >
          <div className="env-editor" onClick={(e) => e.stopPropagation()}>
            <div className="env-editor-header">
              <input
                className="env-name-input"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                placeholder="Environment name"
              />
            </div>

            <div className="env-editor-body">
              <div className="env-var-header">
                <span>Variable</span>
                <span>Value</span>
                <span className="env-var-header-secret">Secret</span>
              </div>
              {editRows.map((row, i) => (
                <div className="kv-row env-var-row" key={i}>
                  <input
                    placeholder="key"
                    value={row.key}
                    onChange={(e) => updateRow(i, "key", e.target.value)}
                  />
                  <input
                    placeholder="value"
                    type={row.secret ? "password" : "text"}
                    value={row.value}
                    onFocus={(e) => {
                      if (
                        row.secret &&
                        row.value === UNCHANGED_SECRET_SENTINEL
                      ) {
                        updateRow(i, "value", "");
                        e.target.value = "";
                      }
                    }}
                    onChange={(e) => updateRow(i, "value", e.target.value)}
                  />
                  <label
                    className="env-var-secret-toggle"
                    title="Store this value in Windows Credential Manager"
                  >
                    <input
                      type="checkbox"
                      checked={row.secret}
                      onChange={() => toggleSecret(i)}
                    />
                  </label>
                </div>
              ))}
              <button
                onClick={() =>
                  setEditRows([
                    ...editRows,
                    { key: "", value: "", secret: false },
                  ])
                }
              >
                <Plus size={14} style={{ marginRight: 4 }} /> Variable
              </button>
              <p className="hint-text">
                Secret variables are stored in the Windows Credential Manager, never written
                to disk as plaintext.
              </p>
            </div>

            <div className="env-editor-footer">
              {editingId && (
                <button className="danger-btn" onClick={handleDelete}>
                  Delete
                </button>
              )}
              <div className="env-editor-footer-right">
                <button onClick={() => setEditorOpen(false)}>Cancel</button>
                <button onClick={handleSave} disabled={!editName.trim()}>
                  Save
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
