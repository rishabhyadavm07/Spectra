import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { ScopeAuthModal } from "./ScopeAuthModal";
import type { AuthConfig, Workspace } from "./types";
import { ChevronUp, ChevronDown, Check, Lock, Plus } from "lucide-react";

interface Props {
  workspaces: Workspace[];
  activeWorkspace: Workspace | null;
  onSelect: (workspace: Workspace) => void;
  onCreate: (name: string) => Promise<void>;
  onWorkspaceUpdated: (workspace: Workspace) => void;
}

/** Shows only the active workspace's name, sitting alongside the icon rail.
 * Clicking it opens a popover listing every other workspace (click to
 * switch) plus an inline "+ New Workspace" row — replaces the old always-
 * visible <select> + separate "+ Workspace" button. */
export function WorkspaceSwitcher({
  workspaces,
  activeWorkspace,
  onSelect,
  onCreate,
  onWorkspaceUpdated,
}: Props) {
  const [open, setOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [editingAuthFor, setEditingAuthFor] = useState<Workspace | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClickOutside(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
        setCreating(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  useEffect(() => {
    if (creating) inputRef.current?.focus();
  }, [creating]);

  async function handleCreate() {
    const name = newName.trim();
    if (!name) return;
    await onCreate(name);
    setNewName("");
    setCreating(false);
    setOpen(false);
  }

  async function handleSaveAuth(auth: AuthConfig) {
    if (!editingAuthFor) return;
    const updated = await api.setWorkspaceAuth(editingAuthFor.id, auth);
    onWorkspaceUpdated(updated);
  }

  return (
    <div className="workspace-switcher" ref={rootRef}>
      <button
        className="workspace-switcher-trigger"
        onClick={() => setOpen((v) => !v)}
        title="Switch workspace"
      >
        <span className="workspace-switcher-name">
          {activeWorkspace?.name ?? "No workspace"}
        </span>
        <span className="workspace-switcher-caret">
          {open ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </span>
      </button>

      {open && (
        <div className="workspace-switcher-menu">
          <div className="workspace-switcher-list">
            {workspaces.length === 0 && (
              <div className="workspace-switcher-empty">No workspaces yet</div>
            )}
            {workspaces.map((w) => (
              <div
                key={w.id}
                className={
                  w.id === activeWorkspace?.id
                    ? "workspace-switcher-item active"
                    : "workspace-switcher-item"
                }
              >
                <button
                  className="workspace-switcher-item-select"
                  onClick={() => {
                    onSelect(w);
                    setOpen(false);
                  }}
                >
                  <span>{w.name}</span>
                  {w.id === activeWorkspace?.id && (
                    <Check size={14} className="workspace-switcher-check" />
                  )}
                </button>
                <button
                  className="workspace-switcher-auth-btn"
                  title="Edit workspace auth"
                  onClick={() => {
                    setEditingAuthFor(w);
                    setOpen(false);
                  }}
                >
                  <Lock size={14} />
                </button>
              </div>
            ))}
          </div>

          <div className="workspace-switcher-divider" />

          {creating ? (
            <form
              className="workspace-switcher-create-form"
              onSubmit={(e) => {
                e.preventDefault();
                handleCreate();
              }}
            >
              <input
                ref={inputRef}
                placeholder="Workspace name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") setCreating(false);
                }}
              />
              <button type="submit" disabled={!newName.trim()}>
                Create
              </button>
            </form>
          ) : (
            <button
              className="workspace-switcher-item workspace-switcher-new"
              onClick={() => setCreating(true)}
            >
              <Plus size={14} style={{ marginRight: 4 }} /> New Workspace
            </button>
          )}
        </div>
      )}

      {editingAuthFor && (
        <ScopeAuthModal
          scope="workspace"
          scopeName={editingAuthFor.name}
          initialAuth={editingAuthFor.auth}
          onSave={handleSaveAuth}
          onClose={() => setEditingAuthFor(null)}
        />
      )}
    </div>
  );
}
