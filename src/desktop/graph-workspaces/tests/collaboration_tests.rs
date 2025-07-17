//! Comprehensive test suite for workspace collaboration features

use horizonos_graph_workspaces::*;
use horizonos_graph_workspaces::collaboration::*;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(test)]
mod collaboration_manager_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_collaboration_manager_initialization() {
        let mut manager = CollaborationManager::new();
        let result = manager.initialize().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_shared_workspace_creation() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let permissions = WorkspacePermissions::default();
        
        let result = manager.create_shared_workspace(workspace, owner_id, permissions).await;
        assert!(result.is_ok());
        
        let workspace_id = result.unwrap();
        assert!(!workspace_id.is_empty());
        
        // Verify workspace was created
        let shared_workspace = manager.get_shared_workspace(&workspace_id);
        assert!(shared_workspace.is_some());
        
        let shared_workspace = shared_workspace.unwrap();
        assert_eq!(shared_workspace.workspace.name, "Test Workspace");
        assert_eq!(shared_workspace.owner_id, "user1");
    }
    
    #[tokio::test]
    async fn test_workspace_joining() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test joining workspace
        let result = manager.join_workspace(&workspace_id, "user2", None).await;
        assert!(result.is_ok());
        
        let session = result.unwrap();
        assert_eq!(session.workspace_id, workspace_id);
        assert_eq!(session.user_id, "user2");
        assert!(session.is_active());
    }
    
    #[tokio::test]
    async fn test_workspace_permissions() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Private Workspace", "A private workspace");
        let owner_id = "user1".to_string();
        let permissions = WorkspacePermissions {
            access_level: AccessLevel::Private,
            allowed_users: vec!["user2".to_string()].into_iter().collect(),
            ..Default::default()
        };
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test that allowed user can join
        let result = manager.join_workspace(&workspace_id, "user2", None).await;
        assert!(result.is_ok());
        
        // Test that non-allowed user cannot join
        let result = manager.join_workspace(&workspace_id, "user3", None).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_invitation_code_access() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Invite Only Workspace", "An invite-only workspace");
        let owner_id = "user1".to_string();
        let permissions = WorkspacePermissions {
            access_level: AccessLevel::InviteOnly,
            invitation_codes: vec!["secret123".to_string()].into_iter().collect(),
            ..Default::default()
        };
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test that correct invitation code allows access
        let result = manager.join_workspace(&workspace_id, "user2", Some("secret123".to_string())).await;
        assert!(result.is_ok());
        
        // Test that incorrect invitation code denies access
        let result = manager.join_workspace(&workspace_id, "user3", Some("wrong".to_string())).await;
        assert!(result.is_err());
        
        // Test that no invitation code denies access
        let result = manager.join_workspace(&workspace_id, "user4", None).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_workspace_leaving() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Join workspace
        let session = manager.join_workspace(&workspace_id, "user2", None).await.unwrap();
        
        // Leave workspace
        let result = manager.leave_workspace(&workspace_id, "user2").await;
        assert!(result.is_ok());
        
        // Verify user is no longer in workspace
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert!(!shared_workspace.participants.contains("user2"));
    }
}

#[cfg(test)]
mod workspace_change_tests {
    use super::*;
    use horizonos_graph_engine::scene::SceneId;
    
    #[tokio::test]
    async fn test_workspace_change_application() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test adding a node
        let node_id = SceneId::new();
        let change = WorkspaceChange::AddNode { node_id };
        let result = manager.apply_workspace_change(&workspace_id, change, "user1").await;
        assert!(result.is_ok());
        
        // Verify node was added
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert!(shared_workspace.workspace.nodes.contains(&node_id));
    }
    
    #[tokio::test]
    async fn test_workspace_change_permissions() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        permissions.user_roles.insert("user2".to_string(), UserRole::Viewer);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test that admin can make changes
        let node_id = SceneId::new();
        let change = WorkspaceChange::AddNode { node_id };
        let result = manager.apply_workspace_change(&workspace_id, change, "user1").await;
        assert!(result.is_ok());
        
        // Test that viewer cannot make changes
        let node_id = SceneId::new();
        let change = WorkspaceChange::AddNode { node_id };
        let result = manager.apply_workspace_change(&workspace_id, change, "user2").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_workspace_change_history() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Apply multiple changes
        let node_id1 = SceneId::new();
        let change1 = WorkspaceChange::AddNode { node_id: node_id1 };
        manager.apply_workspace_change(&workspace_id, change1, "user1").await.unwrap();
        
        let node_id2 = SceneId::new();
        let change2 = WorkspaceChange::AddNode { node_id: node_id2 };
        manager.apply_workspace_change(&workspace_id, change2, "user1").await.unwrap();
        
        // Verify change history
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert_eq!(shared_workspace.changes.len(), 2);
        assert_eq!(shared_workspace.changes[0].user_id, "user1");
        assert_eq!(shared_workspace.changes[1].user_id, "user1");
    }
}

