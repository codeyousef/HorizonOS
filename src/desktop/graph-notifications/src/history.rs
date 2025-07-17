//! Notification history management

use crate::Notification;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Notification history storage
pub struct NotificationHistory {
    /// Historical notifications
    entries: Arc<RwLock<VecDeque<HistoryEntry>>>,
    /// Maximum entries
    max_entries: usize,
}

/// History entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Notification
    pub notification: Notification,
    /// When it was created
    pub created_at: DateTime<Utc>,
    /// When it was dismissed
    pub dismissed_at: Option<DateTime<Utc>>,
    /// How it was dismissed
    pub dismissal_reason: Option<DismissalReason>,
}

/// Reasons for dismissal
#[derive(Debug, Clone)]
pub enum DismissalReason {
    UserDismissed,
    Expired,
    ActionTaken(String),
    Replaced,
    SystemCleared,
}

impl NotificationHistory {
    /// Create new history
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            max_entries: 1000,
        }
    }
    
    /// Add notification to history
    pub async fn add(&self, notification: Notification) -> Result<()> {
        let entry = HistoryEntry {
            notification,
            created_at: Utc::now(),
            dismissed_at: None,
            dismissal_reason: None,
        };
        
        let mut entries = self.entries.write().unwrap();
        entries.push_back(entry);
        
        // Limit size
        while entries.len() > self.max_entries {
            entries.pop_front();
        }
        
        Ok(())
    }
    
    /// Update notification in history
    pub async fn update(&self, notification: Notification) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.iter_mut().rev().find(|e| e.notification.id == notification.id) {
            entry.notification = notification;
        }
        Ok(())
    }
    
    /// Mark notification as dismissed
    pub async fn dismiss(&self, id: Uuid) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.iter_mut().rev().find(|e| e.notification.id == id) {
            entry.dismissed_at = Some(Utc::now());
            entry.dismissal_reason = Some(DismissalReason::UserDismissed);
        }
        Ok(())
    }
    
    /// Get recent notifications
    pub fn get_recent(&self, count: usize) -> Vec<HistoryEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().rev().take(count).cloned().collect()
    }
    
    /// Clear history
    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }
}