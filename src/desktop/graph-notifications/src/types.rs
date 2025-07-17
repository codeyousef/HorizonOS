//! Notification type definitions and utilities

use serde::{Deserialize, Serialize};
use std::fmt;

/// Extended notification metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetadata {
    /// Application that created the notification
    pub app_name: Option<String>,
    /// Category for grouping
    pub category: Option<String>,
    /// Urgency level (freedesktop.org compliant)
    pub urgency: Option<NotificationUrgency>,
    /// Hints for presentation
    pub hints: NotificationHints,
    /// Custom properties
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Notification urgency levels (freedesktop.org specification)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationUrgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

/// Notification presentation hints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationHints {
    /// Action icons to use
    pub action_icons: Option<bool>,
    /// Desktop entry name
    pub desktop_entry: Option<String>,
    /// Image data (raw ARGB)
    pub image_data: Option<Vec<u8>>,
    /// Image path
    pub image_path: Option<String>,
    /// Notification category
    pub category: Option<String>,
    /// Sound name from sound theme
    pub sound_name: Option<String>,
    /// Sound file path
    pub sound_file: Option<String>,
    /// Suppress sound
    pub suppress_sound: Option<bool>,
    /// X position hint
    pub x: Option<i32>,
    /// Y position hint
    pub y: Option<i32>,
    /// Transient notification
    pub transient: Option<bool>,
    /// Resident notification (stays after action)
    pub resident: Option<bool>,
}

/// Graph-specific notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNotificationData {
    /// Associated node IDs
    pub node_ids: Vec<u64>,
    /// Associated edge IDs
    pub edge_ids: Vec<(u64, u64)>,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Cluster ID
    pub cluster_id: Option<String>,
    /// Graph position hint
    pub position: Option<[f32; 3]>,
    /// Visual emphasis
    pub emphasis: NotificationEmphasis,
}

/// Visual emphasis for graph notifications
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NotificationEmphasis {
    /// No special emphasis
    None,
    /// Highlight associated nodes
    Highlight,
    /// Pulse effect on nodes
    Pulse,
    /// Glow effect
    Glow,
    /// Shake for attention
    Shake,
    /// Zoom to nodes
    ZoomTo,
}

/// Notification sound profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundProfile {
    /// System default sound
    Default,
    /// Message received sound
    Message,
    /// Alert/warning sound
    Alert,
    /// Error sound
    Error,
    /// Success/completion sound
    Success,
    /// Custom sound file
    Custom(String),
    /// No sound
    Silent,
}

/// Notification templates for common scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationTemplate {
    /// File operation completed
    FileOperation {
        operation: FileOperationType,
        file_count: usize,
        total_size: Option<u64>,
    },
    /// Application update available
    UpdateAvailable {
        app_name: String,
        current_version: String,
        new_version: String,
    },
    /// AI suggestion
    AiSuggestion {
        suggestion_type: AiSuggestionType,
        confidence: f32,
        actions: Vec<String>,
    },
    /// System alert
    SystemAlert {
        alert_type: SystemAlertType,
        severity: AlertSeverity,
    },
    /// Task completion
    TaskComplete {
        task_name: String,
        duration: std::time::Duration,
        result: TaskResult,
    },
    /// Connection status
    ConnectionStatus {
        service: String,
        connected: bool,
        details: Option<String>,
    },
}

/// File operation types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileOperationType {
    Copy,
    Move,
    Delete,
    Compress,
    Extract,
    Download,
    Upload,
    Sync,
}

/// AI suggestion types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AiSuggestionType {
    RelationshipDiscovered,
    ClusterSuggestion,
    WorkflowOptimization,
    DuplicateDetection,
    OrganizationSuggestion,
    AutomationOpportunity,
}

/// System alert types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SystemAlertType {
    LowMemory,
    HighCpu,
    DiskSpace,
    NetworkIssue,
    SecurityWarning,
    SystemUpdate,
    BackupReminder,
    MaintenanceRequired,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Task completion results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Success,
    PartialSuccess { completed: usize, total: usize },
    Failed { error: String },
    Cancelled,
}

/// Progress notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotificationData {
    /// Current progress (0-100)
    pub current: u8,
    /// Progress message
    pub message: Option<String>,
    /// Estimated time remaining
    pub eta: Option<std::time::Duration>,
    /// Bytes processed (for file operations)
    pub bytes_processed: Option<u64>,
    /// Total bytes (for file operations)
    pub total_bytes: Option<u64>,
    /// Current operation
    pub operation: Option<String>,
    /// Cancelable operation
    pub cancelable: bool,
}

