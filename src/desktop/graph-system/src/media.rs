//! Media control integration for the graph desktop

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::dbus::{EnhancedDBusManager, MediaAction};

/// Media control manager using MPRIS2
pub struct MediaManager {
    /// Active media players
    players: Arc<RwLock<HashMap<String, MediaPlayer>>>,
    /// Current active player
    active_player: Arc<RwLock<Option<String>>>,
    /// Media event channel
    event_tx: mpsc::Sender<MediaEvent>,
    /// Volume control
    volume_control: Arc<RwLock<VolumeControl>>,
    /// Media graph integration
    graph_integration: Arc<RwLock<MediaGraphIntegration>>,
    /// Enhanced D-Bus manager
    dbus_manager: Arc<EnhancedDBusManager>,
}

/// Individual media player information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPlayer {
    /// Player ID (D-Bus name)
    pub id: String,
    /// Player name
    pub name: String,
    /// Current playback status
    pub status: PlaybackStatus,
    /// Current track metadata
    pub metadata: Option<TrackMetadata>,
    /// Volume (0.0 - 1.0)
    pub volume: f32,
    /// Position in current track
    pub position: Duration,
    /// Can control playback
    pub can_control: bool,
    /// Can go to next/previous
    pub can_go_next: bool,
    pub can_go_previous: bool,
    /// Can seek
    pub can_seek: bool,
    /// Associated graph node
    pub node_id: Option<u64>,
}

/// Playback status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

/// Track metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    /// Track ID
    pub track_id: Option<String>,
    /// Track title
    pub title: String,
    /// Artist(s)
    pub artists: Vec<String>,
    /// Album
    pub album: Option<String>,
    /// Album artist
    pub album_artist: Option<String>,
    /// Track length
    pub length: Option<Duration>,
    /// Artwork URL
    pub art_url: Option<String>,
    /// Track URL
    pub url: Option<String>,
}

/// Volume control
#[derive(Debug, Clone)]
pub struct VolumeControl {
    /// Master volume (0.0 - 1.0)
    pub master_volume: f32,
    /// Is muted
    pub is_muted: bool,
    /// Application volumes
    pub app_volumes: HashMap<String, f32>,
    /// Input devices
    pub input_devices: Vec<AudioDevice>,
    /// Output devices
    pub output_devices: Vec<AudioDevice>,
    /// Active output device
    pub active_output: Option<String>,
    /// Active input device
    pub active_input: Option<String>,
}

/// Audio device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Device description
    pub description: Option<String>,
    /// Device type
    pub device_type: AudioDeviceType,
    /// Is default device
    pub is_default: bool,
    /// Current volume
    pub volume: f32,
    /// Is muted
    pub is_muted: bool,
}

/// Audio device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioDeviceType {
    Speakers,
    Headphones,
    Microphone,
    LineIn,
    LineOut,
    HDMI,
    Bluetooth,
    USB,
    Other,
}

/// Media events
#[derive(Debug, Clone)]
pub enum MediaEvent {
    /// Player added
    PlayerAdded(MediaPlayer),
    /// Player removed
    PlayerRemoved(String),
    /// Playback status changed
    StatusChanged { player_id: String, status: PlaybackStatus },
    /// Track changed
    TrackChanged { player_id: String, metadata: TrackMetadata },
    /// Volume changed
    VolumeChanged { player_id: String, volume: f32 },
    /// Position changed
    PositionChanged { player_id: String, position: Duration },
    /// Master volume changed
    MasterVolumeChanged(f32),
    /// Device added
    DeviceAdded(AudioDevice),
    /// Device removed
    DeviceRemoved(String),
    /// Active device changed
    ActiveDeviceChanged { device_type: DeviceClass, device_id: String },
}

/// Device class for active device changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Input,
    Output,
}

impl MediaManager {
    /// Create new media manager
    pub async fn new() -> Result<Self> {
        let (event_tx, mut event_rx) = mpsc::channel(256);
        
        // Create enhanced D-Bus manager
        let dbus_manager = Arc::new(EnhancedDBusManager::new().await?);
        
        let manager = Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            active_player: Arc::new(RwLock::new(None)),
            event_tx: event_tx.clone(),
            volume_control: Arc::new(RwLock::new(VolumeControl::default())),
            graph_integration: Arc::new(RwLock::new(MediaGraphIntegration::new())),
            dbus_manager: dbus_manager.clone(),
        };
        
        // Initialize MPRIS monitoring
        manager.init_mpris_monitor().await?;
        
        // Initialize audio system monitoring
        manager.init_audio_monitor().await?;
        
