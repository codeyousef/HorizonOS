//! Collaboration framework for shared workspaces
//!
//! This module provides real-time collaboration features for HorizonOS graph desktop workspaces,
//! including shared workspace management, real-time synchronization, and multi-user support.

use crate::{Workspace, WorkspaceError};
use horizonos_graph_engine::scene::SceneId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Collaboration manager for shared workspaces
pub struct CollaborationManager {
    /// Shared workspaces
    shared_workspaces: Arc<RwLock<HashMap<String, SharedWorkspace>>>,
    /// Active collaboration sessions
    active_sessions: Arc<RwLock<HashMap<String, CollaborationSession>>>,
    /// User registry
    users: Arc<RwLock<HashMap<String, User>>>,
    /// Event broadcaster for collaboration events
    event_sender: broadcast::Sender<CollaborationEvent>,
    /// Sync engine for real-time updates
    sync_engine: SyncEngine,
}

impl CollaborationManager {
    /// Create a new collaboration manager
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            shared_workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            event_sender: event_sender.clone(),
            sync_engine: SyncEngine::new(event_sender),
        }
    }
    
    /// Initialize collaboration manager
    pub async fn initialize(&mut self) -> Result<(), WorkspaceError> {
        // Load shared workspaces from storage
        self.load_shared_workspaces().await?;
        
        // Initialize sync engine
        self.sync_engine.initialize().await?;
        
        log::info!("Collaboration manager initialized");
        Ok(())
    }
    
    /// Create a shared workspace
    pub async fn create_shared_workspace(
        &self,
        workspace: Workspace,
        owner_id: String,
        permissions: WorkspacePermissions,
    ) -> Result<String, WorkspaceError> {
        let shared_workspace = SharedWorkspace::new(workspace, owner_id, permissions);
        let workspace_id = shared_workspace.workspace.id.clone();
        
        self.shared_workspaces.write().await
            .insert(workspace_id.clone(), shared_workspace.clone());
        
        self.event_sender.send(CollaborationEvent::WorkspaceShared {
            workspace_id: workspace_id.clone(),
            owner_id: shared_workspace.owner_id.clone(),
        }).ok();
        
        // Persist shared workspace
        self.persist_shared_workspace(&workspace_id).await?;
        
        Ok(workspace_id)
    }
    
    /// Join a shared workspace
    pub async fn join_workspace(
        &self,
        workspace_id: &str,
        user_id: &str,
        invitation_code: Option<String>,
    ) -> Result<CollaborationSession, WorkspaceError> {
        let mut shared_workspaces = self.shared_workspaces.write().await;
        
        let shared_workspace = shared_workspaces.get_mut(workspace_id)
            .ok_or_else(|| WorkspaceError::NotFound(workspace_id.to_string()))?;
        
        // Check permissions
        if !self.can_join_workspace(shared_workspace, user_id, invitation_code.as_deref()) {
            return Err(WorkspaceError::AccessDenied);
        }
        
        // Add user to workspace
        shared_workspace.add_participant(user_id.to_string());
        
        // Create collaboration session
        let session = CollaborationSession::new(
            workspace_id.to_string(),
            user_id.to_string(),
            self.event_sender.clone(),
        );
        
        let session_id = session.id.clone();
        self.active_sessions.write().await
            .insert(session_id.clone(), session.clone());
        
        self.event_sender.send(CollaborationEvent::UserJoined {
            workspace_id: workspace_id.to_string(),
            user_id: user_id.to_string(),
            session_id: session_id.clone(),
        }).ok();
        
        Ok(session)
    }
    
    /// Leave a shared workspace
    pub async fn leave_workspace(
        &self,
        workspace_id: &str,
        user_id: &str,
    ) -> Result<(), WorkspaceError> {
        let mut shared_workspaces = self.shared_workspaces.write().await;
        
        if let Some(shared_workspace) = shared_workspaces.get_mut(workspace_id) {
            shared_workspace.remove_participant(user_id);
            
            // Remove active session
            let sessions: Vec<String> = self.active_sessions.read().await
                .iter()
                .filter(|(_, session)| session.workspace_id == workspace_id && session.user_id == user_id)
                .map(|(id, _)| id.clone())
                .collect();
            
            let mut active_sessions = self.active_sessions.write().await;
            for session_id in sessions {
                active_sessions.remove(&session_id);
            }
            
            self.event_sender.send(CollaborationEvent::UserLeft {
                workspace_id: workspace_id.to_string(),
                user_id: user_id.to_string(),
            }).ok();
        }
        
        Ok(())
    }
    
    /// Apply a workspace change and sync to all participants
    pub async fn apply_workspace_change(
        &self,
        workspace_id: &str,
        change: WorkspaceChange,
        user_id: &str,
    ) -> Result<(), WorkspaceError> {
        let mut shared_workspaces = self.shared_workspaces.write().await;
        
        let shared_workspace = shared_workspaces.get_mut(workspace_id)
            .ok_or_else(|| WorkspaceError::NotFound(workspace_id.to_string()))?;
        
        // Check if user has permission to make this change
        if !self.can_modify_workspace(shared_workspace, user_id, &change) {
            return Err(WorkspaceError::AccessDenied);
        }
        
        // Apply the change
        self.apply_change_to_workspace(&mut shared_workspace.workspace, &change)?;
        
        // Record the change
        shared_workspace.add_change(change.clone(), user_id.to_string());
        
        // Sync to all participants
        self.sync_engine.broadcast_change(workspace_id, change, user_id).await?;
        
        Ok(())
    }
    
    /// Get shared workspace
    pub async fn get_shared_workspace(&self, workspace_id: &str) -> Option<SharedWorkspace> {
        self.shared_workspaces.read().await.get(workspace_id).cloned()
    }
    
    /// List shared workspaces for a user
    pub async fn list_shared_workspaces(&self, user_id: &str) -> Vec<SharedWorkspace> {
        self.shared_workspaces.read().await
            .values()
            .filter(|ws| ws.can_access(user_id))
            .cloned()
            .collect()
    }
    
    /// Get active sessions for a workspace
    pub async fn get_active_sessions(&self, workspace_id: &str) -> Vec<CollaborationSession> {
        self.active_sessions.read().await
            .values()
            .filter(|session| session.workspace_id == workspace_id)
            .cloned()
            .collect()
    }
    
    /// Register a user
    pub async fn register_user(&self, user: User) -> Result<(), WorkspaceError> {
        self.users.write().await.insert(user.id.clone(), user);
        Ok(())
    }
    
    /// Subscribe to collaboration events
    pub fn subscribe(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.event_sender.subscribe()
    }
    
    /// Private helper methods
    fn can_join_workspace(
        &self,
        shared_workspace: &SharedWorkspace,
        user_id: &str,
        invitation_code: Option<&str>,
    ) -> bool {
        // Check if user is already a participant
        if shared_workspace.participants.contains(user_id) {
            return true;
        }
        
        // Check permissions
        match shared_workspace.permissions.access_level {
            AccessLevel::Public => true,
            AccessLevel::InviteOnly => {
                if let Some(code) = invitation_code {
                    shared_workspace.permissions.invitation_codes.contains(code)
                } else {
                    false
                }
            }
            AccessLevel::Private => {
                shared_workspace.permissions.allowed_users.contains(user_id)
            }
        }
    }
    
    fn can_modify_workspace(
        &self,
        shared_workspace: &SharedWorkspace,
        user_id: &str,
        change: &WorkspaceChange,
    ) -> bool {
        // Owner can do anything
        if shared_workspace.owner_id == user_id {
            return true;
        }
        
        // Check role-based permissions
        if let Some(role) = shared_workspace.permissions.user_roles.get(user_id) {
            match role {
                UserRole::Admin => true,
                UserRole::Editor => matches!(change, 
                    WorkspaceChange::AddNode { .. } | 
                    WorkspaceChange::RemoveNode { .. } |
                    WorkspaceChange::MoveNode { .. } |
                    WorkspaceChange::UpdateNodeProperties { .. }
                ),
                UserRole::Viewer => false,
            }
        } else {
            false
        }
    }
    
    fn apply_change_to_workspace(
        &self,
        workspace: &mut Workspace,
        change: &WorkspaceChange,
    ) -> Result<(), WorkspaceError> {
        match change {
            WorkspaceChange::AddNode { node_id } => {
                workspace.add_node(*node_id);
            }
            WorkspaceChange::RemoveNode { node_id } => {
                workspace.remove_node(*node_id);
            }
            WorkspaceChange::MoveNode { node_id, position } => {
                // Update node position in workspace layout
                let pos_3d = nalgebra::Point3::new(position.0, position.1, 0.0);
                workspace.layout.node_positions.insert(*node_id, pos_3d);
            }
            WorkspaceChange::UpdateNodeProperties { node_id, properties } => {
                // Update node properties in metadata
                workspace.metadata.insert(
                    format!("node_{}", node_id),
                    serde_json::to_value(properties).unwrap_or_default(),
                );
            }
            WorkspaceChange::UpdateLayout { layout } => {
                workspace.layout = layout.clone();
            }
            WorkspaceChange::UpdateSettings { settings } => {
                workspace.settings = settings.clone();
            }
        }
        
        workspace.touch();
        Ok(())
    }
    
    async fn load_shared_workspaces(&self) -> Result<(), WorkspaceError> {
        // Implementation would load from persistent storage
        // For now, this is a placeholder
        Ok(())
    }
    
    async fn persist_shared_workspace(&self, _workspace_id: &str) -> Result<(), WorkspaceError> {
        // Implementation would save to persistent storage
        // For now, this is a placeholder
        Ok(())
    }
}

