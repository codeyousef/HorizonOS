//! Notification manager for coordinating all notification operations

use crate::{
    Notification, NotificationConfig, NotificationEvent, NotificationFilter, NotificationHistory,
    NotificationPosition, NotificationPriority, NotificationProvider, NotificationRenderTarget,
    NotificationChannel, NotificationAnimation, SlideDirection
};
use anyhow::{Result, Context};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;
use log::{debug, info, warn, error};
use std::time::Duration;

/// Central notification manager
pub struct NotificationManager {
    /// Configuration
    config: Arc<RwLock<NotificationConfig>>,
    /// Active notifications
    active: Arc<RwLock<HashMap<Uuid, Notification>>>,
    /// Notification queue
    queue: Arc<RwLock<VecDeque<Notification>>>,
    /// Notification history
    history: Arc<NotificationHistory>,
    /// Notification providers
    providers: Arc<RwLock<Vec<Box<dyn NotificationProvider>>>>,
    /// Render targets
    render_targets: Arc<RwLock<Vec<Box<dyn NotificationRenderTarget>>>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<NotificationEvent>,
    /// Notification channels
    channels: Arc<RwLock<HashMap<String, NotificationChannel>>>,
    /// Active filters
    filters: Arc<RwLock<Vec<NotificationFilter>>>,
    /// Command channel
    command_tx: mpsc::Sender<NotificationCommand>,
    /// Grouped notifications
    groups: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
}

/// Internal commands for the notification manager
enum NotificationCommand {
    Create(Notification),
    Update(Notification),
    Dismiss(Uuid),
    DismissAll,
    ClearGroup(String),
    ProcessQueue,
    CheckExpiry,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(config: NotificationConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let (command_tx, mut command_rx) = mpsc::channel(1024);
        
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            active: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            history: Arc::new(NotificationHistory::new()),
            providers: Arc::new(RwLock::new(Vec::new())),
            render_targets: Arc::new(RwLock::new(Vec::new())),
            event_tx: event_tx.clone(),
            channels: Arc::new(RwLock::new(HashMap::new())),
            filters: Arc::new(RwLock::new(Vec::new())),
            command_tx,
            groups: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Spawn command processor
        let manager_clone = manager.clone_internals();
        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                if let Err(e) = manager_clone.process_command(cmd).await {
                    error!("Error processing notification command: {}", e);
                }
            }
        });
        
        // Spawn expiry checker
        let manager_clone = manager.clone_internals();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(e) = manager_clone.check_expired_notifications().await {
                    error!("Error checking expired notifications: {}", e);
                }
            }
        });
        
        manager
    }
    
    /// Clone internal references for spawned tasks
    fn clone_internals(&self) -> NotificationManagerInternal {
        NotificationManagerInternal {
            config: self.config.clone(),
            active: self.active.clone(),
            queue: self.queue.clone(),
            history: self.history.clone(),
            providers: self.providers.clone(),
            render_targets: self.render_targets.clone(),
            event_tx: self.event_tx.clone(),
            channels: self.channels.clone(),
            filters: self.filters.clone(),
            groups: self.groups.clone(),
        }
    }
    
    /// Add a notification provider
    pub async fn add_provider(&self, mut provider: Box<dyn NotificationProvider>) -> Result<()> {
        provider.initialize().await
            .context("Failed to initialize notification provider")?;
        
        self.providers.write().await.push(provider);
        Ok(())
    }
    
    /// Add a render target
    pub async fn add_render_target(&self, target: Box<dyn NotificationRenderTarget>) {
        self.render_targets.write().await.push(target);
    }
    
    /// Add a notification channel
    pub async fn add_channel(&self, channel: NotificationChannel) {
        self.channels.write().await.insert(channel.id.clone(), channel);
    }
    
    /// Add a notification filter
    pub async fn add_filter(&self, filter: NotificationFilter) {
        self.filters.write().await.push(filter);
    }
    
    /// Create a new notification
    pub async fn notify(&self, notification: Notification) -> Result<()> {
        self.command_tx.send(NotificationCommand::Create(notification)).await
            .context("Failed to send notification command")?;
        Ok(())
    }
    
    /// Update an existing notification
    pub async fn update(&self, notification: Notification) -> Result<()> {
        self.command_tx.send(NotificationCommand::Update(notification)).await
            .context("Failed to send update command")?;
        Ok(())
    }
    
    /// Dismiss a notification
    pub async fn dismiss(&self, id: Uuid) -> Result<()> {
        self.command_tx.send(NotificationCommand::Dismiss(id)).await
            .context("Failed to send dismiss command")?;
        Ok(())
    }
    
    /// Dismiss all notifications
    pub async fn dismiss_all(&self) -> Result<()> {
        self.command_tx.send(NotificationCommand::DismissAll).await
            .context("Failed to send dismiss all command")?;
        Ok(())
    }
    
    /// Clear a notification group
    pub async fn clear_group(&self, group: String) -> Result<()> {
        self.command_tx.send(NotificationCommand::ClearGroup(group)).await
            .context("Failed to send clear group command")?;
        Ok(())
    }
    
    /// Get active notifications
    pub async fn get_active(&self) -> Vec<Notification> {
        self.active.read().await.values().cloned().collect()
    }
    
    /// Get notification by ID
    pub async fn get_notification(&self, id: Uuid) -> Option<Notification> {
        self.active.read().await.get(&id).cloned()
    }
    
    /// Get notifications for a group
    pub async fn get_group(&self, group: &str) -> Vec<Notification> {
        let groups = self.groups.read().await;
        let active = self.active.read().await;
        
        groups.get(group)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| active.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Subscribe to notification events
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.event_tx.subscribe()
    }
    
    /// Get notification history
    pub fn history(&self) -> &NotificationHistory {
        &self.history
    }
    
    /// Update configuration
    pub async fn update_config(&self, config: NotificationConfig) {
        *self.config.write().await = config;
    }
    
    /// Get current configuration
    pub async fn config(&self) -> NotificationConfig {
        self.config.read().await.clone()
    }
}