        // Spawn event handler
        let players = manager.players.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                Self::handle_event(&players, event).await;
            }
        });
        
        Ok(manager)
    }
    
    /// Initialize MPRIS monitoring
    async fn init_mpris_monitor(&self) -> Result<()> {
        // Discover active media players
        let media_players = self.dbus_manager.discover_media_players().await?;
        
        // Add proxies for each discovered player
        for player_service in &media_players {
            if let Err(e) = self.dbus_manager.add_media_player(player_service).await {
                log::warn!("Failed to add media player {}: {}", player_service, e);
            }
        }
        
        // Create media players from discovered services
        for player_service in &media_players {
            if let Ok(player) = self.create_media_player_from_service(player_service).await {
                self.add_player(player).await?;
            }
        }
        
        // If no players found, create a mock player for testing
        if media_players.is_empty() {
            let mock_player = MediaPlayer {
                id: "org.mpris.MediaPlayer2.MockPlayer".to_string(),
                name: "Mock Music Player".to_string(),
                status: PlaybackStatus::Playing,
                metadata: Some(TrackMetadata {
                    track_id: Some("track123".to_string()),
                    title: "Example Song".to_string(),
                    artists: vec!["Example Artist".to_string()],
                    album: Some("Example Album".to_string()),
                    album_artist: None,
                    length: Some(Duration::from_secs(180)),
                    art_url: Some("https://example.com/album-art.jpg".to_string()),
                    url: None,
                }),
                volume: 0.75,
                position: Duration::from_secs(45),
                can_control: true,
                can_go_next: true,
                can_go_previous: true,
                can_seek: true,
                node_id: None,
            };
            
            self.add_player(mock_player).await?;
        }
        
        Ok(())
    }
    
    /// Create MediaPlayer from D-Bus service
    async fn create_media_player_from_service(&self, service: &str) -> Result<MediaPlayer> {
        if let Some(player_proxy) = self.dbus_manager.get_media_player(service).await {
            // Get player properties - use service name as identity for now
            let identity = service.to_string();
            let can_control = player_proxy.can_control().await.unwrap_or(false);
            let can_go_next = player_proxy.can_go_next().await.unwrap_or(false);
            let can_go_previous = player_proxy.can_go_previous().await.unwrap_or(false);
            let can_seek = player_proxy.can_seek().await.unwrap_or(false);
            let volume = player_proxy.volume().await.unwrap_or(1.0) as f32;
            let position = Duration::from_micros(player_proxy.position().await.unwrap_or(0) as u64);
            
            // Get playback status
            let status = match player_proxy.playback_status().await.unwrap_or_else(|_| "Stopped".to_string()).as_str() {
                "Playing" => PlaybackStatus::Playing,
                "Paused" => PlaybackStatus::Paused,
                _ => PlaybackStatus::Stopped,
            };
            
            // Get metadata - simplified for now
            let metadata = if let Ok(_metadata_map) = player_proxy.metadata().await {
                Some(TrackMetadata {
                    track_id: Some("unknown".to_string()),
                    title: "Unknown Track".to_string(),
                    artists: vec!["Unknown Artist".to_string()],
                    album: Some("Unknown Album".to_string()),
                    album_artist: None,
                    length: Some(Duration::from_secs(180)),
                    art_url: None,
                    url: None,
                })
            } else {
                None
            };
            
            Ok(MediaPlayer {
                id: service.to_string(),
                name: identity,
                status,
                metadata,
                volume,
                position,
                can_control,
                can_go_next,
                can_go_previous,
                can_seek,
                node_id: None,
            })
        } else {
            Err(anyhow::anyhow!("Failed to get media player proxy for {}", service))
        }
    }
    
    /// Initialize audio system monitoring
    async fn init_audio_monitor(&self) -> Result<()> {
        // TODO: Implement PulseAudio/PipeWire monitoring
        // For now, create mock devices
        let mut volume_control = self.volume_control.write().unwrap();
        
        volume_control.output_devices.push(AudioDevice {
            id: "speakers".to_string(),
            name: "Built-in Speakers".to_string(),
            description: Some("Laptop speakers".to_string()),
            device_type: AudioDeviceType::Speakers,
            is_default: true,
            volume: 0.5,
            is_muted: false,
        });
        
        volume_control.output_devices.push(AudioDevice {
            id: "headphones".to_string(),
            name: "Headphones".to_string(),
            description: Some("3.5mm jack".to_string()),
            device_type: AudioDeviceType::Headphones,
            is_default: false,
            volume: 0.7,
            is_muted: false,
        });
        
        volume_control.input_devices.push(AudioDevice {
            id: "mic".to_string(),
            name: "Built-in Microphone".to_string(),
            description: Some("Laptop microphone".to_string()),
            device_type: AudioDeviceType::Microphone,
            is_default: true,
            volume: 0.8,
            is_muted: false,
        });
        
        volume_control.active_output = Some("speakers".to_string());
        volume_control.active_input = Some("mic".to_string());
        
        Ok(())
    }
    
    /// Add a media player
    pub async fn add_player(&self, player: MediaPlayer) -> Result<()> {
        let id = player.id.clone();
        let is_first = self.players.read().unwrap().is_empty();
        
        self.players.write().unwrap().insert(id.clone(), player.clone());
        
        // Set as active if first player
        if is_first {
            *self.active_player.write().unwrap() = Some(id.clone());
        }
        
        // Create graph node for player
        let node_id = self.graph_integration.write().unwrap()
            .create_player_node(&player);
        
        if let Some(node_id) = node_id {
            self.players.write().unwrap().get_mut(&id).unwrap().node_id = Some(node_id);
        }
        
        self.event_tx.send(MediaEvent::PlayerAdded(player)).await
            .context("Failed to send player added event")?;
        
        Ok(())
    }
    
    /// Remove a media player
    pub async fn remove_player(&self, id: &str) -> Result<()> {
        if let Some(player) = self.players.write().unwrap().remove(id) {
            // Remove from graph
            if let Some(node_id) = player.node_id {
                self.graph_integration.write().unwrap()
                    .remove_player_node(node_id);
            }
            
            // Update active player if needed
            if self.active_player.read().unwrap().as_ref() == Some(&id.to_string()) {
                let new_active = self.players.read().unwrap().keys().next().cloned();
                *self.active_player.write().unwrap() = new_active;
            }
            
            self.event_tx.send(MediaEvent::PlayerRemoved(id.to_string())).await
                .context("Failed to send player removed event")?;
        }
        
        Ok(())
    }
    
    /// Get all media players
    pub fn get_players(&self) -> Vec<MediaPlayer> {
        self.players.read().unwrap().values().cloned().collect()
    }
    
    /// Get active player
    pub fn get_active_player(&self) -> Option<MediaPlayer> {
        let active_id = self.active_player.read().unwrap();
        active_id.as_ref().and_then(|id| {
            self.players.read().unwrap().get(id).cloned()
        })
    }
    
    /// Set active player
    pub fn set_active_player(&self, id: &str) -> Result<()> {
        if self.players.read().unwrap().contains_key(id) {
            *self.active_player.write().unwrap() = Some(id.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Player {} not found", id))
        }
    }
    
    /// Control playback
    pub async fn play_pause(&self, player_id: Option<&str>) -> Result<()> {
        let id = player_id.map(String::from).or_else(|| self.active_player.read().unwrap().clone())
            .ok_or_else(|| anyhow::anyhow!("No active player"))?;
        
        // Use D-Bus to control playback
        if let Err(e) = self.dbus_manager.control_media_player(&id, MediaAction::PlayPause).await {
            log::warn!("Failed to control media player via D-Bus: {}", e);
        }
        
        // Update local state
        if let Some(player) = self.players.write().unwrap().get_mut(&id) {
            if player.can_control {
                player.status = match player.status {
                    PlaybackStatus::Playing => PlaybackStatus::Paused,
                    _ => PlaybackStatus::Playing,
                };
                
                let status = player.status;
                self.event_tx.send(MediaEvent::StatusChanged {
                    player_id: id.clone(),
                    status,
                }).await?;
            }
        }
        
        Ok(())
    }
    
    /// Skip to next track
    pub async fn next(&self, player_id: Option<&str>) -> Result<()> {
        let id = player_id.map(String::from).or_else(|| self.active_player.read().unwrap().clone())
            .ok_or_else(|| anyhow::anyhow!("No active player"))?;
        
        // Use D-Bus to control playback
        if let Err(e) = self.dbus_manager.control_media_player(&id, MediaAction::Next).await {
            log::warn!("Failed to control media player via D-Bus: {}", e);
        }
        
        if let Some(player) = self.players.read().unwrap().get(&id) {
            if player.can_go_next {
                log::info!("Skipping to next track on {}", id);
            }
        }
        
        Ok(())
    }
    
    /// Skip to previous track
    pub async fn previous(&self, player_id: Option<&str>) -> Result<()> {
        let id = player_id.map(String::from).or_else(|| self.active_player.read().unwrap().clone())
            .ok_or_else(|| anyhow::anyhow!("No active player"))?;
        
        // Use D-Bus to control playback
        if let Err(e) = self.dbus_manager.control_media_player(&id, MediaAction::Previous).await {
            log::warn!("Failed to control media player via D-Bus: {}", e);
        }
        
        if let Some(player) = self.players.read().unwrap().get(&id) {
            if player.can_go_previous {
                log::info!("Skipping to previous track on {}", id);
            }
        }
        
        Ok(())
    }
    
    /// Seek to position
    pub async fn seek(&self, position: Duration, player_id: Option<&str>) -> Result<()> {
        let id = player_id.map(String::from).or_else(|| self.active_player.read().unwrap().clone())
            .ok_or_else(|| anyhow::anyhow!("No active player"))?;
        
        let offset = position.as_micros() as i64;
        
        // Use D-Bus to control playback
        if let Err(e) = self.dbus_manager.control_media_player(&id, MediaAction::Seek(offset)).await {
            log::warn!("Failed to seek media player via D-Bus: {}", e);
        }
        
        if let Some(player) = self.players.write().unwrap().get_mut(&id) {
            if player.can_seek {
                player.position = position;
                
                self.event_tx.send(MediaEvent::PositionChanged {
                    player_id: id.clone(),
                    position,
                }).await?;
            }
        }
        
        Ok(())
    }
    
    /// Set master volume
    pub async fn set_master_volume(&self, volume: f32) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0);
        self.volume_control.write().unwrap().master_volume = volume;
        
        self.event_tx.send(MediaEvent::MasterVolumeChanged(volume)).await
            .context("Failed to send volume change event")?;
        
        Ok(())
    }
    
    /// Toggle mute
    pub fn toggle_mute(&self) -> Result<()> {
        let mut control = self.volume_control.write().unwrap();
        control.is_muted = !control.is_muted;
        Ok(())
    }
    
    /// Get volume control
    pub fn get_volume_control(&self) -> VolumeControl {
        self.volume_control.read().unwrap().clone()
    }
    
    /// Set active output device
    pub async fn set_active_output(&self, device_id: &str) -> Result<()> {
        let mut control = self.volume_control.write().unwrap();
        
        if control.output_devices.iter().any(|d| d.id == device_id) {
            control.active_output = Some(device_id.to_string());
            
            self.event_tx.send(MediaEvent::ActiveDeviceChanged {
                device_type: DeviceClass::Output,
                device_id: device_id.to_string(),
            }).await?;
        } else {
            return Err(anyhow::anyhow!("Output device {} not found", device_id));
        }
        
        Ok(())
    }
    
    /// Set active input device
    pub async fn set_active_input(&self, device_id: &str) -> Result<()> {
        let mut control = self.volume_control.write().unwrap();
        
        if control.input_devices.iter().any(|d| d.id == device_id) {
            control.active_input = Some(device_id.to_string());
            
            self.event_tx.send(MediaEvent::ActiveDeviceChanged {
                device_type: DeviceClass::Input,
                device_id: device_id.to_string(),
            }).await?;
        } else {
            return Err(anyhow::anyhow!("Input device {} not found", device_id));
        }
        
        Ok(())
    }
    
    /// Handle media event
    async fn handle_event(_players: &Arc<RwLock<HashMap<String, MediaPlayer>>>, event: MediaEvent) {
        match event {
            MediaEvent::PlayerAdded(player) => {
                log::info!("Media player added: {} ({})", player.name, player.id);
            }
            MediaEvent::PlayerRemoved(id) => {
                log::info!("Media player removed: {}", id);
            }
            MediaEvent::StatusChanged { player_id, status } => {
                log::debug!("Player {} status: {:?}", player_id, status);
            }
            MediaEvent::TrackChanged { player_id, metadata } => {
                log::info!("Now playing on {}: {} - {}", 
                    player_id, 
                    metadata.artists.join(", "), 
                    metadata.title
                );
            }
            MediaEvent::VolumeChanged { player_id, volume } => {
                log::debug!("Player {} volume: {}%", player_id, (volume * 100.0) as u32);
            }
            MediaEvent::MasterVolumeChanged(volume) => {
                log::debug!("Master volume: {}%", (volume * 100.0) as u32);
            }
            MediaEvent::ActiveDeviceChanged { device_type, device_id } => {
                log::info!("Active {:?} device changed to: {}", device_type, device_id);
            }
            _ => {}
        }
    }
}