/// Shared workspace with collaboration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedWorkspace {
    /// Base workspace
    pub workspace: Workspace,
    /// Owner user ID
    pub owner_id: String,
    /// Workspace permissions
    pub permissions: WorkspacePermissions,
    /// Active participants
    pub participants: HashSet<String>,
    /// Change history
    pub changes: Vec<WorkspaceChangeRecord>,
    /// Sharing metadata
    pub sharing_metadata: SharingMetadata,
}

impl SharedWorkspace {
    /// Create a new shared workspace
    pub fn new(workspace: Workspace, owner_id: String, permissions: WorkspacePermissions) -> Self {
        let mut participants = HashSet::new();
        participants.insert(owner_id.clone());
        
        Self {
            workspace,
            owner_id,
            permissions,
            participants,
            changes: Vec::new(),
            sharing_metadata: SharingMetadata::new(),
        }
    }
    
    /// Add participant to workspace
    pub fn add_participant(&mut self, user_id: String) {
        self.participants.insert(user_id);
        self.sharing_metadata.last_activity = Utc::now();
    }
    
    /// Remove participant from workspace
    pub fn remove_participant(&mut self, user_id: &str) {
        self.participants.remove(user_id);
        self.sharing_metadata.last_activity = Utc::now();
    }
    
    /// Check if user can access workspace
    pub fn can_access(&self, user_id: &str) -> bool {
        self.participants.contains(user_id) || 
        self.permissions.allowed_users.contains(user_id) ||
        self.owner_id == user_id
    }
    
