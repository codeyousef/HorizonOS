//! Keyboard navigation system for graph desktop accessibility

use crate::{NodeAccessibilityInfo, AccessibleRole, AccessibilityEvent};
use horizonos_graph_engine::SceneId;
use std::collections::{HashMap, HashSet};
use anyhow::Result;

/// Keyboard navigation system for accessible graph navigation
#[derive(Debug)]
pub struct KeyboardNavigator {
    /// Current focus
    current_focus: Option<SceneId>,
    /// Navigation history
    navigation_history: Vec<SceneId>,
    /// Focus ring settings
    focus_ring: FocusRingSettings,
    /// Navigation mode
    navigation_mode: NavigationMode,
    /// Key bindings
    key_bindings: HashMap<KeyCombination, NavigationAction>,
    /// Spatial navigation enabled
    spatial_navigation: bool,
    /// Focus constraints
    focus_constraints: Vec<FocusConstraint>,
}

/// Focus ring visual settings
#[derive(Debug, Clone)]
pub struct FocusRingSettings {
    /// Ring width
    pub width: f32,
    /// Ring color
    pub color: [f32; 4],
    /// Ring style
    pub style: FocusRingStyle,
    /// Animation enabled
    pub animated: bool,
    /// High contrast mode
    pub high_contrast: bool,
}

/// Focus ring styles
#[derive(Debug, Clone)]
pub enum FocusRingStyle {
    /// Solid ring
    Solid,
    /// Dashed ring
    Dashed,
    /// Dotted ring
    Dotted,
    /// Glow effect
    Glow,
}

/// Navigation modes
#[derive(Debug, Clone, Copy)]
pub enum NavigationMode {
    /// Standard tab navigation
    Tab,
    /// Spatial navigation (arrow keys)
    Spatial,
    /// Semantic navigation (by role)
    Semantic,
    /// Graph-aware navigation
    Graph,
}

/// Key combination for bindings
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyCombination {
    /// Main key
    pub key: Key,
    /// Modifier keys
    pub modifiers: Vec<Modifier>,
}

/// Key codes
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Key {
    Tab,
    Arrow(ArrowDirection),
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Space,
    Escape,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Character(char),
}

/// Arrow directions
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Modifier keys
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Modifier {
    Shift,
    Ctrl,
    Alt,
    Meta,
}

/// Navigation actions
#[derive(Debug, Clone)]
pub enum NavigationAction {
    /// Move focus to next element
    NextElement,
    /// Move focus to previous element
    PreviousElement,
    /// Move focus in spatial direction
    SpatialMove(ArrowDirection),
    /// Move to first element
    FirstElement,
    /// Move to last element
    LastElement,
    /// Move to next of specific role
    NextRole(AccessibleRole),
    /// Move to previous of specific role
    PreviousRole(AccessibleRole),
    /// Activate current element
    Activate,
    /// Show context menu
    ContextMenu,
    /// Enter container
    EnterContainer,
    /// Exit container
    ExitContainer,
    /// Navigate to parent
    NavigateToParent,
    /// Navigate to child
    NavigateToChild,
    /// Start spatial exploration
    StartSpatialExploration,
    /// Stop spatial exploration
    StopSpatialExploration,
}

/// Focus constraints to limit navigation
#[derive(Debug, Clone)]
pub struct FocusConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Target nodes (if applicable)
    pub target_nodes: Option<HashSet<SceneId>>,
    /// Roles to constrain to
    pub roles: Option<HashSet<AccessibleRole>>,
}

/// Types of focus constraints
#[derive(Debug, Clone)]
pub enum ConstraintType {
    /// Only navigate within specific nodes
    ContainTo,
    /// Exclude specific nodes from navigation
    Exclude,
    /// Only navigate to specific roles
    RoleOnly,
    /// Skip specific roles
    SkipRoles,
}

/// Navigation suggestions for smart navigation
#[derive(Debug, Clone)]
pub struct NavigationSuggestion {
    /// Target node
    pub target_node: SceneId,
    /// Suggestion type
    pub suggestion_type: SuggestionType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Reason for suggestion
    pub reason: String,
}

