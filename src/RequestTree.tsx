import { useEffect, useState } from "react";
import { api } from "./api";
import { ContextMenu } from "./ContextMenu";
import type { ContextMenuItem } from "./ContextMenu";
import { ScopeAuthModal } from "./ScopeAuthModal";
import type {
  AuthConfig,
  Folder,
  RequestSummary,
  SavedResponse,
} from "./types";
import {
  ChevronRight,
  ChevronDown,
  Plus,
  FolderPlus,
  Trash2,
  FileText,
} from "lucide-react";

interface Props {
  workspaceId: string;
  folders: Folder[];
  requests: RequestSummary[];
  activeRequestId: string | null;
  filter: string;
  onOpenRequest: (id: string) => void;
  onCreateFolder: (parentFolderId: string | null, name: string) => void;
  onRenameFolder: (id: string, name: string) => void;
  onDeleteFolder: (id: string) => void;
  onSetFolderAuth: (id: string, auth: AuthConfig) => void;
  onMoveRequest: (requestId: string, targetFolderId: string | null) => void;
  onCreateRequest: (folderId: string | null) => void;
  onRenameRequest: (id: string, name: string) => void;
  onDeleteRequest: (id: string) => void;
  onDuplicateRequest: (id: string) => void;
  onCopyRequestAsCurl: (id: string) => void;
  onOpenSavedResponse: (saved: SavedResponse) => void;
  savedResponsesRefreshSignal: number;
  /** Bumped by the parent (e.g. sidebar "+ Folder" button) to start creating
   * a new top-level folder inline, since window.prompt() is unavailable in
   * the Tauri webview. */
  newTopLevelFolderSignal: number;
}

type TreeContextMenu =
  | { kind: "folder"; folder: Folder; x: number; y: number }
  | { kind: "request"; request: RequestSummary; x: number; y: number };

