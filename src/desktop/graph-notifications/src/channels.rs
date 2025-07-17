//! Notification channels for categorization and routing

use serde::{Deserialize, Serialize};
use crate::{NotificationPriority, SoundProfile};

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel ID
    pub id: String,
    /// Channel name
    pub name: String,
    /// Channel description
    pub description: Option<String>,
    /// Icon
    pub icon: Option<String>,
    /// Default priority
    pub default_priority: NotificationPriority,
    /// Sound profile
    pub sound: SoundProfile,
    /// Vibration enabled
    pub vibration: bool,
    /// Show on lock screen
    pub show_on_lockscreen: bool,
    /// Show badge
    pub show_badge: bool,
    /// Importance level
    pub importance: ChannelImportance,
}

/// Channel importance levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChannelImportance {
    None,
    Min,
    Low,
    Default,
    High,
    Max,
}

impl NotificationChannel {
    /// Create system channel
    pub fn system() -> Self {
        Self {
            id: "system".to_string(),
            name: "System".to_string(),
            description: Some("System notifications".to_string()),
            icon: Some("gear".to_string()),
            default_priority: NotificationPriority::Normal,
            sound: SoundProfile::Default,
            vibration: false,
            show_on_lockscreen: true,
            show_badge: false,
            importance: ChannelImportance::Default,
        }
    }
    
    /// Create messages channel
    pub fn messages() -> Self {
        Self {
            id: "messages".to_string(),
            name: "Messages".to_string(),
            description: Some("Message notifications".to_string()),
            icon: Some("message".to_string()),
            default_priority: NotificationPriority::High,
            sound: SoundProfile::Message,
            vibration: true,
            show_on_lockscreen: false,
            show_badge: true,
            importance: ChannelImportance::High,
        }
    }
    
    /// Create alerts channel
    pub fn alerts() -> Self {
        Self {
            id: "alerts".to_string(),
            name: "Alerts".to_string(),
            description: Some("Important alerts".to_string()),
            icon: Some("alert".to_string()),
            default_priority: NotificationPriority::Critical,
            sound: SoundProfile::Alert,
            vibration: true,
            show_on_lockscreen: true,
            show_badge: true,
            importance: ChannelImportance::Max,
        }
    }
}