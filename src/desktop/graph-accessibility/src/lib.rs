//! Accessibility framework for HorizonOS graph desktop
//! 
//! This module provides comprehensive accessibility support for the graph-based desktop,
//! including AT-SPI integration, screen reader support, keyboard navigation, and
//! alternative interaction modalities for users with disabilities.

pub mod screen_reader;
pub mod keyboard_nav;
pub mod voice_commands;
pub mod magnification;
pub mod contrast;
pub mod spatial_audio;
pub mod at_spi;

use horizonos_graph_engine::{GraphEngine, SceneId, Scene};
use horizonos_graph_nodes::GraphNode;
use std::collections::HashMap;
use anyhow::Result;

/// Main accessibility manager
#[derive(Debug)]
pub struct AccessibilityManager {
    /// Screen reader interface
    pub screen_reader: screen_reader::ScreenReaderInterface,
    /// Keyboard navigation system
    pub keyboard_nav: keyboard_nav::KeyboardNavigator,
    /// Voice command recognition
    pub voice_commands: voice_commands::VoiceCommandSystem,
    /// Screen magnification
    pub magnification: magnification::MagnificationManager,
    /// High contrast manager
    pub contrast: contrast::ContrastManager,
    /// Spatial audio system
    pub spatial_audio: spatial_audio::SpatialAudioManager,
    /// AT-SPI interface
    pub at_spi: at_spi::AtSpiInterface,
    /// Accessibility settings
    pub settings: AccessibilitySettings,
    /// Node accessibility cache
    node_cache: HashMap<SceneId, NodeAccessibilityInfo>,
}

/// Accessibility settings configuration
#[derive(Debug, Clone)]
pub struct AccessibilitySettings {
    /// Screen reader enabled
    pub screen_reader_enabled: bool,
    /// Voice commands enabled
    pub voice_commands_enabled: bool,
    /// Magnification enabled
    pub magnification_enabled: bool,
    /// High contrast enabled
    pub high_contrast_enabled: bool,
    /// Spatial audio enabled
    pub spatial_audio_enabled: bool,
    /// Keyboard navigation only mode
    pub keyboard_only_mode: bool,
    /// Reduced motion preference
    pub reduced_motion: bool,
    /// Text scaling factor
    pub text_scale: f32,
    /// Focus ring visibility
    pub focus_ring_enhanced: bool,
    /// Audio feedback enabled
    pub audio_feedback: bool,
    /// Haptic feedback enabled
    pub haptic_feedback: bool,
    /// Color blind friendly mode
    pub color_blind_mode: ColorBlindMode,
    /// Speech rate (words per minute)
    pub speech_rate: u32,
    /// Speech volume (0.0 to 1.0)
    pub speech_volume: f32,
}

/// Color blind accessibility modes
#[derive(Debug, Clone, Copy)]
pub enum ColorBlindMode {
    None,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Monochrome,
}

/// Accessibility information for a graph node
#[derive(Debug, Clone)]
pub struct NodeAccessibilityInfo {
    /// Node ID
    pub node_id: SceneId,
    /// Accessible name
    pub name: String,
    /// Accessible description
    pub description: Option<String>,
    /// Accessible role
    pub role: AccessibleRole,
    /// Current state
    pub state: AccessibleState,
    /// Actions available for this node
    pub actions: Vec<AccessibleAction>,
    /// Relationships to other nodes
    pub relationships: Vec<AccessibleRelationship>,
    /// Bounding box for screen readers
    pub bounds: AccessibleBounds,
    /// Text content if applicable
    pub text_content: Option<String>,
    /// Value if applicable (for sliders, progress bars)
    pub value: Option<AccessibleValue>,
}