    /// Add change record
    pub fn add_change(&mut self, change: WorkspaceChange, user_id: String) {
        self.changes.push(WorkspaceChangeRecord {
            id: Uuid::new_v4().to_string(),
            change,
            user_id,
            timestamp: Utc::now(),
        });
        
        self.sharing_metadata.last_activity = Utc::now();
    }
}

/// Workspace permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePermissions {
    /// Access level for the workspace
    pub access_level: AccessLevel,
    /// Allowed user IDs (for private workspaces)
    pub allowed_users: HashSet<String>,
    /// Invitation codes (for invite-only workspaces)
    pub invitation_codes: HashSet<String>,
    /// User role assignments
    pub user_roles: HashMap<String, UserRole>,
}

impl Default for WorkspacePermissions {
    fn default() -> Self {
        Self {
            access_level: AccessLevel::Private,
            allowed_users: HashSet::new(),
            invitation_codes: HashSet::new(),
            user_roles: HashMap::new(),
        }
    }
}

/// Access levels for shared workspaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Anyone can join
    Public,
    /// Requires invitation code
    InviteOnly,
    /// Only specific users allowed
    Private,
}

/// User roles in shared workspaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// Can manage workspace and permissions
    Admin,
    /// Can modify workspace content
    Editor,
    /// Can only view workspace
    Viewer,
}

