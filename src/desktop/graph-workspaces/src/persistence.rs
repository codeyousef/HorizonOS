//! Workspace persistence to disk

use crate::{Workspace, WorkspaceError};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Workspace persistence handler
pub struct WorkspacePersistence {
    /// Base directory for workspace storage
    base_dir: PathBuf,
}

impl WorkspacePersistence {
    /// Create new persistence handler
    pub fn new() -> Self {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("horizonos")
            .join("workspaces");
        
        Self { base_dir }
    }
    
    /// Set custom base directory
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
    
    /// Ensure the workspace directory exists
    async fn ensure_dir(&self) -> Result<(), WorkspaceError> {
        if !self.base_dir.exists() {
            fs::create_dir_all(&self.base_dir).await?;
        }
        Ok(())
    }
    
    /// Get the path for a workspace file
    fn workspace_path(&self, workspace_id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", workspace_id))
    }
    
    /// Save a single workspace
    pub async fn save_workspace(&self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        self.ensure_dir().await?;
        
        let path = self.workspace_path(&workspace.id);
        let json = serde_json::to_string_pretty(workspace)?;
        
        let mut file = fs::File::create(&path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;
        
        Ok(())
    }
    
    /// Load a single workspace
    pub async fn load_workspace(&self, workspace_id: &str) -> Result<Workspace, WorkspaceError> {
        let path = self.workspace_path(workspace_id);
        
        let mut file = fs::File::open(&path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let workspace: Workspace = serde_json::from_str(&contents)?;
        Ok(workspace)
    }
    
    /// Save all workspaces
    pub async fn save_workspaces(&self, workspaces: &[Workspace]) -> Result<(), WorkspaceError> {
        self.ensure_dir().await?;
        
        // Save index file
        let index_path = self.base_dir.join("index.json");
        let workspace_ids: Vec<String> = workspaces.iter()
            .map(|w| w.id.clone())
            .collect();
        
        let index_json = serde_json::to_string_pretty(&workspace_ids)?;
        let mut index_file = fs::File::create(&index_path).await?;
        index_file.write_all(index_json.as_bytes()).await?;
        index_file.sync_all().await?;
        
        // Save individual workspaces
        for workspace in workspaces {
            self.save_workspace(workspace).await?;
        }
        
        Ok(())
    }
    
    /// Load all workspaces
    pub async fn load_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceError> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        
        let index_path = self.base_dir.join("index.json");
        if !index_path.exists() {
            // No index file, scan directory
            return self.scan_workspaces().await;
        }
        
        // Load from index
        let mut file = fs::File::open(&index_path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let workspace_ids: Vec<String> = serde_json::from_str(&contents)?;
        let mut workspaces = Vec::new();
        
        for id in workspace_ids {
            match self.load_workspace(&id).await {
                Ok(workspace) => workspaces.push(workspace),
                Err(e) => {
                    log::warn!("Failed to load workspace {}: {}", id, e);
                }
            }
        }
        
        Ok(workspaces)
    }
    
    /// Scan directory for workspace files
    async fn scan_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceError> {
        let mut workspaces = Vec::new();
        
        let mut entries = fs::read_dir(&self.base_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") 
                && path.file_stem().and_then(|s| s.to_str()) != Some("index") 
            {
                match fs::read_to_string(&path).await {
                    Ok(contents) => {
                        match serde_json::from_str::<Workspace>(&contents) {
                            Ok(workspace) => workspaces.push(workspace),
                            Err(e) => {
                                log::warn!("Failed to parse workspace file {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read workspace file {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(workspaces)
    }
    
    /// Delete a workspace
    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), WorkspaceError> {
        let path = self.workspace_path(workspace_id);
        
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        
        Ok(())
    }
    
    /// Export workspace to a specific path
    pub async fn export_workspace(
        &self,
        workspace: &Workspace,
        path: &Path,
    ) -> Result<(), WorkspaceError> {
        let json = serde_json::to_string_pretty(workspace)?;
        
        let mut file = fs::File::create(path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;
        
        Ok(())
    }
    
    /// Import workspace from a specific path
    pub async fn import_workspace(&self, path: &Path) -> Result<Workspace, WorkspaceError> {
        let mut file = fs::File::open(path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let workspace: Workspace = serde_json::from_str(&contents)?;
        Ok(workspace)
    }
}

// Re-export dirs for finding config directories
pub use dirs;