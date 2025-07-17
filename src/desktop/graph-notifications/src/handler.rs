//! Notification action handler

use crate::{Notification, NotificationAction};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Handler for notification actions
pub struct NotificationHandler {
    /// Action handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn ActionHandler>>>>,
}

/// Trait for action handlers
pub trait ActionHandler: Send + Sync {
    /// Handle the action
    fn handle(&self, notification: &Notification, action: &NotificationAction) -> Result<()>;
}

impl NotificationHandler {
    /// Create new notification handler
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register action handler
    pub fn register_handler(&self, action_id: String, handler: Box<dyn ActionHandler>) {
        self.handlers.write().unwrap().insert(action_id, handler);
    }
    
    /// Handle notification action
    pub fn handle_action(&self, notification: &Notification, action: &NotificationAction) -> Result<()> {
        if let Some(handler) = self.handlers.read().unwrap().get(&action.id) {
            handler.handle(notification, action)
        } else {
            log::warn!("No handler registered for action: {}", action.id);
            Ok(())
        }
    }
}