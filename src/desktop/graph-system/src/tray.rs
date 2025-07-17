//! System tray integration for the graph desktop

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use futures_util::stream::StreamExt;
use crate::dbus::{EnhancedDBusManager, StatusNotifierWatcherProxy};

/// System tray manager
pub struct SystemTrayManager {
    /// Active tray items
    items: Arc<RwLock<HashMap<String, TrayItem>>>,
    /// Enhanced D-Bus manager
    dbus_manager: Arc<EnhancedDBusManager>,
    /// Event channel
    event_tx: mpsc::Sender<TrayEvent>,
    /// Tray host implementation
    host: TrayHost,
}

/// Individual tray item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayItem {
    /// Unique ID
    pub id: String,
    /// Application name
    pub app_name: String,
    /// Icon name or path
    pub icon: TrayIcon,
    /// Tooltip text
    pub tooltip: Option<String>,
    /// Menu items
    pub menu: Vec<MenuItem>,
    /// Status
    pub status: TrayItemStatus,
    /// Category
    pub category: TrayCategory,
    /// Associated node ID
    pub node_id: Option<u64>,
}

/// Tray icon representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrayIcon {
    /// Icon theme name
    Named(String),
    /// Pixmap data
    Pixmap {
        width: i32,
        height: i32,
        data: Vec<u8>,
    },
    /// File path
    Path(String),
}

/// Tray item status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrayItemStatus {
    Passive,
    Active,
    NeedsAttention,
}

/// Tray item category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

/// Menu item for tray
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    /// Item ID
    pub id: String,
    /// Label
    pub label: String,
    /// Enabled state
    pub enabled: bool,
    /// Checkable item
    pub checkable: bool,
    /// Checked state
    pub checked: bool,
    /// Icon
    pub icon: Option<String>,
    /// Submenu
    pub submenu: Vec<MenuItem>,
}

/// Tray events
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// Item added
    ItemAdded(TrayItem),
    /// Item removed
    ItemRemoved(String),
    /// Item updated
    ItemUpdated(TrayItem),
    /// Item activated (clicked)
    ItemActivated(String),
    /// Menu item selected
    MenuItemSelected { item_id: String, menu_id: String },
    /// Scroll event on item
    Scroll { item_id: String, delta: i32, orientation: ScrollOrientation },
}

/// Scroll orientation
#[derive(Debug, Clone, Copy)]
pub enum ScrollOrientation {
    Horizontal,
    Vertical,
}

/// System tray host implementation
struct TrayHost {
    /// Watcher for StatusNotifierItem registrations
    watcher: Option<StatusNotifierWatcherProxy<'static>>,
}

impl SystemTrayManager {
    /// Create new system tray manager
    pub async fn new() -> Result<Self> {
        let (event_tx, mut event_rx) = mpsc::channel(256);
        
        // Create enhanced D-Bus manager
        let mut dbus_manager = EnhancedDBusManager::new().await?;
        
        // Initialize StatusNotifierWatcher
        dbus_manager.init_status_notifier_watcher().await?;
        
        let dbus_manager = Arc::new(dbus_manager);
        
        let manager = Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            dbus_manager: dbus_manager.clone(),
            event_tx: event_tx.clone(),
            host: TrayHost { 
                watcher: dbus_manager.status_notifier_watcher().cloned(),
            },
        };
        
        // Initialize tray host
        manager.initialize_host().await?;
        
        // Spawn event handler
        let items = manager.items.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                Self::handle_event(&items, event).await;
            }
        });
        
        Ok(manager)
    }
    
    /// Initialize tray host
    async fn initialize_host(&self) -> Result<()> {
        // Register as StatusNotifierHost
        let host_name = "org.kde.StatusNotifierHost-graph-desktop";
        
        // Request name on session bus
        let connection = self.dbus_manager.session_connection();
        connection.request_name(host_name).await?;
        
        // Register with StatusNotifierWatcher
        self.dbus_manager.register_status_notifier_host(host_name).await?;
        
        // Set up signal handlers for item registration/removal
        self.setup_signal_handlers().await?;
        
        Ok(())
    }
    
    /// Set up signal handlers for tray item events
    async fn setup_signal_handlers(&self) -> Result<()> {
        // For now, we'll use a simplified approach
        // In a full implementation, we would set up proper signal handlers
        // for StatusNotifierItem registration/unregistration
        
        log::info!("Signal handlers set up for tray events");
        Ok(())
    }
    
    /// Register a tray item
    pub async fn register_item(&self, item: TrayItem) -> Result<()> {
        let id = item.id.clone();
        self.items.write().unwrap().insert(id.clone(), item.clone());
        
        self.event_tx.send(TrayEvent::ItemAdded(item)).await
            .context("Failed to send item added event")?;
        
        Ok(())
    }
    
    /// Remove a tray item
    pub async fn remove_item(&self, id: &str) -> Result<()> {
        if self.items.write().unwrap().remove(id).is_some() {
            self.event_tx.send(TrayEvent::ItemRemoved(id.to_string())).await
                .context("Failed to send item removed event")?;
        }
        Ok(())
    }
    
    /// Update a tray item
    pub async fn update_item(&self, item: TrayItem) -> Result<()> {
        let id = item.id.clone();
        self.items.write().unwrap().insert(id, item.clone());
        
        self.event_tx.send(TrayEvent::ItemUpdated(item)).await
            .context("Failed to send item updated event")?;
        
        Ok(())
    }
    
    /// Get all tray items
    pub fn get_items(&self) -> Vec<TrayItem> {
        self.items.read().unwrap().values().cloned().collect()
    }
    
    /// Get item by ID
    pub fn get_item(&self, id: &str) -> Option<TrayItem> {
        self.items.read().unwrap().get(id).cloned()
    }
    
    /// Handle click on tray item
    pub async fn activate_item(&self, id: &str) -> Result<()> {
        if self.items.read().unwrap().contains_key(id) {
            self.event_tx.send(TrayEvent::ItemActivated(id.to_string())).await
                .context("Failed to send item activated event")?;
        }
        Ok(())
    }
    
    /// Handle menu item selection
    pub async fn select_menu_item(&self, item_id: &str, menu_id: &str) -> Result<()> {
        self.event_tx.send(TrayEvent::MenuItemSelected {
            item_id: item_id.to_string(),
            menu_id: menu_id.to_string(),
        }).await
        .context("Failed to send menu item selected event")?;
        
        Ok(())
    }
    
    /// Handle scroll on tray item
    pub async fn scroll_item(&self, item_id: &str, delta: i32, orientation: ScrollOrientation) -> Result<()> {
        self.event_tx.send(TrayEvent::Scroll {
            item_id: item_id.to_string(),
            delta,
            orientation,
        }).await
        .context("Failed to send scroll event")?;
        
        Ok(())
    }
    
    /// Handle tray event
    async fn handle_event(_items: &Arc<RwLock<HashMap<String, TrayItem>>>, event: TrayEvent) {
        match event {
            TrayEvent::ItemAdded(item) => {
                log::info!("Tray item added: {} ({})", item.app_name, item.id);
            }
            TrayEvent::ItemRemoved(id) => {
                log::info!("Tray item removed: {}", id);
            }
            TrayEvent::ItemUpdated(item) => {
                log::debug!("Tray item updated: {}", item.id);
            }
            TrayEvent::ItemActivated(id) => {
                log::debug!("Tray item activated: {}", id);
                // TODO: Handle activation (e.g., show app window, open menu)
            }
            TrayEvent::MenuItemSelected { item_id, menu_id } => {
                log::debug!("Menu item selected: {} on {}", menu_id, item_id);
                // TODO: Forward to application
            }
            TrayEvent::Scroll { item_id, delta, orientation } => {
                log::debug!("Scroll on {}: {} {:?}", item_id, delta, orientation);
                // TODO: Forward to application
            }
        }
    }
}