/// Rich notification content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichContent {
    /// Formatted text with markup
    pub formatted_body: Option<String>,
    /// Inline images
    pub images: Vec<InlineImage>,
    /// Embedded buttons
    pub buttons: Vec<NotificationButton>,
    /// Progress indicators
    pub progress: Option<ProgressNotificationData>,
    /// Lists
    pub lists: Vec<NotificationList>,
}

/// Inline image in notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineImage {
    /// Image URL or path
    pub source: String,
    /// Alt text
    pub alt: Option<String>,
    /// Maximum width
    pub max_width: Option<u32>,
    /// Maximum height  
    pub max_height: Option<u32>,
}

/// Button in notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationButton {
    /// Button ID for action handling
    pub id: String,
    /// Button label
    pub label: String,
    /// Button icon
    pub icon: Option<String>,
    /// Button style
    pub style: ButtonStyle,
}

/// Button styles
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ButtonStyle {
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Link,
}

/// List in notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationList {
    /// List title
    pub title: Option<String>,
    /// List items
    pub items: Vec<String>,
    /// List style
    pub style: ListStyle,
}

/// List styles
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ListStyle {
    Bullet,
    Numbered,
    Checklist,
}

impl Default for NotificationMetadata {
    fn default() -> Self {
        Self {
            app_name: None,
            category: None,
            urgency: None,
            hints: NotificationHints::default(),
            properties: std::collections::HashMap::new(),
        }
    }
}

impl fmt::Display for NotificationUrgency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationUrgency::Low => write!(f, "low"),
            NotificationUrgency::Normal => write!(f, "normal"),
            NotificationUrgency::Critical => write!(f, "critical"),
        }
    }
}

impl fmt::Display for FileOperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileOperationType::Copy => write!(f, "Copy"),
            FileOperationType::Move => write!(f, "Move"),
            FileOperationType::Delete => write!(f, "Delete"),
            FileOperationType::Compress => write!(f, "Compress"),
            FileOperationType::Extract => write!(f, "Extract"),
            FileOperationType::Download => write!(f, "Download"),
            FileOperationType::Upload => write!(f, "Upload"),
            FileOperationType::Sync => write!(f, "Sync"),
        }
    }
}

impl fmt::Display for AiSuggestionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiSuggestionType::RelationshipDiscovered => write!(f, "Relationship Discovered"),
            AiSuggestionType::ClusterSuggestion => write!(f, "Cluster Suggestion"),
            AiSuggestionType::WorkflowOptimization => write!(f, "Workflow Optimization"),
            AiSuggestionType::DuplicateDetection => write!(f, "Duplicate Detection"),
            AiSuggestionType::OrganizationSuggestion => write!(f, "Organization Suggestion"),
            AiSuggestionType::AutomationOpportunity => write!(f, "Automation Opportunity"),
        }
    }
}

impl fmt::Display for SystemAlertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemAlertType::LowMemory => write!(f, "Low Memory"),
            SystemAlertType::HighCpu => write!(f, "High CPU Usage"),
            SystemAlertType::DiskSpace => write!(f, "Low Disk Space"),
            SystemAlertType::NetworkIssue => write!(f, "Network Issue"),
            SystemAlertType::SecurityWarning => write!(f, "Security Warning"),
            SystemAlertType::SystemUpdate => write!(f, "System Update"),
            SystemAlertType::BackupReminder => write!(f, "Backup Reminder"),
            SystemAlertType::MaintenanceRequired => write!(f, "Maintenance Required"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_urgency_display() {
        assert_eq!(NotificationUrgency::Low.to_string(), "low");
        assert_eq!(NotificationUrgency::Normal.to_string(), "normal");
        assert_eq!(NotificationUrgency::Critical.to_string(), "critical");
    }
    
    #[test]
    fn test_file_operation_display() {
        assert_eq!(FileOperationType::Copy.to_string(), "Copy");
        assert_eq!(FileOperationType::Download.to_string(), "Download");
    }
    
    #[test]
    fn test_notification_metadata_default() {
        let metadata = NotificationMetadata::default();
        assert!(metadata.app_name.is_none());
        assert!(metadata.category.is_none());
        assert!(metadata.urgency.is_none());
        assert!(metadata.properties.is_empty());
    }
}