/// Media graph integration
#[derive(Debug, Default)]
pub struct MediaGraphIntegration {
    /// Player to node mapping
    player_nodes: HashMap<String, u64>,
    /// Album nodes
    album_nodes: HashMap<String, u64>,
    /// Artist nodes
    artist_nodes: HashMap<String, u64>,
    /// Next node ID
    next_node_id: u64,
}

impl MediaGraphIntegration {
    /// Create new media graph integration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create player node
    pub fn create_player_node(&mut self, player: &MediaPlayer) -> Option<u64> {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        self.player_nodes.insert(player.id.clone(), node_id);
        
        // TODO: Actually create graph node
        log::debug!("Created player node {} for {}", node_id, player.name);
        
        Some(node_id)
    }
    
    /// Remove player node
    pub fn remove_player_node(&mut self, node_id: u64) {
        self.player_nodes.retain(|_, id| *id != node_id);
        
        // TODO: Actually remove graph node
        log::debug!("Removed player node {}", node_id);
    }
    
    /// Create or get album node
    pub fn get_or_create_album_node(&mut self, album: &str) -> u64 {
        if let Some(node_id) = self.album_nodes.get(album) {
            *node_id
        } else {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            
            self.album_nodes.insert(album.to_string(), node_id);
            
            // TODO: Actually create graph node
            log::debug!("Created album node {} for {}", node_id, album);
            
            node_id
        }
    }
    