export function RequestTree({
  workspaceId,
  folders,
  requests,
  activeRequestId,
  filter,
  onOpenRequest,
  onCreateFolder,
  onRenameFolder,
  onDeleteFolder,
  onSetFolderAuth,
  onMoveRequest,
  onCreateRequest,
  onRenameRequest,
  onDeleteRequest,
  onDuplicateRequest,
  onCopyRequestAsCurl,
  onOpenSavedResponse,
  savedResponsesRefreshSignal,
  newTopLevelFolderSignal,
}: Props) {
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [expandedRequests, setExpandedRequests] = useState<Set<string>>(
    new Set(),
  );
  const [savedByRequest, setSavedByRequest] = useState<
    Record<string, SavedResponse[]>
  >({});
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renamingType, setRenamingType] = useState<"folder" | "request">(
    "folder",
  );
  const [renameValue, setRenameValue] = useState("");
  const [dragOverFolder, setDragOverFolder] = useState<string | null>(null);
  // parentFolderId of null with creatingFolder true means "creating at top level".
  const [creatingFolderUnder, setCreatingFolderUnder] = useState<string | null>(
    null,
  );
  const [creatingFolderAtRoot, setCreatingFolderAtRoot] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [contextMenu, setContextMenu] = useState<TreeContextMenu | null>(null);
  const [editingFolderAuth, setEditingFolderAuth] = useState<Folder | null>(
    null,
  );

  useEffect(() => {
    if (newTopLevelFolderSignal > 0) {
      setCreatingFolderAtRoot(true);
      setCreatingFolderUnder(null);
      setNewFolderName("");
    }
  }, [newTopLevelFolderSignal]);

  function startCreateFolder(parentId: string | null) {
    setCreatingFolderUnder(parentId);
    setCreatingFolderAtRoot(parentId === null);
    setNewFolderName("");
  }

  function commitCreateFolder(parentId: string | null) {
    const name = newFolderName.trim();
    if (name) onCreateFolder(parentId, name);
    setCreatingFolderUnder(null);
    setCreatingFolderAtRoot(false);
  }

  function cancelCreateFolder() {
    setCreatingFolderUnder(null);
    setCreatingFolderAtRoot(false);
  }

  const query = filter.trim().toLowerCase();
  const matchesFilter = (name: string) =>
    query === "" || name.toLowerCase().includes(query);

  // When filtering, a folder should show if it or any descendant matches.
  function folderMatchesRecursively(folderId: string): boolean {
    const folder = folders.find((f) => f.id === folderId);
    if (folder && matchesFilter(folder.name)) return true;
    if (requests.some((r) => r.folder_id === folderId && matchesFilter(r.name)))
      return true;
    return folders.some(
      (f) => f.parent_folder_id === folderId && folderMatchesRecursively(f.id),
    );
  }

  useEffect(() => {
    for (const id of expandedRequests) loadSaved(id);
  }, [savedResponsesRefreshSignal]);

  async function loadSaved(requestId: string) {
    const saved = await api.listSavedResponses(workspaceId, requestId);
    setSavedByRequest((prev) => ({ ...prev, [requestId]: saved }));
  }

  function toggleFolder(id: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function toggleRequest(id: string) {
    setExpandedRequests((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
        loadSaved(id);
      }
      return next;
    });
  }

  function startRenameFolder(folder: Folder) {
    setRenamingId(folder.id);
    setRenamingType("folder");
    setRenameValue(folder.name);
  }

  function startRenameRequest(r: RequestSummary) {
    setRenamingId(r.id);
    setRenamingType("request");
    setRenameValue(r.name);
  }

  function commitRename() {
    const value = renameValue.trim();
    if (renamingId && value) {
      if (renamingType === "folder") onRenameFolder(renamingId, value);
      else onRenameRequest(renamingId, value);
    }
    setRenamingId(null);
  }

  function childFolders(parentId: string | null) {
    const kids = folders.filter((f) => f.parent_folder_id === parentId);
    return query === ""
      ? kids
      : kids.filter((f) => folderMatchesRecursively(f.id));
  }

  function childRequests(parentId: string | null) {
    const kids = requests.filter((r) => r.folder_id === parentId);
    return query === "" ? kids : kids.filter((r) => matchesFilter(r.name));
  }

  function renderFolder(folder: Folder, depth: number) {
    const isCollapsed = query === "" && collapsed.has(folder.id);
    return (
      <li key={folder.id} className="tree-node">
        <div
          className={`tree-row folder-row${dragOverFolder === folder.id ? " drag-over" : ""}`}
          style={{ paddingLeft: `${depth * 14 + 6}px` }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOverFolder(folder.id);
          }}
          onDragLeave={() =>
            setDragOverFolder((f) => (f === folder.id ? null : f))
          }
          onDrop={(e) => {
            e.preventDefault();
            const requestId = e.dataTransfer.getData("text/request-id");
            if (requestId) onMoveRequest(requestId, folder.id);
            setDragOverFolder(null);
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({
              kind: "folder",
              folder,
              x: e.clientX,
              y: e.clientY,
            });
          }}
        >
          <button
            className="tree-toggle"
            onClick={() => toggleFolder(folder.id)}
          >
            {isCollapsed ? (
              <ChevronRight size={14} />
            ) : (
              <ChevronDown size={14} />
            )}
          </button>
          {renamingId === folder.id ? (
            <input
              autoFocus
              className="tree-rename-input"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={commitRename}
              onKeyDown={(e) => e.key === "Enter" && commitRename()}
            />
          ) : (
            <span
              className="tree-label"
              onDoubleClick={() => startRenameFolder(folder)}
            >
              {folder.name}
            </span>
          )}
          <span className="tree-row-actions">
            <button
              title="New request here"
              onClick={() => onCreateRequest(folder.id)}
            >
              <Plus size={14} />
            </button>
            <button
              title="New subfolder"
              onClick={() => startCreateFolder(folder.id)}
            >
              <FolderPlus size={14} />
            </button>
            <button
              title="Delete folder"
              className="danger-btn"
              onClick={() => onDeleteFolder(folder.id)}
            >
              <Trash2 size={14} />
            </button>
          </span>
        </div>
        {!isCollapsed && (
          <ul className="tree-children">
            {childFolders(folder.id).map((f) => renderFolder(f, depth + 1))}
            {childRequests(folder.id).map((r) => renderRequest(r, depth + 1))}
            {creatingFolderUnder === folder.id &&
              renderNewFolderInput(folder.id, depth + 1)}
          </ul>
        )}
      </li>
    );
  }

  function renderNewFolderInput(parentId: string | null, depth: number) {
    return (
      <li
        key="__new_folder__"
        className="tree-row new-folder-row"
        style={{ paddingLeft: `${depth * 14 + 6}px` }}
      >
        <span className="tree-toggle" />
        <input
          autoFocus
          className="tree-rename-input"
          placeholder="Folder name"
          value={newFolderName}
          onChange={(e) => setNewFolderName(e.target.value)}
          onBlur={() => commitCreateFolder(parentId)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitCreateFolder(parentId);
            if (e.key === "Escape") cancelCreateFolder();
          }}
        />
      </li>
    );
  }

  function renderRequest(r: RequestSummary, depth: number) {
    const isExpanded = expandedRequests.has(r.id);
    const saved = savedByRequest[r.id] ?? [];
    return (
      <li key={r.id} className="tree-node">
        <div
          className={`tree-row request-row${activeRequestId === r.id ? " active" : ""}`}
          style={{ paddingLeft: `${depth * 14 + 6}px` }}
          draggable
          onDragStart={(e) => e.dataTransfer.setData("text/request-id", r.id)}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({
              kind: "request",
              request: r,
              x: e.clientX,
              y: e.clientY,
            });
          }}
        >
          <button className="tree-toggle" onClick={() => toggleRequest(r.id)}>
            {isExpanded ? (
              <ChevronDown size={14} />
            ) : (
              <ChevronRight size={14} />
            )}
          </button>
          <span className={`method-badge method-${r.method}`}>{r.method}</span>
          {renamingId === r.id ? (
            <input
              autoFocus
              className="tree-rename-input"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={commitRename}
              onKeyDown={(e) => e.key === "Enter" && commitRename()}
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span
              className="request-name"
              onClick={() => onOpenRequest(r.id)}
              onDoubleClick={() => startRenameRequest(r)}
            >
              {r.name}
            </span>
          )}
        </div>
        {isExpanded && (
          <ul className="tree-children">
            {saved.length === 0 && (
              <li
                className="tree-row saved-response-empty"
                style={{ paddingLeft: `${(depth + 1) * 14 + 6}px` }}
              >
                No saved responses
              </li>
            )}
            {saved.map((s) => (
              <li
                key={s.id}
                className="tree-row saved-response-row"
                style={{ paddingLeft: `${(depth + 1) * 14 + 6}px` }}
                onClick={() => onOpenSavedResponse(s)}
              >
                <span className="saved-response-icon">
                  <FileText size={14} />
                </span>
                <span className="request-name">{s.name}</span>
              </li>
            ))}
          </ul>
        )}
      </li>
    );
  }

  function folderMenuItems(folder: Folder): ContextMenuItem[] {
    return [
      { label: "New Request", onClick: () => onCreateRequest(folder.id) },
      { label: "New Subfolder", onClick: () => startCreateFolder(folder.id) },
      {
        label: "Rename",
        onClick: () => startRenameFolder(folder),
        separatorBefore: true,
      },
      { label: "Edit Auth", onClick: () => setEditingFolderAuth(folder) },
      {
        label: "Delete",
        onClick: () => onDeleteFolder(folder.id),
        danger: true,
        separatorBefore: true,
      },
    ];
  }

  function requestMenuItems(r: RequestSummary): ContextMenuItem[] {
    return [
      { label: "Rename", onClick: () => startRenameRequest(r) },
      {
        label: "Copy as cURL",
        onClick: () => onCopyRequestAsCurl(r.id),
        shortcut: "⌘C",
      },
      {
        label: "Duplicate",
        onClick: () => onDuplicateRequest(r.id),
        shortcut: "⌘D",
      },
      {
        label: "Delete",
        onClick: () => onDeleteRequest(r.id),
        danger: true,
        separatorBefore: true,
      },
    ];
  }

  return (
    <>
      <ul
        className="tree-children root-tree"
        onDragOver={(e) => {
          e.preventDefault();
          setDragOverFolder("__root__");
        }}
        onDragLeave={() =>
          setDragOverFolder((f) => (f === "__root__" ? null : f))
        }
        onDrop={(e) => {
          e.preventDefault();
          const requestId = e.dataTransfer.getData("text/request-id");
          if (requestId) onMoveRequest(requestId, null);
          setDragOverFolder(null);
        }}
      >
        {childFolders(null).map((f) => renderFolder(f, 0))}
        {childRequests(null).map((r) => renderRequest(r, 0))}
        {creatingFolderAtRoot && renderNewFolderInput(null, 0)}
        {dragOverFolder === "__root__" && (
          <li className="drop-hint">Drop here to move to top level</li>
        )}
      </ul>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={
            contextMenu.kind === "folder"
              ? folderMenuItems(contextMenu.folder)
              : requestMenuItems(contextMenu.request)
          }
          onClose={() => setContextMenu(null)}
        />
      )}

      {editingFolderAuth && (
        <ScopeAuthModal
          scope="folder"
          scopeName={editingFolderAuth.name}
          initialAuth={editingFolderAuth.auth}
          onSave={async (auth) => onSetFolderAuth(editingFolderAuth.id, auth)}
          onClose={() => setEditingFolderAuth(null)}
        />
      )}
    </>
  );
}