/// Accessible roles following ARIA specifications
#[derive(Debug, Clone)]
pub enum AccessibleRole {
    /// Application window
    Application,
    /// Dialog box
    Dialog,
    /// Button
    Button,
    /// Link
    Link,
    /// Text input
    TextInput,
    /// Label
    Label,
    /// Menu
    Menu,
    /// Menu item
    MenuItem,
    /// List
    List,
    /// List item
    ListItem,
    /// Tab
    Tab,
    /// Tab panel
    TabPanel,
    /// Tree
    Tree,
    /// Tree item
    TreeItem,
    /// Graph container
    Graph,
    /// Graph node
    GraphNode,
    /// Graph edge
    GraphEdge,
    /// Custom role
    Custom(String),
    /// Generic object for complex nodes
    GenericObject,
    /// Check box control
    CheckBox,
    /// Document role
    Document,
}

/// Accessible states
#[derive(Debug, Clone, Default)]
pub struct AccessibleState {
    /// Focused
    pub focused: bool,
    /// Selected
    pub selected: bool,
    /// Expanded (for tree items)
    pub expanded: Option<bool>,
    /// Enabled/disabled
    pub enabled: bool,
    /// Visible
    pub visible: bool,
    /// Checked (for checkboxes)
    pub checked: Option<bool>,
    /// Pressed (for buttons)
    pub pressed: bool,
    /// Busy/loading
    pub busy: bool,
}

/// Available actions on accessible objects
#[derive(Debug, Clone)]
pub enum AccessibleAction {
    /// Activate/click
    Activate,
    /// Focus
    Focus,
    /// Select
    Select,
    /// Expand/collapse
    Toggle,
    /// Show context menu
    ShowContextMenu,
    /// Navigate to
    NavigateTo,
    /// Open
    Open,
    /// Close
    Close,
    /// Custom action
    Custom(String),
}

/// Relationships between accessible objects
#[derive(Debug, Clone)]
pub struct AccessibleRelationship {
    /// Relationship type
    pub relation_type: RelationType,
    /// Target node ID
    pub target: SceneId,
}

/// Types of accessible relationships
#[derive(Debug, Clone)]
pub enum RelationType {
    /// Parent-child
    ParentChild,
    /// Sibling
    Sibling,
    /// Label for
    LabelFor,
    /// Labeled by
    LabeledBy,
    /// Controlled by
    ControlledBy,
    /// Controls
    Controls,
    /// Flows to
    FlowsTo,
    /// Flows from
    FlowsFrom,
    /// Member of
    MemberOf,
    /// Owns
    Owns,
}

/// Accessible bounds information
#[derive(Debug, Clone)]
pub struct AccessibleBounds {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

/// Accessible value for form controls
#[derive(Debug, Clone)]
pub struct AccessibleValue {
    /// Current value
    pub current: f64,
    /// Minimum value
    pub minimum: Option<f64>,
    /// Maximum value
    pub maximum: Option<f64>,
    /// Text representation
    pub text: Option<String>,
}

/// Events that can be fired for accessibility
#[derive(Debug, Clone)]
pub enum AccessibilityEvent {
    /// Focus changed
    FocusChanged {
        old_focus: Option<SceneId>,
        new_focus: Option<SceneId>,
    },
    /// Selection changed
    SelectionChanged {
        selected_nodes: Vec<SceneId>,
    },
    /// Node state changed
    StateChanged {
        node_id: SceneId,
        old_state: AccessibleState,
        new_state: AccessibleState,
    },
    /// Node structure changed
    StructureChanged {
        parent: Option<SceneId>,
        added: Vec<SceneId>,
        removed: Vec<SceneId>,
    },
    /// Text changed
    TextChanged {
        node_id: SceneId,
        old_text: String,
        new_text: String,
    },
    /// Value changed
    ValueChanged {
        node_id: SceneId,
        old_value: Option<AccessibleValue>,
        new_value: Option<AccessibleValue>,
    },
}

impl AccessibilityManager {
    /// Create a new accessibility manager
    pub fn new(settings: AccessibilitySettings) -> Result<Self> {
        Ok(Self {
            screen_reader: screen_reader::ScreenReaderInterface::new()?,
            keyboard_nav: keyboard_nav::KeyboardNavigator::new(),
            voice_commands: voice_commands::VoiceCommandSystem::new()?,
            magnification: magnification::MagnificationManager::new(),
            contrast: contrast::ContrastManager::new(),
            spatial_audio: spatial_audio::SpatialAudioManager::new()?,
            at_spi: at_spi::AtSpiInterface::new()?,
            settings,
            node_cache: HashMap::new(),
        })
    }

