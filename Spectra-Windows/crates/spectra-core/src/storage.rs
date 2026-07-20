use crate::error::{ApiError, ApiResult};
use crate::model::{Environment, Folder, HistoryEntry, Request, SavedResponse, Workspace};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, Row};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone)]
pub struct Storage {
    pub pool: SqlitePool,
    pub responses_dir: PathBuf,
}

impl Storage {
    pub async fn new(root: PathBuf) -> ApiResult<Self> {
        let db_path = root.join("spectra.db");
        let responses_dir = root.join("responses");

        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(&responses_dir)?;

        let db_url = format!("sqlite://{}", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let storage = Self {
            pool,
            responses_dir,
        };
        storage.migrate().await?;
        Ok(storage)
    }

    async fn migrate(&self) -> ApiResult<()> {
        let sql = r#"
        CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            active_environment_id TEXT,
            auth_json TEXT NOT NULL,
            created_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS folders (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            parent_folder_id TEXT,
            name TEXT NOT NULL,
            auth_json TEXT NOT NULL,
            created_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS requests (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            folder_id TEXT,
            name TEXT NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            headers_json TEXT NOT NULL,
            params_json TEXT NOT NULL,
            body_json TEXT NOT NULL,
            auth_json TEXT NOT NULL,
            notes TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS environments (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            name TEXT NOT NULL,
            variables_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            request_id TEXT NOT NULL,
            request_snapshot_json TEXT NOT NULL,
            response_json TEXT,
            error TEXT,
            executed_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS saved_responses (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            request_id TEXT NOT NULL,
            name TEXT NOT NULL,
            response_json TEXT NOT NULL,
            saved_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS workspace_saved_auths (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            name TEXT NOT NULL,
            auth_json TEXT NOT NULL,
            created_at DATETIME NOT NULL
        );

        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            theme TEXT NOT NULL,
            ssl_verification BOOLEAN NOT NULL,
            request_timeout_ms INTEGER NOT NULL
        );
        "#;
        
        sqlx::query(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        Ok(())
    }

    // --- Workspaces ---

    pub async fn list_workspaces(&self) -> ApiResult<Vec<Workspace>> {
        let rows = sqlx::query("SELECT id, name, active_environment_id, auth_json, created_at FROM workspaces ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(Workspace {
                id: row.get("id"),
                name: row.get("name"),
                active_environment_id: row.get("active_environment_id"),
                auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
                created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
            });
        }
        Ok(workspaces)
    }

    pub async fn get_workspace(&self, id: &str) -> ApiResult<Workspace> {
        let row = sqlx::query("SELECT id, name, active_environment_id, auth_json, created_at FROM workspaces WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "workspace", id: id.to_string() })?;

        Ok(Workspace {
            id: row.get("id"),
            name: row.get("name"),
            active_environment_id: row.get("active_environment_id"),
            auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
            created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
        })
    }

    pub async fn save_workspace(&self, ws: &Workspace) -> ApiResult<()> {
        let auth_json = serde_json::to_string(&ws.auth).unwrap_or_default();
        let created_at = ws.created_at.to_rfc3339();

        sqlx::query("INSERT INTO workspaces (id, name, active_environment_id, auth_json, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, active_environment_id=excluded.active_environment_id, auth_json=excluded.auth_json")
            .bind(&ws.id)
            .bind(&ws.name)
            .bind(&ws.active_environment_id)
            .bind(&auth_json)
            .bind(&created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    // --- Folders ---

    pub async fn list_folders(&self, workspace_id: &str) -> ApiResult<Vec<Folder>> {
        let rows = sqlx::query("SELECT id, parent_folder_id, name, auth_json, created_at FROM folders WHERE workspace_id = ? ORDER BY created_at ASC")
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut folders = Vec::new();
        for row in rows {
            folders.push(Folder {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                parent_folder_id: row.get("parent_folder_id"),
                name: row.get("name"),
                auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
                created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
            });
        }
        Ok(folders)
    }

    pub async fn get_folder(&self, workspace_id: &str, id: &str) -> ApiResult<Folder> {
        let row = sqlx::query("SELECT parent_folder_id, name, auth_json, created_at FROM folders WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "folder", id: id.to_string() })?;

        Ok(Folder {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            parent_folder_id: row.get("parent_folder_id"),
            name: row.get("name"),
            auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
            created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
        })
    }

    pub async fn save_folder(&self, folder: &Folder) -> ApiResult<()> {
        let auth_json = serde_json::to_string(&folder.auth).unwrap_or_default();
        let created_at = folder.created_at.to_rfc3339();

        sqlx::query("INSERT INTO folders (id, workspace_id, parent_folder_id, name, auth_json, created_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, parent_folder_id=excluded.parent_folder_id, auth_json=excluded.auth_json")
            .bind(&folder.id)
            .bind(&folder.workspace_id)
            .bind(&folder.parent_folder_id)
            .bind(&folder.name)
            .bind(&auth_json)
            .bind(&created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn delete_folder(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM folders WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- Requests ---

    pub async fn list_requests(&self, workspace_id: &str) -> ApiResult<Vec<Request>> {
        let rows = sqlx::query("SELECT id, folder_id, name, method, url, headers_json, params_json, body_json, auth_json, notes, created_at, updated_at FROM requests WHERE workspace_id = ? ORDER BY created_at ASC")
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut requests = Vec::new();
        for row in rows {
            requests.push(Request {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                folder_id: row.get("folder_id"),
                name: row.get("name"),
                method: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("method"))).unwrap_or(crate::model::HttpMethod::Get),
                url: row.get("url"),
                headers: serde_json::from_str(&row.get::<String, _>("headers_json")).unwrap_or_default(),
                params: serde_json::from_str(&row.get::<String, _>("params_json")).unwrap_or_default(),
                body: serde_json::from_str(&row.get::<String, _>("body_json")).unwrap_or_default(),
                auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
                notes: row.get("notes"),
                created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
                updated_at: chrono::DateTime::from_str(&row.get::<String, _>("updated_at")).unwrap_or_default(),
            });
        }
        Ok(requests)
    }

    pub async fn get_request(&self, workspace_id: &str, id: &str) -> ApiResult<Request> {
        let row = sqlx::query("SELECT folder_id, name, method, url, headers_json, params_json, body_json, auth_json, notes, created_at, updated_at FROM requests WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "request", id: id.to_string() })?;

        Ok(Request {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            folder_id: row.get("folder_id"),
            name: row.get("name"),
            method: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("method"))).unwrap_or(crate::model::HttpMethod::Get),
            url: row.get("url"),
            headers: serde_json::from_str(&row.get::<String, _>("headers_json")).unwrap_or_default(),
            params: serde_json::from_str(&row.get::<String, _>("params_json")).unwrap_or_default(),
            body: serde_json::from_str(&row.get::<String, _>("body_json")).unwrap_or_default(),
            auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
            notes: row.get("notes"),
            created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
            updated_at: chrono::DateTime::from_str(&row.get::<String, _>("updated_at")).unwrap_or_default(),
        })
    }
    
    pub async fn find_request(&self, id: &str) -> ApiResult<Request> {
        let row = sqlx::query("SELECT workspace_id, folder_id, name, method, url, headers_json, params_json, body_json, auth_json, notes, created_at, updated_at FROM requests WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "request", id: id.to_string() })?;

        Ok(Request {
            id: id.to_string(),
            workspace_id: row.get("workspace_id"),
            folder_id: row.get("folder_id"),
            name: row.get("name"),
            method: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("method"))).unwrap_or(crate::model::HttpMethod::Get),
            url: row.get("url"),
            headers: serde_json::from_str(&row.get::<String, _>("headers_json")).unwrap_or_default(),
            params: serde_json::from_str(&row.get::<String, _>("params_json")).unwrap_or_default(),
            body: serde_json::from_str(&row.get::<String, _>("body_json")).unwrap_or_default(),
            auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap_or_default(),
            notes: row.get("notes"),
            created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
            updated_at: chrono::DateTime::from_str(&row.get::<String, _>("updated_at")).unwrap_or_default(),
        })
    }

    pub async fn save_request(&self, req: &Request) -> ApiResult<()> {
        let method = serde_json::to_string(&req.method).unwrap_or_default().replace("\"", "");
        let headers_json = serde_json::to_string(&req.headers).unwrap_or_default();
        let params_json = serde_json::to_string(&req.params).unwrap_or_default();
        let body_json = serde_json::to_string(&req.body).unwrap_or_default();
        let auth_json = serde_json::to_string(&req.auth).unwrap_or_default();
        let created_at = req.created_at.to_rfc3339();
        let updated_at = req.updated_at.to_rfc3339();

        sqlx::query("INSERT INTO requests (id, workspace_id, folder_id, name, method, url, headers_json, params_json, body_json, auth_json, notes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET folder_id=excluded.folder_id, name=excluded.name, method=excluded.method, url=excluded.url, headers_json=excluded.headers_json, params_json=excluded.params_json, body_json=excluded.body_json, auth_json=excluded.auth_json, notes=excluded.notes, updated_at=excluded.updated_at")
            .bind(&req.id)
            .bind(&req.workspace_id)
            .bind(&req.folder_id)
            .bind(&req.name)
            .bind(&method)
            .bind(&req.url)
            .bind(&headers_json)
            .bind(&params_json)
            .bind(&body_json)
            .bind(&auth_json)
            .bind(&req.notes)
            .bind(&created_at)
            .bind(&updated_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn delete_request(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM requests WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    pub async fn find_and_delete_request(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM requests WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- Environments ---

    pub async fn list_environments(&self, workspace_id: &str) -> ApiResult<Vec<Environment>> {
        let rows = sqlx::query("SELECT id, name, variables_json FROM environments WHERE workspace_id = ?")
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut envs = Vec::new();
        for row in rows {
            envs.push(Environment {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                name: row.get("name"),
                variables: serde_json::from_str(&row.get::<String, _>("variables_json")).unwrap_or_default(),
            });
        }
        Ok(envs)
    }

    pub async fn get_environment(&self, workspace_id: &str, id: &str) -> ApiResult<Environment> {
        let row = sqlx::query("SELECT name, variables_json FROM environments WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "environment", id: id.to_string() })?;

        Ok(Environment {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            name: row.get("name"),
            variables: serde_json::from_str(&row.get::<String, _>("variables_json")).unwrap_or_default(),
        })
    }

    pub async fn save_environment(&self, env: &Environment) -> ApiResult<()> {
        let variables_json = serde_json::to_string(&env.variables).unwrap_or_default();

        sqlx::query("INSERT INTO environments (id, workspace_id, name, variables_json) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, variables_json=excluded.variables_json")
            .bind(&env.id)
            .bind(&env.workspace_id)
            .bind(&env.name)
            .bind(&variables_json)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn delete_environment(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM environments WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- History ---

    pub async fn list_history(&self, workspace_id: &str) -> ApiResult<Vec<HistoryEntry>> {
        let rows = sqlx::query("SELECT id, request_id, request_snapshot_json, response_json, error, executed_at FROM history WHERE workspace_id = ? ORDER BY executed_at DESC")
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut history = Vec::new();
        for row in rows {
            history.push(HistoryEntry {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                request_id: row.get("request_id"),
                request_snapshot: serde_json::from_str(&row.get::<String, _>("request_snapshot_json")).unwrap(),
                response: row.get::<Option<String>, _>("response_json").and_then(|s| serde_json::from_str(&s).ok()),
                error: row.get("error"),
                executed_at: chrono::DateTime::from_str(&row.get::<String, _>("executed_at")).unwrap_or_default(),
            });
        }
        Ok(history)
    }

    pub async fn list_history_for_request(&self, workspace_id: &str, request_id: &str) -> ApiResult<Vec<HistoryEntry>> {
        let rows = sqlx::query("SELECT id, request_snapshot_json, response_json, error, executed_at FROM history WHERE workspace_id = ? AND request_id = ? ORDER BY executed_at DESC LIMIT 5")
            .bind(workspace_id)
            .bind(request_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut history = Vec::new();
        for row in rows {
            history.push(HistoryEntry {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                request_id: request_id.to_string(),
                request_snapshot: serde_json::from_str(&row.get::<String, _>("request_snapshot_json")).unwrap(),
                response: row.get::<Option<String>, _>("response_json").and_then(|s| serde_json::from_str(&s).ok()),
                error: row.get("error"),
                executed_at: chrono::DateTime::from_str(&row.get::<String, _>("executed_at")).unwrap_or_default(),
            });
        }
        Ok(history)
    }

    pub async fn get_history_entry(&self, workspace_id: &str, id: &str) -> ApiResult<HistoryEntry> {
        let row = sqlx::query("SELECT request_id, request_snapshot_json, response_json, error, executed_at FROM history WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "history_entry", id: id.to_string() })?;

        Ok(HistoryEntry {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            request_id: row.get("request_id"),
            request_snapshot: serde_json::from_str(&row.get::<String, _>("request_snapshot_json")).unwrap(),
            response: row.get::<Option<String>, _>("response_json").and_then(|s| serde_json::from_str(&s).ok()),
            error: row.get("error"),
            executed_at: chrono::DateTime::from_str(&row.get::<String, _>("executed_at")).unwrap_or_default(),
        })
    }

    pub async fn save_history_entry(&self, entry: &HistoryEntry) -> ApiResult<()> {
        let snapshot_json = serde_json::to_string(&entry.request_snapshot).unwrap_or_default();
        let response_json = entry.response.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default());
        let executed_at = entry.executed_at.to_rfc3339();

        sqlx::query("INSERT INTO history (id, workspace_id, request_id, request_snapshot_json, response_json, error, executed_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET response_json=excluded.response_json, error=excluded.error")
            .bind(&entry.id)
            .bind(&entry.workspace_id)
            .bind(&entry.request_id)
            .bind(&snapshot_json)
            .bind(&response_json)
            .bind(&entry.error)
            .bind(&executed_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        sqlx::query("DELETE FROM history WHERE workspace_id = ? AND request_id = ? AND id NOT IN (SELECT id FROM history WHERE workspace_id = ? AND request_id = ? ORDER BY executed_at DESC LIMIT 5)")
            .bind(&entry.workspace_id)
            .bind(&entry.request_id)
            .bind(&entry.workspace_id)
            .bind(&entry.request_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn delete_history_entry(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM history WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- Saved Responses ---

    pub async fn list_saved_responses(&self, workspace_id: &str, request_id: &str) -> ApiResult<Vec<SavedResponse>> {
        let rows = sqlx::query("SELECT id, name, response_json, saved_at FROM saved_responses WHERE workspace_id = ? AND request_id = ? ORDER BY saved_at ASC")
            .bind(workspace_id)
            .bind(request_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut responses = Vec::new();
        for row in rows {
            responses.push(SavedResponse {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                request_id: request_id.to_string(),
                name: row.get("name"),
                response: serde_json::from_str(&row.get::<String, _>("response_json")).unwrap(),
                saved_at: chrono::DateTime::from_str(&row.get::<String, _>("saved_at")).unwrap_or_default(),
            });
        }
        Ok(responses)
    }

    pub async fn save_saved_response(&self, saved: &SavedResponse) -> ApiResult<()> {
        let response_json = serde_json::to_string(&saved.response).unwrap_or_default();
        let saved_at = saved.saved_at.to_rfc3339();

        sqlx::query("INSERT INTO saved_responses (id, workspace_id, request_id, name, response_json, saved_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, response_json=excluded.response_json")
            .bind(&saved.id)
            .bind(&saved.workspace_id)
            .bind(&saved.request_id)
            .bind(&saved.name)
            .bind(&response_json)
            .bind(&saved_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn get_saved_response(&self, workspace_id: &str, id: &str) -> ApiResult<SavedResponse> {
        let row = sqlx::query("SELECT request_id, name, response_json, saved_at FROM saved_responses WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "saved_response", id: id.to_string() })?;

        Ok(SavedResponse {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            request_id: row.get("request_id"),
            name: row.get("name"),
            response: serde_json::from_str(&row.get::<String, _>("response_json")).unwrap(),
            saved_at: chrono::DateTime::from_str(&row.get::<String, _>("saved_at")).unwrap_or_default(),
        })
    }

    pub async fn delete_saved_response(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM saved_responses WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- Workspace Saved Auths ---

    pub async fn list_saved_auths(&self, workspace_id: &str) -> ApiResult<Vec<crate::model::WorkspaceSavedAuth>> {
        let rows = sqlx::query("SELECT id, name, auth_json, created_at FROM workspace_saved_auths WHERE workspace_id = ? ORDER BY created_at ASC")
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        let mut auths = Vec::new();
        for row in rows {
            auths.push(crate::model::WorkspaceSavedAuth {
                id: row.get("id"),
                workspace_id: workspace_id.to_string(),
                name: row.get("name"),
                auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap(),
                created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
            });
        }
        Ok(auths)
    }

    pub async fn get_saved_auth(&self, workspace_id: &str, id: &str) -> ApiResult<crate::model::WorkspaceSavedAuth> {
        let row = sqlx::query("SELECT name, auth_json, created_at FROM workspace_saved_auths WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound { entity: "workspace_saved_auth", id: id.to_string() })?;

        Ok(crate::model::WorkspaceSavedAuth {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            name: row.get("name"),
            auth: serde_json::from_str(&row.get::<String, _>("auth_json")).unwrap(),
            created_at: chrono::DateTime::from_str(&row.get::<String, _>("created_at")).unwrap_or_default(),
        })
    }

    pub async fn save_saved_auth(&self, auth: &crate::model::WorkspaceSavedAuth) -> ApiResult<()> {
        let auth_json = serde_json::to_string(&auth.auth).unwrap_or_default();
        let created_at = auth.created_at.to_rfc3339();

        sqlx::query("INSERT INTO workspace_saved_auths (id, workspace_id, name, auth_json, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, auth_json=excluded.auth_json")
            .bind(&auth.id)
            .bind(&auth.workspace_id)
            .bind(&auth.name)
            .bind(&auth_json)
            .bind(&created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        
        Ok(())
    }

    pub async fn delete_saved_auth(&self, workspace_id: &str, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM workspace_saved_auths WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }

    // --- Settings ---

    pub async fn get_settings(&self) -> ApiResult<crate::model::AppSettings> {
        let row = sqlx::query("SELECT theme, ssl_verification, request_timeout_ms FROM settings WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;

        if let Some(r) = row {
            Ok(crate::model::AppSettings {
                theme: r.get("theme"),
                ssl_verification: r.get("ssl_verification"),
                request_timeout_ms: r.get::<i64, _>("request_timeout_ms") as u64,
            })
        } else {
            Ok(crate::model::AppSettings::default())
        }
    }

    pub async fn save_settings(&self, settings: &crate::model::AppSettings) -> ApiResult<()> {
        let timeout = settings.request_timeout_ms as i64;
        sqlx::query("INSERT INTO settings (id, theme, ssl_verification, request_timeout_ms) VALUES (1, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET theme=excluded.theme, ssl_verification=excluded.ssl_verification, request_timeout_ms=excluded.request_timeout_ms")
            .bind(&settings.theme)
            .bind(settings.ssl_verification)
            .bind(timeout)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::IoError(e.to_string()))?;
        Ok(())
    }
}