/// User information for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Email address
    pub email: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// User status
    pub status: UserStatus,
    /// User preferences
    pub preferences: UserPreferences,
}

/// User status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// User preferences for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Show cursor of other users
    pub show_other_cursors: bool,
    /// Show real-time edits
    pub show_real_time_edits: bool,
    /// Notification settings
    pub notifications: NotificationSettings,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            show_other_cursors: true,
            show_real_time_edits: true,
            notifications: NotificationSettings::default(),
        }
    }
}

/// Notification settings for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Notify when someone joins workspace
    pub on_user_join: bool,
    /// Notify when someone leaves workspace
    pub on_user_leave: bool,
    /// Notify when workspace is modified
    pub on_workspace_change: bool,
    /// Notify when mentioned in comments
    pub on_mention: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            on_user_join: true,
            on_user_leave: true,
            on_workspace_change: false,
            on_mention: true,
        }
    }
}

/// Collaboration session for a user in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    /// Session ID
    pub id: String,
    /// Workspace ID
    pub workspace_id: String,
    /// User ID
    pub user_id: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Current cursor position
    pub cursor_position: Option<(f32, f32)>,
    /// Session-specific settings
    pub settings: SessionSettings,
}

impl CollaborationSession {
    /// Create a new collaboration session
    pub fn new(
        workspace_id: String,
        user_id: String,
        _event_sender: broadcast::Sender<CollaborationEvent>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            user_id,
            started_at: Utc::now(),
            last_activity: Utc::now(),
            cursor_position: None,
            settings: SessionSettings::default(),
        }
    }
    
    /// Update cursor position
    pub fn update_cursor(&mut self, position: (f32, f32)) {
        self.cursor_position = Some(position);
        self.last_activity = Utc::now();
    }
    
    /// Check if session is active
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_activity);
        duration.num_minutes() < 5 // Consider active if activity within 5 minutes
    }
}

/// Session-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    /// Show my cursor to others
    pub show_cursor: bool,
    /// Show my selections to others
    pub show_selections: bool,
    /// Color for cursor and selections
    pub cursor_color: [f32; 3],
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            show_cursor: true,
            show_selections: true,
            cursor_color: [0.2, 0.6, 1.0], // Blue
        }
    }
}

/// Workspace change types for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceChange {
    /// Add a node to the workspace
    AddNode { node_id: SceneId },
    /// Remove a node from the workspace
    RemoveNode { node_id: SceneId },
    /// Move a node to a new position
    MoveNode { node_id: SceneId, position: (f32, f32) },
    /// Update node properties
    UpdateNodeProperties { node_id: SceneId, properties: HashMap<String, serde_json::Value> },
    /// Update workspace layout
    UpdateLayout { layout: crate::layout::WorkspaceLayout },
    /// Update workspace settings
    UpdateSettings { settings: crate::WorkspaceSettings },
}

/// Record of a workspace change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceChangeRecord {
    /// Change ID
    pub id: String,
    /// The change that was made
    pub change: WorkspaceChange,
    /// User who made the change
    pub user_id: String,
    /// When the change was made
    pub timestamp: DateTime<Utc>,
}

/// Sharing metadata for shared workspaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingMetadata {
    /// When workspace was shared
    pub shared_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Total number of collaborators
    pub total_collaborators: usize,
    /// Invitation links
    pub invitation_links: Vec<InvitationLink>,
}