/// StatusNotifierItem D-Bus interface implementation
pub struct StatusNotifierItemImpl {
    item: TrayItem,
    manager: Arc<SystemTrayManager>,
}

impl StatusNotifierItemImpl {
    /// Create new StatusNotifierItem implementation
    pub fn new(item: TrayItem, manager: Arc<SystemTrayManager>) -> Self {
        Self { item, manager }
    }
    
    // TODO: Implement full StatusNotifierItem D-Bus interface
    // This includes properties like Category, Id, Title, Status, IconName, etc.
    // And methods like Activate, SecondaryActivate, Scroll, ContextMenu
}

/// Graph-specific tray integration
pub struct GraphTrayIntegration {
    /// Tray manager
    manager: Arc<SystemTrayManager>,
    /// Node associations
    node_associations: Arc<RwLock<HashMap<String, u64>>>,
}

impl GraphTrayIntegration {
    /// Create new graph tray integration
    pub fn new(manager: Arc<SystemTrayManager>) -> Self {
        Self {
            manager,
            node_associations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Associate tray item with graph node
    pub fn associate_with_node(&self, item_id: &str, node_id: u64) {
        self.node_associations.write().unwrap().insert(item_id.to_string(), node_id);
    }
    
    /// Get node for tray item
    pub fn get_node_for_item(&self, item_id: &str) -> Option<u64> {
        self.node_associations.read().unwrap().get(item_id).copied()
    }
    
    /// Create tray item from application node
    pub fn create_item_from_app_node(
        &self,
        node_id: u64,
        app_name: &str,
        icon: &str,
    ) -> TrayItem {
        let item = TrayItem {
            id: format!("app-{}", node_id),
            app_name: app_name.to_string(),
            icon: TrayIcon::Named(icon.to_string()),
            tooltip: Some(format!("{} (Node {})", app_name, node_id)),
            menu: vec![
                MenuItem {
                    id: "show".to_string(),
                    label: "Show Window".to_string(),
                    enabled: true,
                    checkable: false,
                    checked: false,
                    icon: Some("window".to_string()),
                    submenu: vec![],
                },
                MenuItem {
                    id: "quit".to_string(),
                    label: "Quit".to_string(),
                    enabled: true,
                    checkable: false,
                    checked: false,
                    icon: Some("window-close".to_string()),
                    submenu: vec![],
                },
            ],
            status: TrayItemStatus::Active,
            category: TrayCategory::ApplicationStatus,
            node_id: Some(node_id),
        };
        
        self.associate_with_node(&item.id, node_id);
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tray_item_creation() {
        let item = TrayItem {
            id: "test-app".to_string(),
            app_name: "Test Application".to_string(),
            icon: TrayIcon::Named("test-icon".to_string()),
            tooltip: Some("Test tooltip".to_string()),
            menu: vec![],
            status: TrayItemStatus::Active,
            category: TrayCategory::ApplicationStatus,
            node_id: None,
        };
        
        assert_eq!(item.app_name, "Test Application");
        assert_eq!(item.status, TrayItemStatus::Active);
    }
    
    #[test]
    fn test_graph_tray_integration() {
        // This would need a mock SystemTrayManager for proper testing
        // let manager = Arc::new(SystemTrayManager::new().await.unwrap());
        // let integration = GraphTrayIntegration::new(manager);
        
        // let item = integration.create_item_from_app_node(123, "Test App", "test-icon");
        // assert_eq!(item.node_id, Some(123));
        // assert_eq!(integration.get_node_for_item(&item.id), Some(123));
    }
}