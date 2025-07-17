//! Notification filtering system

use crate::{Notification, NotificationPriority, NotificationType};
use serde::{Deserialize, Serialize};

/// Notification filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationFilter {
    /// Filter name
    pub name: String,
    /// Filter enabled
    pub enabled: bool,
    /// Filter rules
    pub rules: Vec<FilterRule>,
    /// Filter action
    pub action: FilterAction,
}

/// Filter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterRule {
    /// Filter by source
    Source { pattern: String },
    /// Filter by type
    Type { types: Vec<NotificationType> },
    /// Filter by priority
    Priority { min: NotificationPriority },
    /// Filter by tag
    Tag { tags: Vec<String> },
    /// Filter by time
    Time { start: String, end: String },
    /// Custom filter
    Custom { expression: String },
}

/// Filter action
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterAction {
    /// Block the notification
    Block,
    /// Allow the notification
    Allow,
    /// Modify priority
    ChangePriority(NotificationPriority),
    /// Add to group
    Group,
}

impl NotificationFilter {
    /// Check if notification should be shown
    pub fn should_show(&self, notification: &Notification) -> bool {
        if !self.enabled {
            return true;
        }
        
        let matches = self.rules.iter().all(|rule| self.matches_rule(notification, rule));
        
        match self.action {
            FilterAction::Block => !matches,
            FilterAction::Allow => matches,
            _ => true,
        }
    }
    
    /// Check if notification matches rule
    fn matches_rule(&self, notification: &Notification, rule: &FilterRule) -> bool {
        match rule {
            FilterRule::Source { pattern } => {
                notification.source.name.contains(pattern)
            }
            FilterRule::Type { types } => {
                types.contains(&notification.notification_type)
            }
            FilterRule::Priority { min } => {
                notification.priority >= *min
            }
            FilterRule::Tag { tags } => {
                tags.iter().any(|tag| notification.tags.contains(tag))
            }
            FilterRule::Time { .. } => {
                // TODO: Implement time-based filtering
                true
            }
            FilterRule::Custom { .. } => {
                // TODO: Implement custom expression evaluation
                true
            }
        }
    }
}