impl SharingMetadata {
    fn new() -> Self {
        Self {
            shared_at: Utc::now(),
            last_activity: Utc::now(),
            total_collaborators: 1,
            invitation_links: Vec::new(),
        }
    }
}

/// Invitation link for workspace sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationLink {
    /// Link ID
    pub id: String,
    /// Invitation code
    pub code: String,
    /// Link creation time
    pub created_at: DateTime<Utc>,
    /// Link expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Maximum uses
    pub max_uses: Option<usize>,
    /// Current use count
    pub use_count: usize,
    /// Creator user ID
    pub created_by: String,
}

/// Collaboration events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationEvent {
    /// Workspace was shared
    WorkspaceShared { workspace_id: String, owner_id: String },
    /// User joined workspace
    UserJoined { workspace_id: String, user_id: String, session_id: String },
    /// User left workspace
    UserLeft { workspace_id: String, user_id: String },
    /// Workspace was modified
    WorkspaceChanged { workspace_id: String, change: WorkspaceChange, user_id: String },
    /// User cursor moved
    CursorMoved { workspace_id: String, user_id: String, position: (f32, f32) },
    /// User status changed
    UserStatusChanged { user_id: String, status: UserStatus },
}

/// Sync engine for real-time collaboration
pub struct SyncEngine {
    /// Event broadcaster
    event_sender: broadcast::Sender<CollaborationEvent>,
    /// Pending changes queue
    pending_changes: Arc<RwLock<Vec<(String, WorkspaceChange, String)>>>,
    /// Sync interval
    sync_interval: tokio::time::Duration,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(event_sender: broadcast::Sender<CollaborationEvent>) -> Self {
        Self {
            event_sender,
            pending_changes: Arc::new(RwLock::new(Vec::new())),
            sync_interval: tokio::time::Duration::from_millis(100),
        }
    }
    
    /// Initialize sync engine
    pub async fn initialize(&self) -> Result<(), WorkspaceError> {
        // Start sync loop
        let pending_changes = self.pending_changes.clone();
        let event_sender = self.event_sender.clone();
        let sync_interval = self.sync_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval);
            
            loop {
                interval.tick().await;
                
                // Process pending changes
                let changes: Vec<(String, WorkspaceChange, String)> = {
                    let mut pending = pending_changes.write().await;
                    let changes = pending.clone();
                    pending.clear();
                    changes
                };
                
                for (workspace_id, change, user_id) in changes {
                    event_sender.send(CollaborationEvent::WorkspaceChanged {
                        workspace_id,
                        change,
                        user_id,
                    }).ok();
                }
            }
        });
        
        Ok(())
    }
    
    /// Broadcast a change to all participants
    pub async fn broadcast_change(
        &self,
        workspace_id: &str,
        change: WorkspaceChange,
        user_id: &str,
    ) -> Result<(), WorkspaceError> {
        // Queue change for processing
        self.pending_changes.write().await.push((
            workspace_id.to_string(),
            change,
            user_id.to_string(),
        ));
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Workspace;
    
    #[tokio::test]
    async fn test_collaboration_manager_creation() {
        let mut manager = CollaborationManager::new();
        assert!(manager.initialize().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_shared_workspace_creation() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test", "Test workspace");
        let owner_id = "user1".to_string();
        let permissions = WorkspacePermissions::default();
        
        let result = manager.create_shared_workspace(workspace, owner_id, permissions).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_user_permissions() {
        let user = User {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            avatar_url: None,
            status: UserStatus::Online,
            preferences: UserPreferences::default(),
        };
        
        assert_eq!(user.status, UserStatus::Online);
        assert!(user.preferences.show_other_cursors);
    }
    
    #[test]
    fn test_collaboration_session() {
        let (event_sender, _) = broadcast::channel(100);
        let session = CollaborationSession::new(
            "workspace1".to_string(),
            "user1".to_string(),
            event_sender,
        );
        
        assert!(session.is_active());
        assert_eq!(session.workspace_id, "workspace1");
        assert_eq!(session.user_id, "user1");
    }
}