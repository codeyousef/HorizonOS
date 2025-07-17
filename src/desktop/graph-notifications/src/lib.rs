//! Notification system for HorizonOS graph desktop
//! 
//! This module provides a comprehensive notification system that integrates
//! with the graph desktop, supporting various notification types, priorities,
//! and delivery mechanisms.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use async_trait::async_trait;

pub mod manager;
pub mod types;
pub mod renderer;
pub mod handler;
pub mod history;
pub mod filters;
pub mod actions;
pub mod channels;

pub use manager::NotificationManager;
pub use types::*;
pub use renderer::NotificationRenderer;
pub use handler::NotificationHandler;
pub use history::NotificationHistory;
pub use filters::NotificationFilter;
pub use actions::NotificationAction;
pub use channels::NotificationChannel;

/// Notification system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Maximum number of notifications to display
    pub max_visible: usize,
    /// Default timeout for notifications
    pub default_timeout: Duration,
    /// Enable notification sounds
    pub enable_sounds: bool,
    /// Enable visual effects
    pub enable_effects: bool,
    /// Notification position on screen
    pub position: NotificationPosition,
    /// Animation settings
    pub animations: AnimationConfig,
    /// Priority thresholds
    pub priority_settings: PrioritySettings,
    /// Do not disturb mode
    pub do_not_disturb: bool,
    /// Notification grouping
    pub enable_grouping: bool,
    /// History settings
    pub history_settings: HistorySettings,
}

/// Notification position on screen
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NotificationPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Center,
    GraphIntegrated, // Special position for graph integration
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Slide in animation
    pub slide_in: bool,
    /// Fade in animation
    pub fade_in: bool,
    /// Animation duration
    pub duration: Duration,
    /// Animation easing
    pub easing: AnimationEasing,
}

/// Animation easing types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnimationEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Spring,
}

/// Priority settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritySettings {
    /// Critical notifications bypass do not disturb
    pub critical_bypass_dnd: bool,
    /// High priority timeout multiplier
    pub high_timeout_multiplier: f32,
    /// Low priority auto-dismiss
    pub low_auto_dismiss: bool,
}

/// History settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySettings {
    /// Enable history
    pub enabled: bool,
    /// Maximum history entries
    pub max_entries: usize,
    /// History retention period
    pub retention_days: u32,
    /// Store dismissed notifications
    pub store_dismissed: bool,
}

/// Main notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique notification ID
    pub id: Uuid,
    /// Notification type
    pub notification_type: NotificationType,
    /// Title
    pub title: String,
    /// Body text
    pub body: String,
    /// Priority level
    pub priority: NotificationPriority,
    /// Source of the notification
    pub source: NotificationSource,
    /// Icon
    pub icon: Option<String>,
    /// Image attachment
    pub image: Option<String>,
    /// Actions available
    pub actions: Vec<NotificationAction>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Expiry time
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether it has been read
    pub read: bool,
    /// Whether it has been dismissed
    pub dismissed: bool,
    /// Associated graph node ID
    pub node_id: Option<u64>,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Notification group
    pub group: Option<String>,
    /// Progress value (0-100)
    pub progress: Option<u8>,
    /// Sound to play
    pub sound: Option<String>,
    /// Vibration pattern
    pub vibration: Option<Vec<u32>>,
    /// Persistent notification
    pub persistent: bool,
    /// Private notification (hide content on lock screen)
    pub private: bool,
}

/// Notification types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// System notification
    System,
    /// Application notification
    Application,
    /// Message notification
    Message,
    /// Alert notification
    Alert,
    /// Progress notification
    Progress,
    /// Error notification
    Error,
    /// Success notification
    Success,
    /// Info notification
    Info,
    /// Calendar/event notification
    Calendar,
    /// Reminder notification
    Reminder,
    /// Update notification
    Update,
    /// Network notification
    Network,
    /// Security notification
    Security,
    /// AI suggestion
    AiSuggestion,
    /// Graph event
    GraphEvent,
}

/// Notification priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Notification source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSource {
    /// Application or system component name
    pub name: String,
    /// Application ID
    pub app_id: Option<String>,
    /// Process ID
    pub pid: Option<u32>,
    /// Icon for the source
    pub icon: Option<String>,
}

/// Notification event for handling
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// New notification created
    Created(Notification),
    /// Notification updated
    Updated(Notification),
    /// Notification dismissed
    Dismissed(Uuid),
    /// Notification expired
    Expired(Uuid),
    /// Action triggered
    ActionTriggered {
        notification_id: Uuid,
        action_id: String,
    },
    /// Notification clicked
    Clicked(Uuid),
    /// Group expanded
    GroupExpanded(String),
    /// Group collapsed
    GroupCollapsed(String),
}

/// Trait for notification providers
#[async_trait]
pub trait NotificationProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;
    
    /// Initialize the provider
    async fn initialize(&mut self) -> Result<()>;
    
    /// Send a notification
    async fn send(&self, notification: &Notification) -> Result<()>;
    
    /// Update a notification
    async fn update(&self, notification: &Notification) -> Result<()>;
    
    /// Dismiss a notification
    async fn dismiss(&self, id: Uuid) -> Result<()>;
    
    /// Check if provider is available
    fn is_available(&self) -> bool;
}