    /// Create or get artist node
    pub fn get_or_create_artist_node(&mut self, artist: &str) -> u64 {
        if let Some(node_id) = self.artist_nodes.get(artist) {
            *node_id
        } else {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            
            self.artist_nodes.insert(artist.to_string(), node_id);
            
            // TODO: Actually create graph node
            log::debug!("Created artist node {} for {}", node_id, artist);
            
            node_id
        }
    }
    
    /// Create edges for current track
    pub fn create_track_edges(&mut self, player_node: u64, metadata: &TrackMetadata) {
        // Create album edge if album exists
        if let Some(album) = &metadata.album {
            let album_node = self.get_or_create_album_node(album);
            // TODO: Create edge between player and album
            log::debug!("Created edge: player {} -> album {}", player_node, album_node);
        }
        
        // Create artist edges
        for artist in &metadata.artists {
            let artist_node = self.get_or_create_artist_node(artist);
            // TODO: Create edge between player and artist
            log::debug!("Created edge: player {} -> artist {}", player_node, artist_node);
        }
    }
}

impl Default for VolumeControl {
    fn default() -> Self {
        Self {
            master_volume: 0.5,
            is_muted: false,
            app_volumes: HashMap::new(),
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            active_output: None,
            active_input: None,
        }
    }
}

