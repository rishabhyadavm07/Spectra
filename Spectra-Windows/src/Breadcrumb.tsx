import type { Folder } from "./types";

interface Props {
  workspaceName: string;
  folders: Folder[];
  currentFolderId: string | null;
  requestName: string;
  savedResponseName?: string;
}

function folderChain(folders: Folder[], folderId: string | null): Folder[] {
  const chain: Folder[] = [];
  let current = folders.find((f) => f.id === folderId) ?? null;
  while (current) {
    chain.unshift(current);
    current = folders.find((f) => f.id === current!.parent_folder_id) ?? null;
  }
  return chain;
}

export function Breadcrumb({
  workspaceName,
  folders,
  currentFolderId,
  requestName,
  savedResponseName,
}: Props) {
  const chain = folderChain(folders, currentFolderId);

  return (
    <div className="breadcrumb">
      <span className="breadcrumb-item">{workspaceName}</span>
      {chain.map((f) => (
        <span key={f.id} className="breadcrumb-item">
          <span className="breadcrumb-sep">›</span>
          {f.name}
        </span>
      ))}
      <span className="breadcrumb-item breadcrumb-current">
        <span className="breadcrumb-sep">›</span>
        {requestName}
      </span>
      {savedResponseName && (
        <span className="breadcrumb-item breadcrumb-current">
          <span className="breadcrumb-sep">›</span>
          {savedResponseName}
        </span>
      )}
    </div>
  );
}