    /// Update accessibility information for a node
    pub fn update_node_accessibility(
        &mut self,
        node_id: SceneId,
        node: &dyn GraphNode,
        bounds: AccessibleBounds,
    ) -> Result<()> {
        let accessibility_info = self.create_accessibility_info(node_id, node, bounds)?;
        
        // Check if this is a new node or if information changed
        let needs_update = self.node_cache.get(&node_id)
            .map(|cached| !self.accessibility_info_equal(cached, &accessibility_info))
            .unwrap_or(true);

        if needs_update {
            // Update AT-SPI
            self.at_spi.update_object(&accessibility_info)?;
            
            // Update screen reader
            if self.settings.screen_reader_enabled {
                self.screen_reader.update_object(&accessibility_info)?;
            }
            
            // Cache the new information
            self.node_cache.insert(node_id, accessibility_info);
        }

        Ok(())
    }

    /// Remove accessibility information for a node
    pub fn remove_node_accessibility(&mut self, node_id: SceneId) -> Result<()> {
        if let Some(info) = self.node_cache.remove(&node_id) {
            // Remove from AT-SPI
            self.at_spi.remove_object(node_id)?;
            
            // Update screen reader
            if self.settings.screen_reader_enabled {
                self.screen_reader.remove_object(node_id)?;
            }
        }
        Ok(())
    }

    /// Handle accessibility events
    pub fn handle_event(&mut self, event: AccessibilityEvent) -> Result<()> {
        // Fire AT-SPI events
        self.at_spi.fire_event(&event)?;
        
        // Update screen reader
        if self.settings.screen_reader_enabled {
            self.screen_reader.handle_event(&event)?;
        }
        
        // Update spatial audio
        if self.settings.spatial_audio_enabled {
            self.spatial_audio.handle_event(&event)?;
        }
        
        // Handle voice feedback
        if self.settings.audio_feedback {
            self.provide_audio_feedback(&event)?;
        }

        Ok(())
    }

    /// Get accessible node information
    pub fn get_node_info(&self, node_id: SceneId) -> Option<&NodeAccessibilityInfo> {
        self.node_cache.get(&node_id)
    }