/// Trait for notification renderers
#[async_trait]
pub trait NotificationRenderTarget: Send + Sync {
    /// Render a notification
    async fn render(&mut self, notification: &Notification, position: NotificationPosition) -> Result<()>;
    
    /// Update rendered notification
    async fn update_render(&mut self, notification: &Notification) -> Result<()>;
    
    /// Remove rendered notification
    async fn remove_render(&mut self, id: Uuid) -> Result<()>;
    
    /// Animate notification
    async fn animate(&mut self, id: Uuid, animation: NotificationAnimation) -> Result<()>;
}

/// Notification animations
#[derive(Debug, Clone)]
pub enum NotificationAnimation {
    /// Slide in from edge
    SlideIn { duration: Duration, direction: SlideDirection },
    /// Slide out to edge
    SlideOut { duration: Duration, direction: SlideDirection },
    /// Fade in
    FadeIn { duration: Duration },
    /// Fade out
    FadeOut { duration: Duration },
    /// Shake for attention
    Shake { duration: Duration, intensity: f32 },
    /// Pulse effect
    Pulse { duration: Duration, count: u32 },
    /// Progress update
    ProgressUpdate { value: u8, duration: Duration },
}

/// Slide directions
#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
    Left,
    Right,
    Top,
    Bottom,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            max_visible: 5,
            default_timeout: Duration::from_secs(5),
            enable_sounds: true,
            enable_effects: true,
            position: NotificationPosition::TopRight,
            animations: AnimationConfig::default(),
            priority_settings: PrioritySettings::default(),
            do_not_disturb: false,
            enable_grouping: true,
            history_settings: HistorySettings::default(),
        }
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            slide_in: true,
            fade_in: true,
            duration: Duration::from_millis(300),
            easing: AnimationEasing::EaseInOut,
        }
    }
}

impl Default for PrioritySettings {
    fn default() -> Self {
        Self {
            critical_bypass_dnd: true,
            high_timeout_multiplier: 2.0,
            low_auto_dismiss: true,
        }
    }
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            retention_days: 7,
            store_dismissed: true,
        }
    }
}

impl Notification {
    /// Create a new notification
    pub fn new(title: String, body: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type: NotificationType::Info,
            title,
            body,
            priority: NotificationPriority::Normal,
            source: NotificationSource {
                name: "System".to_string(),
                app_id: None,
                pid: None,
                icon: None,
            },
            icon: None,
            image: None,
            actions: Vec::new(),
            timestamp: Utc::now(),
            expires_at: None,
            read: false,
            dismissed: false,
            node_id: None,
            metadata: HashMap::new(),
            tags: Vec::new(),
            group: None,
            progress: None,
            sound: None,
            vibration: None,
            persistent: false,
            private: false,
        }
    }
    
    /// Set notification type
    pub fn with_type(mut self, notification_type: NotificationType) -> Self {
        self.notification_type = notification_type;
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set source
    pub fn with_source(mut self, source: NotificationSource) -> Self {
        self.source = source;
        self
    }
    
    /// Set icon
    pub fn with_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
    
    /// Add action
    pub fn add_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }
    
    /// Set associated node
    pub fn with_node(mut self, node_id: u64) -> Self {
        self.node_id = Some(node_id);
        self
    }
    
    /// Set expiry
    pub fn expires_in(mut self, duration: Duration) -> Self {
        self.expires_at = Some(Utc::now() + chrono::Duration::from_std(duration).unwrap());
        self
    }
    
    /// Set as persistent
    pub fn persistent(mut self) -> Self {
        self.persistent = true;
        self
    }
    
    /// Set as private
    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }
    
    /// Check if notification is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            "Test Title".to_string(),
            "Test Body".to_string()
        );
        
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.body, "Test Body");
        assert_eq!(notification.priority, NotificationPriority::Normal);
        assert!(!notification.read);
        assert!(!notification.dismissed);
    }
    
    #[test]
    fn test_notification_builder() {
        let notification = Notification::new(
            "Test".to_string(),
            "Body".to_string()
        )
        .with_type(NotificationType::Alert)
        .with_priority(NotificationPriority::High)
        .with_icon("alert-icon".to_string())
        .persistent();
        
        assert_eq!(notification.notification_type, NotificationType::Alert);
        assert_eq!(notification.priority, NotificationPriority::High);
        assert_eq!(notification.icon, Some("alert-icon".to_string()));
        assert!(notification.persistent);
    }
    
    #[test]
    fn test_notification_expiry() {
        let mut notification = Notification::new(
            "Test".to_string(),
            "Body".to_string()
        );
        
        assert!(!notification.is_expired());
        
        notification = notification.expires_in(Duration::from_secs(1));
        assert!(!notification.is_expired());
        
        // Set expiry to past
        notification.expires_at = Some(Utc::now() - chrono::Duration::seconds(1));
        assert!(notification.is_expired());
    }
}