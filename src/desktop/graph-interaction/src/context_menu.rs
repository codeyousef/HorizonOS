//! Context menu system for node interactions

use horizonos_graph_engine::SceneId;
use std::collections::HashMap;

/// Manages context menus for nodes
pub struct ContextMenuManager {
    /// Active context menu
    active_menu: Option<ContextMenu>,
    /// Registered menu items by node type
    menu_items: HashMap<String, Vec<MenuItem>>,
}

/// A context menu instance
#[derive(Clone)]
pub struct ContextMenu {
    pub node_id: SceneId,
    pub position: (f32, f32),
    pub items: Vec<MenuItem>,
    pub visible: bool,
}

/// A menu item
#[derive(Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub separator: bool,
    pub submenu: Option<Vec<MenuItem>>,
}

impl ContextMenuManager {
    /// Create a new context menu manager
    pub fn new() -> Self {
        Self {
            active_menu: None,
            menu_items: HashMap::new(),
        }
    }
    
    /// Show context menu for a node
    pub fn show_for_node(&mut self, node_id: SceneId, position: (f32, f32)) {
        // Get default menu items
        let items = self.get_default_menu_items();
        
        self.active_menu = Some(ContextMenu {
            node_id,
            position,
            items,
            visible: true,
        });
    }
    
    /// Hide the active context menu
    pub fn hide(&mut self) {
        if let Some(menu) = &mut self.active_menu {
            menu.visible = false;
        }
    }
    
    /// Clear the active context menu
    pub fn clear(&mut self) {
        self.active_menu = None;
    }
    
    /// Get the active context menu
    pub fn get_active(&self) -> Option<&ContextMenu> {
        self.active_menu.as_ref().filter(|m| m.visible)
    }
    
    /// Register menu items for a node type
    pub fn register_items(&mut self, node_type: String, items: Vec<MenuItem>) {
        self.menu_items.insert(node_type, items);
    }
    
    /// Handle menu item click
    pub fn handle_item_click(&mut self, item_id: &str) -> Option<(SceneId, String)> {
        if let Some(menu) = &self.active_menu {
            Some((menu.node_id, item_id.to_string()))
        } else {
            None
        }
    }
    
    /// Get default menu items
    fn get_default_menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "open".to_string(),
                label: "Open".to_string(),
                icon: Some("folder-open".to_string()),
                shortcut: Some("Enter".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "edit".to_string(),
                label: "Edit".to_string(),
                icon: Some("edit".to_string()),
                shortcut: Some("F2".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "".to_string(),
                label: "".to_string(),
                icon: None,
                shortcut: None,
                enabled: false,
                separator: true,
                submenu: None,
            },
            MenuItem {
                id: "cut".to_string(),
                label: "Cut".to_string(),
                icon: Some("scissors".to_string()),
                shortcut: Some("Ctrl+X".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "copy".to_string(),
                label: "Copy".to_string(),
                icon: Some("copy".to_string()),
                shortcut: Some("Ctrl+C".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "paste".to_string(),
                label: "Paste".to_string(),
                icon: Some("clipboard".to_string()),
                shortcut: Some("Ctrl+V".to_string()),
                enabled: false,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "".to_string(),
                label: "".to_string(),
                icon: None,
                shortcut: None,
                enabled: false,
                separator: true,
                submenu: None,
            },
            MenuItem {
                id: "delete".to_string(),
                label: "Delete".to_string(),
                icon: Some("trash".to_string()),
                shortcut: Some("Delete".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
            MenuItem {
                id: "".to_string(),
                label: "".to_string(),
                icon: None,
                shortcut: None,
                enabled: false,
                separator: true,
                submenu: None,
            },
            MenuItem {
                id: "properties".to_string(),
                label: "Properties".to_string(),
                icon: Some("info".to_string()),
                shortcut: Some("Alt+Enter".to_string()),
                enabled: true,
                separator: false,
                submenu: None,
            },
        ]
    }
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            shortcut: None,
            enabled: true,
            separator: false,
            submenu: None,
        }
    }
    
    /// Create a separator
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            icon: None,
            shortcut: None,
            enabled: false,
            separator: true,
            submenu: None,
        }
    }
    
    /// Set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    
    /// Set shortcut
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
    
    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Set submenu
    pub fn with_submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }
}