/// Internal notification manager for spawned tasks
struct NotificationManagerInternal {
    config: Arc<RwLock<NotificationConfig>>,
    active: Arc<RwLock<HashMap<Uuid, Notification>>>,
    queue: Arc<RwLock<VecDeque<Notification>>>,
    history: Arc<NotificationHistory>,
    providers: Arc<RwLock<Vec<Box<dyn NotificationProvider>>>>,
    render_targets: Arc<RwLock<Vec<Box<dyn NotificationRenderTarget>>>>,
    event_tx: broadcast::Sender<NotificationEvent>,
    channels: Arc<RwLock<HashMap<String, NotificationChannel>>>,
    filters: Arc<RwLock<Vec<NotificationFilter>>>,
    groups: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
}

impl NotificationManagerInternal {
    /// Process a command
    async fn process_command(&self, command: NotificationCommand) -> Result<()> {
        match command {
            NotificationCommand::Create(notification) => {
                self.create_notification(notification).await?;
            }
            NotificationCommand::Update(notification) => {
                self.update_notification(notification).await?;
            }
            NotificationCommand::Dismiss(id) => {
                self.dismiss_notification(id).await?;
            }
            NotificationCommand::DismissAll => {
                self.dismiss_all_notifications().await?;
            }
            NotificationCommand::ClearGroup(group) => {
                self.clear_notification_group(group).await?;
            }
            NotificationCommand::ProcessQueue => {
                self.process_notification_queue().await?;
            }
            NotificationCommand::CheckExpiry => {
                self.check_expired_notifications().await?;
            }
        }
        Ok(())
    }
    