#[cfg(test)]
mod user_management_tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            status: UserStatus::Online,
            preferences: UserPreferences::default(),
        };
        
        assert_eq!(user.id, "user1");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.status, UserStatus::Online);
        assert!(user.preferences.show_other_cursors);
    }
    
    #[test]
    fn test_user_preferences() {
        let mut preferences = UserPreferences::default();
        assert!(preferences.show_other_cursors);
        assert!(preferences.show_real_time_edits);
        assert!(preferences.notifications.on_user_join);
        
        preferences.show_other_cursors = false;
        preferences.notifications.on_user_join = false;
        
        assert!(!preferences.show_other_cursors);
        assert!(!preferences.notifications.on_user_join);
    }
    
    #[test]
    fn test_user_status_transitions() {
        let statuses = [
            UserStatus::Online,
            UserStatus::Away,
            UserStatus::Busy,
            UserStatus::Offline,
        ];
        
        for status in statuses {
            let user = User {
                id: "user1".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                avatar_url: None,
                status,
                preferences: UserPreferences::default(),
            };
            
            assert_eq!(user.status, status);
        }
    }
    
    #[tokio::test]
    async fn test_user_registration() {
        let manager = CollaborationManager::new();
        let user = User {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            avatar_url: None,
            status: UserStatus::Online,
            preferences: UserPreferences::default(),
        };
        
        let result = manager.register_user(user);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod collaboration_session_tests {
    use super::*;
    use tokio::sync::broadcast;
    
    #[test]
    fn test_session_creation() {
        let (event_sender, _) = broadcast::channel(100);
        let session = CollaborationSession::new(
            "workspace1".to_string(),
            "user1".to_string(),
            event_sender,
        );
        
        assert_eq!(session.workspace_id, "workspace1");
        assert_eq!(session.user_id, "user1");
        assert!(session.is_active());
        assert!(session.cursor_position.is_none());
    }
    
    #[test]
    fn test_session_cursor_updates() {
        let (event_sender, _) = broadcast::channel(100);
        let mut session = CollaborationSession::new(
            "workspace1".to_string(),
            "user1".to_string(),
            event_sender,
        );
        
        session.update_cursor((100.0, 200.0));
        assert_eq!(session.cursor_position, Some((100.0, 200.0)));
        
        session.update_cursor((150.0, 250.0));
        assert_eq!(session.cursor_position, Some((150.0, 250.0)));
    }
    
    #[test]
    fn test_session_activity_tracking() {
        let (event_sender, _) = broadcast::channel(100);
        let session = CollaborationSession::new(
            "workspace1".to_string(),
            "user1".to_string(),
            event_sender,
        );
        
        // New session should be active
        assert!(session.is_active());
        
        // Test with old session (simulated)
        let old_time = chrono::Utc::now() - chrono::Duration::minutes(10);
        let old_session = CollaborationSession {
            id: "session1".to_string(),
            workspace_id: "workspace1".to_string(),
            user_id: "user1".to_string(),
            started_at: old_time,
            last_activity: old_time,
            cursor_position: None,
            settings: SessionSettings::default(),
        };
        
        // Old session should not be active
        assert!(!old_session.is_active());
    }
    
    #[test]
    fn test_session_settings() {
        let settings = SessionSettings::default();
        assert!(settings.show_cursor);
        assert!(settings.show_selections);
        assert_eq!(settings.cursor_color, [0.2, 0.6, 1.0]);
        
        let custom_settings = SessionSettings {
            show_cursor: false,
            show_selections: false,
            cursor_color: [1.0, 0.0, 0.0],
        };
        
        assert!(!custom_settings.show_cursor);
        assert!(!custom_settings.show_selections);
        assert_eq!(custom_settings.cursor_color, [1.0, 0.0, 0.0]);
    }
}

#[cfg(test)]
mod sync_engine_tests {
    use super::*;
    use tokio::sync::broadcast;
    
    #[tokio::test]
    async fn test_sync_engine_initialization() {
        let (event_sender, _) = broadcast::channel(100);
        let sync_engine = SyncEngine::new(event_sender);
        
        let result = sync_engine.initialize().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_sync_engine_change_broadcasting() {
        let (event_sender, mut event_receiver) = broadcast::channel(100);
        let sync_engine = SyncEngine::new(event_sender);
        
        sync_engine.initialize().await.unwrap();
        
        // Broadcast a change
        let change = WorkspaceChange::AddNode { node_id: SceneId::new() };
        let result = sync_engine.broadcast_change("workspace1", change.clone(), "user1").await;
        assert!(result.is_ok());
        
        // Wait for event processing
        sleep(Duration::from_millis(150)).await;
        
        // Check that event was received
        let received_event = event_receiver.try_recv();
        assert!(received_event.is_ok());
        
        if let Ok(CollaborationEvent::WorkspaceChanged { workspace_id, user_id, .. }) = received_event {
            assert_eq!(workspace_id, "workspace1");
            assert_eq!(user_id, "user1");
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_collaboration_workflow() {
        let manager = CollaborationManager::new();
        
        // Create a shared workspace
        let workspace = Workspace::new("Collaboration Test", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        permissions.user_roles.insert("user2".to_string(), UserRole::Editor);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Register users
        let user1 = User {
            id: "user1".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            avatar_url: None,
            status: UserStatus::Online,
            preferences: UserPreferences::default(),
        };
        
        let user2 = User {
            id: "user2".to_string(),
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            avatar_url: None,
            status: UserStatus::Online,
            preferences: UserPreferences::default(),
        };
        
        manager.register_user(user1).unwrap();
        manager.register_user(user2).unwrap();
        
        // User2 joins workspace
        let session = manager.join_workspace(&workspace_id, "user2", None).await.unwrap();
        assert_eq!(session.user_id, "user2");
        
        // User1 makes changes
        let node_id = SceneId::new();
        let change = WorkspaceChange::AddNode { node_id };
        let result = manager.apply_workspace_change(&workspace_id, change, "user1").await;
        assert!(result.is_ok());
        
        // User2 makes changes
        let node_id2 = SceneId::new();
        let change2 = WorkspaceChange::AddNode { node_id: node_id2 };
        let result2 = manager.apply_workspace_change(&workspace_id, change2, "user2").await;
        assert!(result2.is_ok());
        
        // Verify workspace state
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert_eq!(shared_workspace.workspace.nodes.len(), 2);
        assert!(shared_workspace.workspace.nodes.contains(&node_id));
        assert!(shared_workspace.workspace.nodes.contains(&node_id2));
        assert_eq!(shared_workspace.changes.len(), 2);
        assert!(shared_workspace.participants.contains("user1"));
        assert!(shared_workspace.participants.contains("user2"));
        
        // User2 leaves workspace
        let result = manager.leave_workspace(&workspace_id, "user2").await;
        assert!(result.is_ok());
        
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert!(!shared_workspace.participants.contains("user2"));
    }
    
    #[tokio::test]
    async fn test_concurrent_collaboration() {
        let manager = std::sync::Arc::new(CollaborationManager::new());
        
        // Create a shared workspace
        let workspace = Workspace::new("Concurrent Test", "A test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        permissions.user_roles.insert("user2".to_string(), UserRole::Editor);
        permissions.user_roles.insert("user3".to_string(), UserRole::Editor);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Spawn multiple users making concurrent changes
        let mut tasks = Vec::new();
        
        for i in 2..=5 {
            let manager_clone = manager.clone();
            let workspace_id_clone = workspace_id.clone();
            let user_id = format!("user{}", i);
            
            let task = tokio::spawn(async move {
                // Join workspace
                let _session = manager_clone.join_workspace(&workspace_id_clone, &user_id, None).await.unwrap();
                
                // Make multiple changes
                for j in 0..10 {
                    let node_id = SceneId::new();
                    let change = WorkspaceChange::AddNode { node_id };
                    let result = manager_clone.apply_workspace_change(&workspace_id_clone, change, &user_id).await;
                    if result.is_err() {
                        eprintln!("Error applying change for {}: {:?}", user_id, result);
                    }
                    
                    // Small delay to simulate realistic usage
                    sleep(Duration::from_millis(10)).await;
                }
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        for task in tasks {
            task.await.unwrap();
        }
        
        // Verify final state
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert!(shared_workspace.workspace.nodes.len() > 0);
        assert!(shared_workspace.changes.len() > 0);
        
        // All users should have been participants at some point
        let participants: Vec<_> = shared_workspace.changes.iter()
            .map(|change| change.user_id.clone())
            .collect();
        
        assert!(participants.contains(&"user2".to_string()));
        assert!(participants.contains(&"user3".to_string()));
        assert!(participants.contains(&"user4".to_string()));
        assert!(participants.contains(&"user5".to_string()));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_nonexistent_workspace_errors() {
        let manager = CollaborationManager::new();
        
        // Test joining non-existent workspace
        let result = manager.join_workspace("nonexistent", "user1", None).await;
        assert!(result.is_err());
        
        // Test leaving non-existent workspace
        let result = manager.leave_workspace("nonexistent", "user1").await;
        assert!(result.is_ok()); // Should not error
        
        // Test applying change to non-existent workspace
        let change = WorkspaceChange::AddNode { node_id: SceneId::new() };
        let result = manager.apply_workspace_change("nonexistent", change, "user1").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_permission_errors() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Test Workspace", "A test workspace");
        let owner_id = "user1".to_string();
        let permissions = WorkspacePermissions {
            access_level: AccessLevel::Private,
            ..Default::default()
        };
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Test unauthorized access
        let result = manager.join_workspace(&workspace_id, "user2", None).await;
        assert!(result.is_err());
        
        // Test unauthorized change
        let change = WorkspaceChange::AddNode { node_id: SceneId::new() };
        let result = manager.apply_workspace_change(&workspace_id, change, "user2").await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_large_workspace_performance() {
        let manager = CollaborationManager::new();
        let workspace = Workspace::new("Large Test", "A large test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        permissions.user_roles.insert("user1".to_string(), UserRole::Admin);
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Add many nodes
        let start = std::time::Instant::now();
        for i in 0..1000 {
            let node_id = SceneId::new();
            let change = WorkspaceChange::AddNode { node_id };
            let result = manager.apply_workspace_change(&workspace_id, change, "user1").await;
            assert!(result.is_ok());
            
            // Progress indicator
            if i % 100 == 0 {
                println!("Added {} nodes", i);
            }
        }
        let duration = start.elapsed();
        
        println!("Added 1000 nodes in {:?}", duration);
        assert!(duration < Duration::from_secs(10)); // Should be reasonably fast
        
        // Verify final state
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert_eq!(shared_workspace.workspace.nodes.len(), 1000);
        assert_eq!(shared_workspace.changes.len(), 1000);
    }
    
    #[tokio::test]
    async fn test_many_users_performance() {
        let manager = std::sync::Arc::new(CollaborationManager::new());
        let workspace = Workspace::new("Multi-user Test", "A multi-user test workspace");
        let owner_id = "user1".to_string();
        let mut permissions = WorkspacePermissions::default();
        permissions.access_level = AccessLevel::Public;
        
        let workspace_id = manager.create_shared_workspace(workspace, owner_id, permissions).await.unwrap();
        
        // Register many users
        let start = std::time::Instant::now();
        for i in 1..=100 {
            let user = User {
                id: format!("user{}", i),
                name: format!("User {}", i),
                email: format!("user{}@example.com", i),
                avatar_url: None,
                status: UserStatus::Online,
                preferences: UserPreferences::default(),
            };
            
            manager.register_user(user).unwrap();
        }
        let registration_duration = start.elapsed();
        
        println!("Registered 100 users in {:?}", registration_duration);
        assert!(registration_duration < Duration::from_secs(5));
        
        // Have users join concurrently
        let start = std::time::Instant::now();
        let mut tasks = Vec::new();
        
        for i in 1..=50 {
            let manager_clone = manager.clone();
            let workspace_id_clone = workspace_id.clone();
            let user_id = format!("user{}", i);
            
            let task = tokio::spawn(async move {
                let result = manager_clone.join_workspace(&workspace_id_clone, &user_id, None).await;
                assert!(result.is_ok());
            });
            
            tasks.push(task);
        }
        
        for task in tasks {
            task.await.unwrap();
        }
        
        let join_duration = start.elapsed();
        println!("50 users joined in {:?}", join_duration);
        assert!(join_duration < Duration::from_secs(10));
        
        // Verify final state
        let shared_workspace = manager.get_shared_workspace(&workspace_id).unwrap();
        assert_eq!(shared_workspace.participants.len(), 51); // 50 + owner
    }
}