/// Types of navigation suggestions
#[derive(Debug, Clone)]
pub enum SuggestionType {
    /// Related content
    Related,
    /// Frequently accessed
    Frequent,
    /// Spatial proximity
    Spatial,
    /// Semantic similarity
    Semantic,
    /// Workflow continuation
    Workflow,
}

impl KeyboardNavigator {
    /// Create a new keyboard navigator
    pub fn new() -> Self {
        let mut key_bindings = HashMap::new();
        
        // Set up default key bindings
        key_bindings.insert(
            KeyCombination {
                key: Key::Tab,
                modifiers: Vec::new(),
            },
            NavigationAction::NextElement,
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Tab,
                modifiers: vec![Modifier::Shift],
            },
            NavigationAction::PreviousElement,
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Arrow(ArrowDirection::Right),
                modifiers: Vec::new(),
            },
            NavigationAction::SpatialMove(ArrowDirection::Right),
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Arrow(ArrowDirection::Left),
                modifiers: Vec::new(),
            },
            NavigationAction::SpatialMove(ArrowDirection::Left),
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Arrow(ArrowDirection::Up),
                modifiers: Vec::new(),
            },
            NavigationAction::SpatialMove(ArrowDirection::Up),
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Arrow(ArrowDirection::Down),
                modifiers: Vec::new(),
            },
            NavigationAction::SpatialMove(ArrowDirection::Down),
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Home,
                modifiers: Vec::new(),
            },
            NavigationAction::FirstElement,
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::End,
                modifiers: Vec::new(),
            },
            NavigationAction::LastElement,
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Enter,
                modifiers: Vec::new(),
            },
            NavigationAction::Activate,
        );
        
        key_bindings.insert(
            KeyCombination {
                key: Key::Space,
                modifiers: Vec::new(),
            },
            NavigationAction::Activate,
        );

        Self {
            current_focus: None,
            navigation_history: Vec::new(),
            focus_ring: FocusRingSettings::default(),
            navigation_mode: NavigationMode::Tab,
            key_bindings,
            spatial_navigation: true,
            focus_constraints: Vec::new(),
        }
    }

    /// Handle key press event
    pub fn handle_key_press(
        &mut self,
        key: Key,
        modifiers: Vec<Modifier>,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Result<Option<NavigationAction>> {
        let key_combination = KeyCombination { key, modifiers };
        
        if let Some(action) = self.key_bindings.get(&key_combination) {
            self.execute_navigation_action(action.clone(), node_cache)?;
            return Ok(Some(action.clone()));
        }
        
        Ok(None)
    }

    /// Execute navigation action
    pub fn execute_navigation_action(
        &mut self,
        action: NavigationAction,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Result<()> {
        match action {
            NavigationAction::NextElement => {
                self.navigate_to_next_element(node_cache)?;
            }
            NavigationAction::PreviousElement => {
                self.navigate_to_previous_element(node_cache)?;
            }
            NavigationAction::SpatialMove(direction) => {
                self.navigate_spatially(direction, node_cache)?;
            }
            NavigationAction::FirstElement => {
                self.navigate_to_first_element(node_cache)?;
            }
            NavigationAction::LastElement => {
                self.navigate_to_last_element(node_cache)?;
            }
            NavigationAction::NextRole(role) => {
                self.navigate_to_next_role(role, node_cache)?;
            }
            NavigationAction::PreviousRole(role) => {
                self.navigate_to_previous_role(role, node_cache)?;
            }
            NavigationAction::Activate => {
                self.activate_current_element()?;
            }
            NavigationAction::ContextMenu => {
                self.show_context_menu()?;
            }
            NavigationAction::NavigateToParent => {
                self.navigate_to_parent(node_cache)?;
            }
            NavigationAction::NavigateToChild => {
                self.navigate_to_child(node_cache)?;
            }
            _ => {
                log::debug!("Navigation action not yet implemented: {:?}", action);
            }
        }
        
        Ok(())
    }

    /// Set focus to specific node
    pub fn set_focus(&mut self, node_id: Option<SceneId>) -> Result<()> {
        if let Some(old_focus) = self.current_focus {
            self.navigation_history.push(old_focus);
            if self.navigation_history.len() > 100 {
                self.navigation_history.remove(0);
            }
        }
        
        self.current_focus = node_id;
        
        if let Some(focus) = node_id {
            log::debug!("Keyboard focus set to node: {:?}", focus);
        } else {
            log::debug!("Keyboard focus cleared");
        }
        
        Ok(())
    }

    /// Get current focus
    pub fn get_focus(&self) -> Option<SceneId> {
        self.current_focus
    }

    /// Get navigation suggestions
    pub fn get_navigation_suggestions(
        &self,
        current_node: SceneId,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Vec<SceneId> {
        let mut suggestions = Vec::new();
        
        // Get spatially nearby nodes
        if let Some(current_info) = node_cache.get(&current_node) {
            let nearby_nodes = self.find_spatially_nearby_nodes(current_info, node_cache);
            suggestions.extend(nearby_nodes);
        }
        
        // Get semantically related nodes
        let related_nodes = self.find_semantically_related_nodes(current_node, node_cache);
        suggestions.extend(related_nodes);
        
        // Remove duplicates and current node
        suggestions.retain(|&id| id != current_node);
        suggestions.dedup();
        
        suggestions
    }

    /// Update focus ring settings
    pub fn update_focus_ring(&mut self, settings: FocusRingSettings) {
        self.focus_ring = settings;
    }

    /// Set navigation mode
    pub fn set_navigation_mode(&mut self, mode: NavigationMode) {
        self.navigation_mode = mode;
        log::debug!("Navigation mode set to: {:?}", mode);
    }

    /// Add focus constraint
    pub fn add_focus_constraint(&mut self, constraint: FocusConstraint) {
        self.focus_constraints.push(constraint);
    }

    /// Clear focus constraints
    pub fn clear_focus_constraints(&mut self) {
        self.focus_constraints.clear();
    }

    /// Navigate to next element
    fn navigate_to_next_element(&mut self, node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        let navigable_nodes = self.get_navigable_nodes(node_cache);
        
        if let Some(current) = self.current_focus {
            if let Some(current_index) = navigable_nodes.iter().position(|&id| id == current) {
                let next_index = (current_index + 1) % navigable_nodes.len();
                self.set_focus(Some(navigable_nodes[next_index]))?;
            }
        } else if !navigable_nodes.is_empty() {
            self.set_focus(Some(navigable_nodes[0]))?;
        }
        
        Ok(())
    }

    /// Navigate to previous element
    fn navigate_to_previous_element(&mut self, node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        let navigable_nodes = self.get_navigable_nodes(node_cache);
        
        if let Some(current) = self.current_focus {
            if let Some(current_index) = navigable_nodes.iter().position(|&id| id == current) {
                let prev_index = if current_index == 0 {
                    navigable_nodes.len() - 1
                } else {
                    current_index - 1
                };
                self.set_focus(Some(navigable_nodes[prev_index]))?;
            }
        } else if !navigable_nodes.is_empty() {
            self.set_focus(Some(navigable_nodes[navigable_nodes.len() - 1]))?;
        }
        
        Ok(())
    }

    /// Navigate spatially in a direction
    fn navigate_spatially(
        &mut self,
        direction: ArrowDirection,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Result<()> {
        if !self.spatial_navigation {
            return Ok(());
        }
        
        if let Some(current) = self.current_focus {
            if let Some(current_info) = node_cache.get(&current) {
                let next_node = self.find_spatial_neighbor(current_info, direction, node_cache);
                if let Some(next) = next_node {
                    self.set_focus(Some(next))?;
                }
            }
        }
        
        Ok(())
    }

    /// Navigate to first element
    fn navigate_to_first_element(&mut self, node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        let navigable_nodes = self.get_navigable_nodes(node_cache);
        if !navigable_nodes.is_empty() {
            self.set_focus(Some(navigable_nodes[0]))?;
        }
        Ok(())
    }

    /// Navigate to last element
    fn navigate_to_last_element(&mut self, node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        let navigable_nodes = self.get_navigable_nodes(node_cache);
        if !navigable_nodes.is_empty() {
            self.set_focus(Some(navigable_nodes[navigable_nodes.len() - 1]))?;
        }
        Ok(())
    }

    /// Navigate to next element of specific role
    fn navigate_to_next_role(
        &mut self,
        role: AccessibleRole,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Result<()> {
        let role_nodes: Vec<SceneId> = node_cache.iter()
            .filter(|(_, info)| std::mem::discriminant(&info.role) == std::mem::discriminant(&role))
            .map(|(&id, _)| id)
            .collect();
        
        if let Some(current) = self.current_focus {
            if let Some(current_index) = role_nodes.iter().position(|&id| id == current) {
                let next_index = (current_index + 1) % role_nodes.len();
                self.set_focus(Some(role_nodes[next_index]))?;
            } else if !role_nodes.is_empty() {
                self.set_focus(Some(role_nodes[0]))?;
            }
        } else if !role_nodes.is_empty() {
            self.set_focus(Some(role_nodes[0]))?;
        }
        
        Ok(())
    }

    /// Navigate to previous element of specific role
    fn navigate_to_previous_role(
        &mut self,
        role: AccessibleRole,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Result<()> {
        let role_nodes: Vec<SceneId> = node_cache.iter()
            .filter(|(_, info)| std::mem::discriminant(&info.role) == std::mem::discriminant(&role))
            .map(|(&id, _)| id)
            .collect();
        
        if let Some(current) = self.current_focus {
            if let Some(current_index) = role_nodes.iter().position(|&id| id == current) {
                let prev_index = if current_index == 0 {
                    role_nodes.len() - 1
                } else {
                    current_index - 1
                };
                self.set_focus(Some(role_nodes[prev_index]))?;
            } else if !role_nodes.is_empty() {
                self.set_focus(Some(role_nodes[role_nodes.len() - 1]))?;
            }
        } else if !role_nodes.is_empty() {
            self.set_focus(Some(role_nodes[role_nodes.len() - 1]))?;
        }
        
        Ok(())
    }

    /// Activate current element
    fn activate_current_element(&mut self) -> Result<()> {
        if let Some(current) = self.current_focus {
            log::debug!("Activating element: {:?}", current);
            // TODO: Send activation event
        }
        Ok(())
    }

    /// Show context menu for current element
    fn show_context_menu(&mut self) -> Result<()> {
        if let Some(current) = self.current_focus {
            log::debug!("Showing context menu for: {:?}", current);
            // TODO: Show context menu
        }
        Ok(())
    }

    /// Navigate to parent node
    fn navigate_to_parent(&mut self, _node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        // TODO: Implement parent navigation using graph relationships
        Ok(())
    }

    /// Navigate to child node
    fn navigate_to_child(&mut self, _node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Result<()> {
        // TODO: Implement child navigation using graph relationships
        Ok(())
    }

    /// Get list of navigable nodes
    fn get_navigable_nodes(&self, node_cache: &HashMap<SceneId, NodeAccessibilityInfo>) -> Vec<SceneId> {
        let mut nodes: Vec<SceneId> = node_cache.iter()
            .filter(|(_, info)| info.state.enabled && info.state.visible)
            .filter(|(id, info)| self.is_node_navigable(**id, info))
            .map(|(&id, _)| id)
            .collect();
        
        // Sort by position for consistent navigation order
        nodes.sort_by(|&a, &b| {
            let a_info = node_cache.get(&a).unwrap();
            let b_info = node_cache.get(&b).unwrap();
            
            // Sort by Y position first, then X position
            let y_cmp = a_info.bounds.y.partial_cmp(&b_info.bounds.y).unwrap();
            if y_cmp == std::cmp::Ordering::Equal {
                a_info.bounds.x.partial_cmp(&b_info.bounds.x).unwrap()
            } else {
                y_cmp
            }
        });
        
        nodes
    }

    /// Check if node is navigable based on constraints
    fn is_node_navigable(&self, node_id: SceneId, info: &NodeAccessibilityInfo) -> bool {
        for constraint in &self.focus_constraints {
            match &constraint.constraint_type {
                ConstraintType::ContainTo => {
                    if let Some(target_nodes) = &constraint.target_nodes {
                        if !target_nodes.contains(&node_id) {
                            return false;
                        }
                    }
                }
                ConstraintType::Exclude => {
                    if let Some(target_nodes) = &constraint.target_nodes {
                        if target_nodes.contains(&node_id) {
                            return false;
                        }
                    }
                }
                ConstraintType::RoleOnly => {
                    if let Some(roles) = &constraint.roles {
                        if !roles.iter().any(|role| std::mem::discriminant(role) == std::mem::discriminant(&info.role)) {
                            return false;
                        }
                    }
                }
                ConstraintType::SkipRoles => {
                    if let Some(roles) = &constraint.roles {
                        if roles.iter().any(|role| std::mem::discriminant(role) == std::mem::discriminant(&info.role)) {
                            return false;
                        }
                    }
                }
            }
        }
        
        true
    }

    /// Find spatial neighbor in given direction
    fn find_spatial_neighbor(
        &self,
        current_info: &NodeAccessibilityInfo,
        direction: ArrowDirection,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Option<SceneId> {
        let mut candidates = Vec::new();
        
        for (&id, info) in node_cache.iter() {
            if id == current_info.node_id || !info.state.enabled || !info.state.visible {
                continue;
            }
            
            let is_candidate = match direction {
                ArrowDirection::Right => info.bounds.x > current_info.bounds.x,
                ArrowDirection::Left => info.bounds.x < current_info.bounds.x,
                ArrowDirection::Down => info.bounds.y > current_info.bounds.y,
                ArrowDirection::Up => info.bounds.y < current_info.bounds.y,
            };
            
            if is_candidate {
                let distance = self.calculate_spatial_distance(current_info, info, &direction);
                candidates.push((id, distance));
            }
        }
        
        // Sort by distance and return closest
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        candidates.first().map(|(id, _)| *id)
    }

    /// Calculate spatial distance between nodes
    fn calculate_spatial_distance(
        &self,
        from: &NodeAccessibilityInfo,
        to: &NodeAccessibilityInfo,
        direction: &ArrowDirection,
    ) -> f32 {
        let dx = to.bounds.x - from.bounds.x;
        let dy = to.bounds.y - from.bounds.y;
        
        match direction {
            ArrowDirection::Right | ArrowDirection::Left => {
                dx.abs() + dy.abs() * 0.5 // Prefer horizontal alignment
            }
            ArrowDirection::Up | ArrowDirection::Down => {
                dy.abs() + dx.abs() * 0.5 // Prefer vertical alignment
            }
        }
    }

    /// Find spatially nearby nodes
    fn find_spatially_nearby_nodes(
        &self,
        current_info: &NodeAccessibilityInfo,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Vec<SceneId> {
        let mut nearby = Vec::new();
        
        for (&id, info) in node_cache.iter() {
            if id == current_info.node_id {
                continue;
            }
            
            let distance = ((info.bounds.x - current_info.bounds.x).powi(2) + 
                           (info.bounds.y - current_info.bounds.y).powi(2)).sqrt();
            
            if distance < 200.0 { // Within 200 pixels
                nearby.push(id);
            }
        }
        
        nearby
    }

    /// Find semantically related nodes
    fn find_semantically_related_nodes(
        &self,
        current_node: SceneId,
        node_cache: &HashMap<SceneId, NodeAccessibilityInfo>,
    ) -> Vec<SceneId> {
        let mut related = Vec::new();
        
        if let Some(current_info) = node_cache.get(&current_node) {
            for (&id, info) in node_cache.iter() {
                if id == current_node {
                    continue;
                }
                
                // Same role
                if std::mem::discriminant(&info.role) == std::mem::discriminant(&current_info.role) {
                    related.push(id);
                }
                
                // Name similarity
                if info.name.to_lowercase().contains(&current_info.name.to_lowercase()) ||
                   current_info.name.to_lowercase().contains(&info.name.to_lowercase()) {
                    related.push(id);
                }
            }
        }
        
        related
    }
}

impl Default for FocusRingSettings {
    fn default() -> Self {
        Self {
            width: 2.0,
            color: [0.0, 0.5, 1.0, 1.0], // Blue
            style: FocusRingStyle::Solid,
            animated: true,
            high_contrast: false,
        }
    }
}