    /// Create a new notification
    async fn create_notification(&self, mut notification: Notification) -> Result<()> {
        let config = self.config.read().await;
        
        // Check do not disturb mode
        if config.do_not_disturb && notification.priority < NotificationPriority::Critical {
            if !config.priority_settings.critical_bypass_dnd {
                debug!("Notification blocked by do not disturb mode");
                return Ok(());
            }
        }
        
        // Apply filters
        let filters = self.filters.read().await;
        for filter in filters.iter() {
            if !filter.should_show(&notification) {
                debug!("Notification blocked by filter");
                return Ok(());
            }
        }
        
        // Set default timeout if not persistent
        if !notification.persistent && notification.expires_at.is_none() {
            let timeout = match notification.priority {
                NotificationPriority::High => {
                    let multiplier = config.priority_settings.high_timeout_multiplier;
                    Duration::from_secs_f32(config.default_timeout.as_secs_f32() * multiplier)
                }
                NotificationPriority::Low if config.priority_settings.low_auto_dismiss => {
                    Duration::from_secs(2)
                }
                _ => config.default_timeout,
            };
            notification = notification.expires_in(timeout);
        }
        
        // Check if we need to queue
        let active_count = self.active.read().await.len();
        if active_count >= config.max_visible {
            self.queue.write().await.push_back(notification.clone());
            info!("Notification queued: {}", notification.title);
            return Ok(());
        }
        
        // Add to active notifications
        self.active.write().await.insert(notification.id, notification.clone());
        
        // Add to group if specified
        if let Some(group) = &notification.group {
            self.groups.write().await
                .entry(group.clone())
                .or_insert_with(Vec::new)
                .push(notification.id);
        }
        
        // Send to providers
        let providers = self.providers.read().await;
        for provider in providers.iter() {
            if provider.is_available() {
                if let Err(e) = provider.send(&notification).await {
                    warn!("Provider {} failed to send notification: {}", provider.name(), e);
                }
            }
        }
        
        // Render notification
        self.render_notification(&notification).await?;
        
        // Add to history
        self.history.add(notification.clone()).await?;
        
        // Broadcast event
        let _ = self.event_tx.send(NotificationEvent::Created(notification));
        
        Ok(())
    }
    
    /// Update an existing notification
    async fn update_notification(&self, notification: Notification) -> Result<()> {
        let mut active = self.active.write().await;
        
        if let Some(existing) = active.get_mut(&notification.id) {
            *existing = notification.clone();
            drop(active);
            
            // Update in providers
            let providers = self.providers.read().await;
            for provider in providers.iter() {
                if provider.is_available() {
                    if let Err(e) = provider.update(&notification).await {
                        warn!("Provider {} failed to update notification: {}", provider.name(), e);
                    }
                }
            }
            
            // Update render
            let mut render_targets = self.render_targets.write().await;
            for target in render_targets.iter_mut() {
                if let Err(e) = target.update_render(&notification).await {
                    warn!("Failed to update render: {}", e);
                }
            }
            
            // Update history
            self.history.update(notification.clone()).await?;
            
            // Broadcast event
            let _ = self.event_tx.send(NotificationEvent::Updated(notification));
        }
        
        Ok(())
    }
    
    /// Dismiss a notification
    async fn dismiss_notification(&self, id: Uuid) -> Result<()> {
        if let Some(mut notification) = self.active.write().await.remove(&id) {
            notification.dismissed = true;
            
            // Remove from groups
            let mut groups = self.groups.write().await;
            for (_, ids) in groups.iter_mut() {
                ids.retain(|&nid| nid != id);
            }
            
            // Dismiss in providers
            let providers = self.providers.read().await;
            for provider in providers.iter() {
                if provider.is_available() {
                    if let Err(e) = provider.dismiss(id).await {
                        warn!("Provider {} failed to dismiss notification: {}", provider.name(), e);
                    }
                }
            }
            
            // Remove render
            self.remove_render(id).await?;
            
            // Update history
            self.history.dismiss(id).await?;
            
            // Broadcast event
            let _ = self.event_tx.send(NotificationEvent::Dismissed(id));
            
            // Process queue if there are waiting notifications
            if !self.queue.read().await.is_empty() {
                let _ = self.process_notification_queue().await;
            }
        }
        
        Ok(())
    }
    
    /// Dismiss all notifications
    async fn dismiss_all_notifications(&self) -> Result<()> {
        let ids: Vec<Uuid> = self.active.read().await.keys().cloned().collect();
        
        for id in ids {
            self.dismiss_notification(id).await?;
        }
        
        // Clear queue
        self.queue.write().await.clear();
        
        Ok(())
    }
    