/// Media control widget for graph UI
#[derive(Debug, Clone)]
pub struct MediaControlWidget {
    /// Position in graph space
    pub position: (f32, f32, f32),
    /// Size
    pub size: (f32, f32),
    /// Is expanded
    pub expanded: bool,
    /// Associated player ID
    pub player_id: Option<String>,
}

impl MediaControlWidget {
    /// Create new media control widget
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0, 0.0),
            size: (200.0, 60.0),
            expanded: false,
            player_id: None,
        }
    }
    
    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        
        // Adjust size based on state
        self.size = if self.expanded {
            (300.0, 200.0)
        } else {
            (200.0, 60.0)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_media_manager() {
        let manager = MediaManager::new().await.unwrap();
        
        // Should have mock player
        let players = manager.get_players();
        assert!(!players.is_empty());
        
        // Test playback control
        manager.play_pause(None).await.unwrap();
        
        // Test volume control
        manager.set_master_volume(0.75).await.unwrap();
        assert_eq!(manager.get_volume_control().master_volume, 0.75);
    }
    
    #[test]
    fn test_media_graph_integration() {
        let mut integration = MediaGraphIntegration::new();
        
        // Test node creation
        let album_node1 = integration.get_or_create_album_node("Test Album");
        let album_node2 = integration.get_or_create_album_node("Test Album");
        assert_eq!(album_node1, album_node2); // Should return same node
        
        let artist_node = integration.get_or_create_artist_node("Test Artist");
        assert_ne!(album_node1, artist_node); // Should be different
    }
    
    #[test]
    fn test_media_widget() {
        let mut widget = MediaControlWidget::new();
        assert!(!widget.expanded);
        assert_eq!(widget.size, (200.0, 60.0));
        
        widget.toggle_expanded();
        assert!(widget.expanded);
        assert_eq!(widget.size, (300.0, 200.0));
    }
}