//! Export support — the counterpart to `import`. Takes a workspace's real
//! Folders/Requests (as already persisted) and serializes them into an
//! external format. Each format module owns its own serialization; this
//! file only owns the shared tree-walking helper and the format enum.

pub mod curl;
pub mod openapi;
pub mod postman;

use crate::model::{Folder, Request};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Curl,
    PostmanCollection,
    OpenApi,
}

/// One folder's worth of requests plus its nested sub-folders, rooted at
/// `None` (top-level). Built once per export from the flat storage lists so
/// each format module can recurse without re-deriving parent/child links.
pub struct FolderNode<'a> {
    pub folder: Option<&'a Folder>,
    pub requests: Vec<&'a Request>,
    pub children: Vec<FolderNode<'a>>,
}

pub fn build_tree<'a>(folders: &'a [Folder], requests: &'a [Request], parent_id: Option<&str>) -> FolderNode<'a> {
    let folder = parent_id.and_then(|id| folders.iter().find(|f| f.id == id));
    let own_requests: Vec<&Request> = requests.iter().filter(|r| r.folder_id.as_deref() == parent_id).collect();
    let children: Vec<FolderNode<'a>> = folders
        .iter()
        .filter(|f| f.parent_folder_id.as_deref() == parent_id)
        .map(|f| build_tree(folders, requests, Some(f.id.as_str())))
        .collect();
    FolderNode { folder, requests: own_requests, children }
}