    /// Find nodes by accessible name
    pub fn find_nodes_by_name(&self, name: &str) -> Vec<SceneId> {
        self.node_cache
            .iter()
            .filter(|(_, info)| info.name.contains(name))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get navigation suggestions for keyboard navigation
    pub fn get_navigation_suggestions(&self, current_node: SceneId) -> Vec<SceneId> {
        self.keyboard_nav.get_navigation_suggestions(current_node, &self.node_cache)
    }

    /// Update accessibility settings
    pub fn update_settings(&mut self, settings: AccessibilitySettings) -> Result<()> {
        let old_settings = self.settings.clone();
        self.settings = settings;

        // Handle setting changes
        if old_settings.screen_reader_enabled != self.settings.screen_reader_enabled {
            if self.settings.screen_reader_enabled {
                self.screen_reader.enable()?;
            } else {
                self.screen_reader.disable()?;
            }
        }

        if old_settings.voice_commands_enabled != self.settings.voice_commands_enabled {
            if self.settings.voice_commands_enabled {
                self.voice_commands.enable()?;
            } else {
                self.voice_commands.disable()?;
            }
        }

        // Update other subsystems as needed
        self.magnification.update_settings(&self.settings)?;
        self.contrast.update_settings(&self.settings)?;
        self.spatial_audio.update_settings(&self.settings)?;

        Ok(())
    }

    /// Create accessibility information from a graph node
    fn create_accessibility_info(
        &self,
        node_id: SceneId,
        node: &dyn GraphNode,
        bounds: AccessibleBounds,
    ) -> Result<NodeAccessibilityInfo> {
        let node_type = node.node_type();
        let metadata = node.metadata();
        
        // Determine accessible role based on node type
        let role = match node_type {
            horizonos_graph_nodes::NodeType::Application { .. } => AccessibleRole::Application,
            horizonos_graph_nodes::NodeType::File { .. } => AccessibleRole::Document,
            horizonos_graph_nodes::NodeType::Person { .. } => AccessibleRole::ListItem,
            horizonos_graph_nodes::NodeType::Task { .. } => AccessibleRole::CheckBox,
            horizonos_graph_nodes::NodeType::Device { .. } => AccessibleRole::ListItem,
            horizonos_graph_nodes::NodeType::AIAgent { .. } => AccessibleRole::GenericObject,
            horizonos_graph_nodes::NodeType::System { .. } => AccessibleRole::ListItem,
            horizonos_graph_nodes::NodeType::URL { .. } => AccessibleRole::Link,
            horizonos_graph_nodes::NodeType::Automation { .. } => AccessibleRole::ListItem,
            horizonos_graph_nodes::NodeType::Setting { .. } => AccessibleRole::CheckBox,
            horizonos_graph_nodes::NodeType::ConfigGroup { .. } => AccessibleRole::TreeItem,
            horizonos_graph_nodes::NodeType::Concept { .. } => AccessibleRole::GenericObject,
        };

        // Create name from node type and metadata
        let name = metadata.description
            .clone()
            .unwrap_or_else(|| format!("{:?}", node_type));

        // Create accessible actions based on node type
        let actions = vec![
            AccessibleAction::Activate,
            AccessibleAction::Focus,
            AccessibleAction::Select,
            AccessibleAction::ShowContextMenu,
        ];

        Ok(NodeAccessibilityInfo {
            node_id,
            name,
            description: metadata.description.clone(),
            role,
            state: AccessibleState {
                enabled: true,
                visible: true,
                ..Default::default()
            },
            actions,
            relationships: Vec::new(), // TODO: Calculate relationships
            bounds,
            text_content: None,
            value: None,
        })
    }

    /// Check if accessibility information has changed
    fn accessibility_info_equal(&self, a: &NodeAccessibilityInfo, b: &NodeAccessibilityInfo) -> bool {
        a.name == b.name &&
        a.description == b.description &&
        a.state.focused == b.state.focused &&
        a.state.selected == b.state.selected &&
        a.state.enabled == b.state.enabled &&
        a.state.visible == b.state.visible
    }

    /// Provide audio feedback for events
    fn provide_audio_feedback(&mut self, event: &AccessibilityEvent) -> Result<()> {
        match event {
            AccessibilityEvent::FocusChanged { new_focus, .. } => {
                if let Some(node_id) = new_focus {
                    if let Some(info) = self.node_cache.get(node_id) {
                        // Provide spatial audio feedback
                        self.spatial_audio.play_focus_sound(&info.bounds)?;
                    }
                }
            }
            AccessibilityEvent::SelectionChanged { selected_nodes } => {
                if !selected_nodes.is_empty() {
                    self.spatial_audio.play_selection_sound()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            screen_reader_enabled: false,
            voice_commands_enabled: false,
            magnification_enabled: false,
            high_contrast_enabled: false,
            spatial_audio_enabled: false,
            keyboard_only_mode: false,
            reduced_motion: false,
            text_scale: 1.0,
            focus_ring_enhanced: false,
            audio_feedback: false,
            haptic_feedback: false,
            color_blind_mode: ColorBlindMode::None,
            speech_rate: 200,
            speech_volume: 0.8,
        }
    }
}

/// Error types for accessibility system
#[derive(Debug, thiserror::Error)]
pub enum AccessibilityError {
    #[error("AT-SPI error: {0}")]
    AtSpi(String),
    
    #[error("Screen reader error: {0}")]
    ScreenReader(String),
    
    #[error("Voice command error: {0}")]
    VoiceCommand(String),
    
    #[error("Audio system error: {0}")]
    Audio(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("System error: {0}")]
    System(#[from] std::io::Error),
}