    /// Clear a notification group
    async fn clear_notification_group(&self, group: String) -> Result<()> {
        let groups = self.groups.read().await;
        if let Some(ids) = groups.get(&group) {
            let ids = ids.clone();
            drop(groups);
            
            for id in ids {
                self.dismiss_notification(id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Process notification queue
    async fn process_notification_queue(&self) -> Result<()> {
        let config = self.config.read().await;
        let max_visible = config.max_visible;
        drop(config);
        
        while self.active.read().await.len() < max_visible {
            if let Some(notification) = self.queue.write().await.pop_front() {
                self.create_notification(notification).await?;
            } else {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Check for expired notifications
    async fn check_expired_notifications(&self) -> Result<()> {
        let expired: Vec<Uuid> = self.active.read().await
            .iter()
            .filter(|(_, n)| n.is_expired())
            .map(|(id, _)| *id)
            .collect();
        
        for id in expired {
            info!("Notification expired: {}", id);
            self.dismiss_notification(id).await?;
            let _ = self.event_tx.send(NotificationEvent::Expired(id));
        }
        
        Ok(())
    }
    
    /// Render a notification
    async fn render_notification(&self, notification: &Notification) -> Result<()> {
        let config = self.config.read().await;
        let position = config.position;
        let animations = config.animations.clone();
        drop(config);
        
        let mut render_targets = self.render_targets.write().await;
        for target in render_targets.iter_mut() {
            target.render(notification, position).await?;
            
            // Apply entrance animations
            if animations.slide_in {
                let direction = match position {
                    NotificationPosition::TopLeft | NotificationPosition::BottomLeft => SlideDirection::Left,
                    NotificationPosition::TopRight | NotificationPosition::BottomRight => SlideDirection::Right,
                    NotificationPosition::TopCenter => SlideDirection::Top,
                    NotificationPosition::BottomCenter => SlideDirection::Bottom,
                    _ => SlideDirection::Right,
                };
                
                target.animate(
                    notification.id,
                    NotificationAnimation::SlideIn {
                        duration: animations.duration,
                        direction,
                    }
                ).await?;
            }
            
            if animations.fade_in {
                target.animate(
                    notification.id,
                    NotificationAnimation::FadeIn {
                        duration: animations.duration,
                    }
                ).await?;
            }
        }
        
        Ok(())
    }
    
    /// Remove notification render
    async fn remove_render(&self, id: Uuid) -> Result<()> {
        let config = self.config.read().await;
        let animations = config.animations.clone();
        let position = config.position;
        drop(config);
        
        let mut render_targets = self.render_targets.write().await;
        for target in render_targets.iter_mut() {
            // Apply exit animations
            if animations.slide_in {
                let direction = match position {
                    NotificationPosition::TopLeft | NotificationPosition::BottomLeft => SlideDirection::Left,
                    NotificationPosition::TopRight | NotificationPosition::BottomRight => SlideDirection::Right,
                    NotificationPosition::TopCenter => SlideDirection::Top,
                    NotificationPosition::BottomCenter => SlideDirection::Bottom,
                    _ => SlideDirection::Right,
                };
                
                target.animate(
                    id,
                    NotificationAnimation::SlideOut {
                        duration: animations.duration,
                        direction,
                    }
                ).await?;
            }
            
            if animations.fade_in {
                target.animate(
                    id,
                    NotificationAnimation::FadeOut {
                        duration: animations.duration,
                    }
                ).await?;
            }
            
            // Wait for animation to complete
            tokio::time::sleep(animations.duration).await;
            
            // Remove render
            target.remove_render(id).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_notification_manager_creation() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config);
        
        assert_eq!(manager.get_active().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_notification_creation() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config);
        
        let notification = Notification::new(
            "Test".to_string(),
            "Test notification".to_string()
        );
        
        manager.notify(notification.clone()).await.unwrap();
        
        // Give time for async processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let active = manager.get_active().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Test");
    }
    
    #[tokio::test]
    async fn test_notification_dismissal() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config);
        
        let notification = Notification::new(
            "Test".to_string(),
            "Test notification".to_string()
        );
        let id = notification.id;
        
        manager.notify(notification).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(manager.get_active().await.len(), 1);
        
        manager.dismiss(id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(manager.get_active().await.len(), 0);
    }
}