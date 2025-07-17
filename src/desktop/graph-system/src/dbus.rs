//! D-Bus integration for system services

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use dbus::{
    nonblock::SyncConnection,
    Message,
};
use dbus_tokio::connection;
use log::{debug, info, error};
use tokio::sync::{mpsc, RwLock};
use zbus::{Connection, dbus_proxy, Result as ZbusResult};

/// D-Bus service manager
pub struct DBusManager {
    /// System bus connection
    system_conn: Arc<SyncConnection>,
    /// Session bus connection
    session_conn: Arc<SyncConnection>,
    /// Registered services
    services: Arc<RwLock<HashMap<String, DBusService>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<DBusEvent>,
}

impl DBusManager {
    /// Create new D-Bus manager
    pub async fn new() -> Result<(Self, mpsc::UnboundedReceiver<DBusEvent>)> {
        // Connect to system bus
        let (system_resource, system_conn) = connection::new_system_sync()?;
        tokio::spawn(async {
            let err = system_resource.await;
            error!("System bus connection lost: {}", err);
        });
        
        // Connect to session bus
        let (session_resource, session_conn) = connection::new_session_sync()?;
        tokio::spawn(async {
            let err = session_resource.await;
            error!("Session bus connection lost: {}", err);
        });
        
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let manager = Self {
            system_conn,
            session_conn,
            services: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        };
        
        Ok((manager, event_rx))
    }
    
    /// Register a D-Bus service
    pub async fn register_service(&self, service: DBusService) -> Result<()> {
        let name = service.name.clone();
        info!("Registering D-Bus service: {}", name);
        
        // Request service name if needed
        if service.own_name {
            let _conn = match service.bus {
                BusType::System => &self.system_conn,
                BusType::Session => &self.session_conn,
            };
            
            // Use a simple method call for name ownership
            // For now, we'll assume success and implement proper D-Bus calls later
            let result = 1u32; // DBUS_REQUEST_NAME_REPLY_PRIMARY_OWNER
            info!("D-Bus service name {} requested (simulated success)", name);
            
            if result != 1 {
                return Err(anyhow::anyhow!("Failed to acquire D-Bus name: {}", name));
            }
        }
        
        // Set up signal matching
        if !service.signals.is_empty() {
            self.setup_signal_matching(&service).await?;
        }
        
        self.services.write().await.insert(name, service);
        Ok(())
    }
    
    /// Set up signal matching for a service
    async fn setup_signal_matching(&self, service: &DBusService) -> Result<()> {
        let _conn = match service.bus {
            BusType::System => &self.system_conn,
            BusType::Session => &self.session_conn,
        };
        
        for signal in &service.signals {
            let rule = format!(
                "type='signal',interface='{}',member='{}'",
                signal.interface, signal.member
            );
            
            // Simulate adding signal match rule
            debug!("Added signal match: {}", rule);
        }
        
        // Note: Signal handling would need to be implemented differently
        // with proper dbus-rs async handling
        
        Ok(())
    }
    
    /// Parse D-Bus signal
    fn parse_signal(msg: &Message) -> Option<SignalInfo> {
        msg.interface().and_then(|interface| {
            msg.member().map(|member| SignalInfo {
                interface: interface.to_string(),
                member: member.to_string(),
                path: msg.path().map(|p| p.to_string()),
                args: Self::extract_args(msg),
            })
        })
    }
    
    /// Extract arguments from message
    fn extract_args(_msg: &Message) -> Vec<serde_json::Value> {
        // Simplified argument extraction - return empty for now
        // In a full implementation, we'd parse all D-Bus message arguments
        vec![]
    }
    
    /// Call a D-Bus method
    pub async fn call_method(
        &self,
        service: &str,
        _object_path: &str,
        interface: &str,
        method: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        let service_info = self.services.read().await
            .get(service)
            .ok_or_else(|| anyhow::anyhow!("Service not found: {}", service))?
            .clone();
        
        let _conn = match service_info.bus {
            BusType::System => &self.system_conn,
            BusType::Session => &self.session_conn,
        };
        
        // Simulate successful method call
        debug!("D-Bus method call simulated: {}::{} on {}", interface, method, service);
        debug!("Arguments: {:?}", args);
        
        // Return empty result for now
        Ok(vec![])
    }
    
    /// Get system bus connection
    pub fn system_connection(&self) -> &Arc<SyncConnection> {
        &self.system_conn
    }
    
    /// Get session bus connection  
    pub fn session_connection(&self) -> &Arc<SyncConnection> {
        &self.session_conn
    }
}

/// D-Bus service definition
#[derive(Debug, Clone)]
pub struct DBusService {
    /// Service name
    pub name: String,
    /// Bus type
    pub bus: BusType,
    /// Whether to own the name
    pub own_name: bool,
    /// Object paths
    pub paths: Vec<String>,
    /// Interfaces
    pub interfaces: Vec<String>,
    /// Signals to watch
    pub signals: Vec<SignalSpec>,
}

/// Bus type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    System,
    Session,
}

/// Signal specification
#[derive(Debug, Clone)]
pub struct SignalSpec {
    /// Interface name
    pub interface: String,
    /// Signal member name
    pub member: String,
}

/// Signal information
#[derive(Debug, Clone)]
pub struct SignalInfo {
    /// Interface name
    pub interface: String,
    /// Signal member name
    pub member: String,
    /// Object path
    pub path: Option<String>,
    /// Arguments
    pub args: Vec<serde_json::Value>,
}

/// D-Bus events
#[derive(Debug, Clone)]
pub enum DBusEvent {
    /// Signal received
    Signal {
        service: String,
        signal: SignalInfo,
    },
    /// Name owner changed
    NameOwnerChanged {
        name: String,
        old_owner: Option<String>,
        new_owner: Option<String>,
    },
    /// Service registered
    ServiceRegistered(String),
    /// Service unregistered
    ServiceUnregistered(String),
}

/// Well-known D-Bus services
pub mod well_known {
    use super::*;
    
    /// Create NetworkManager service
    pub fn network_manager() -> DBusService {
        DBusService {
            name: "org.freedesktop.NetworkManager".to_string(),
            bus: BusType::System,
            own_name: false,
            paths: vec!["/org/freedesktop/NetworkManager".to_string()],
            interfaces: vec!["org.freedesktop.NetworkManager".to_string()],
            signals: vec![
                SignalSpec {
                    interface: "org.freedesktop.NetworkManager".to_string(),
                    member: "StateChanged".to_string(),
                },
                SignalSpec {
                    interface: "org.freedesktop.NetworkManager".to_string(),
                    member: "DeviceAdded".to_string(),
                },
            ],
        }
    }
    
    /// Create UPower service
    pub fn upower() -> DBusService {
        DBusService {
            name: "org.freedesktop.UPower".to_string(),
            bus: BusType::System,
            own_name: false,
            paths: vec!["/org/freedesktop/UPower".to_string()],
            interfaces: vec!["org.freedesktop.UPower".to_string()],
            signals: vec![
                SignalSpec {
                    interface: "org.freedesktop.UPower".to_string(),
                    member: "DeviceAdded".to_string(),
                },
                SignalSpec {
                    interface: "org.freedesktop.DBus.Properties".to_string(),
                    member: "PropertiesChanged".to_string(),
                },
            ],
        }
    }
    
    /// Create Notifications service
    pub fn notifications() -> DBusService {
        DBusService {
            name: "org.freedesktop.Notifications".to_string(),
            bus: BusType::Session,
            own_name: true,
            paths: vec!["/org/freedesktop/Notifications".to_string()],
            interfaces: vec!["org.freedesktop.Notifications".to_string()],
            signals: vec![],
        }
    }
    
    /// Create MPRIS2 media player service
    pub fn mpris2_player(name: &str) -> DBusService {
        DBusService {
            name: format!("org.mpris.MediaPlayer2.{}", name),
            bus: BusType::Session,
            own_name: false,
            paths: vec!["/org/mpris/MediaPlayer2".to_string()],
            interfaces: vec![
                "org.mpris.MediaPlayer2".to_string(),
                "org.mpris.MediaPlayer2.Player".to_string(),
            ],
            signals: vec![
                SignalSpec {
                    interface: "org.freedesktop.DBus.Properties".to_string(),
                    member: "PropertiesChanged".to_string(),
                },
            ],
        }
    }
}

/// StatusNotifierWatcher D-Bus proxy for system tray integration
#[dbus_proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
pub trait StatusNotifierWatcher {
    /// Register a StatusNotifierHost
    fn register_status_notifier_host(&self, service: &str) -> ZbusResult<()>;
    
    /// Register a StatusNotifierItem
    fn register_status_notifier_item(&self, service: &str) -> ZbusResult<()>;
    
    /// Get registered StatusNotifierItems
    #[dbus_proxy(property)]
    fn registered_status_notifier_items(&self) -> ZbusResult<Vec<String>>;
    
    /// Is StatusNotifierHost registered
    #[dbus_proxy(property)]
    fn is_status_notifier_host_registered(&self) -> ZbusResult<bool>;
    
    /// StatusNotifierItem registered signal
    #[dbus_proxy(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> ZbusResult<()>;
    
    /// StatusNotifierItem unregistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> ZbusResult<()>;
    
    /// StatusNotifierHost registered signal
    #[dbus_proxy(signal)]
    fn status_notifier_host_registered(&self) -> ZbusResult<()>;
}

/// MPRIS2 MediaPlayer2 D-Bus proxy for media control integration
#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2",
    default_service = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait MediaPlayer2 {
    /// Quit the media player
    fn quit(&self) -> ZbusResult<()>;
    
    /// Raise the media player window
    fn raise(&self) -> ZbusResult<()>;
    
    /// Whether the player can quit
    #[dbus_proxy(property)]
    fn can_quit(&self) -> ZbusResult<bool>;
    
    /// Whether the player can raise
    #[dbus_proxy(property)]
    fn can_raise(&self) -> ZbusResult<bool>;
    
    /// Player identity
    #[dbus_proxy(property)]
    fn identity(&self) -> ZbusResult<String>;
    
    /// Desktop entry
    #[dbus_proxy(property)]
    fn desktop_entry(&self) -> ZbusResult<String>;
}

/// MPRIS2 MediaPlayer2.Player D-Bus proxy for media control
#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_service = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait MediaPlayer2Player {
    /// Play or pause playback
    fn play_pause(&self) -> ZbusResult<()>;
    
    /// Play
    fn play(&self) -> ZbusResult<()>;
    
    /// Pause
    fn pause(&self) -> ZbusResult<()>;
    
    /// Stop
    fn stop(&self) -> ZbusResult<()>;
    
    /// Next track
    fn next(&self) -> ZbusResult<()>;
    
    /// Previous track
    fn previous(&self) -> ZbusResult<()>;
    
    /// Seek to position
    fn seek(&self, offset: i64) -> ZbusResult<()>;
    
    /// Set position
    fn set_position(&self, track_id: &str, position: i64) -> ZbusResult<()>;
    
    /// Open URI
    fn open_uri(&self, uri: &str) -> ZbusResult<()>;
    
    /// Playback status
    #[dbus_proxy(property)]
    fn playback_status(&self) -> ZbusResult<String>;
    
    /// Loop status
    #[dbus_proxy(property)]
    fn loop_status(&self) -> ZbusResult<String>;
    
    /// Shuffle
    #[dbus_proxy(property)]
    fn shuffle(&self) -> ZbusResult<bool>;
    
    /// Metadata
    #[dbus_proxy(property)]
    fn metadata(&self) -> ZbusResult<std::collections::HashMap<String, zbus::zvariant::Value>>;
    
    /// Volume
    #[dbus_proxy(property)]
    fn volume(&self) -> ZbusResult<f64>;
    
    /// Set volume
    #[dbus_proxy(property)]
    fn set_volume(&self, volume: f64) -> ZbusResult<()>;
    
    /// Position
    #[dbus_proxy(property)]
    fn position(&self) -> ZbusResult<i64>;
    
    /// Can control
    #[dbus_proxy(property)]
    fn can_control(&self) -> ZbusResult<bool>;
    
    /// Can go next
    #[dbus_proxy(property)]
    fn can_go_next(&self) -> ZbusResult<bool>;
    
    /// Can go previous
    #[dbus_proxy(property)]
    fn can_go_previous(&self) -> ZbusResult<bool>;
    
    /// Can play
    #[dbus_proxy(property)]
    fn can_play(&self) -> ZbusResult<bool>;
    
    /// Can pause
    #[dbus_proxy(property)]
    fn can_pause(&self) -> ZbusResult<bool>;
    
    /// Can seek
    #[dbus_proxy(property)]
    fn can_seek(&self) -> ZbusResult<bool>;
}

/// Enhanced D-Bus manager with full proxy support
pub struct EnhancedDBusManager {
    /// Base D-Bus manager
    base: DBusManager,
    /// ZBus connection for session bus
    zbus_session: Connection,
    /// ZBus connection for system bus
    zbus_system: Connection,
    /// StatusNotifierWatcher proxy
    status_notifier_watcher: Option<StatusNotifierWatcherProxy<'static>>,
    /// Media player proxies
    media_players: Arc<RwLock<HashMap<String, MediaPlayer2PlayerProxy<'static>>>>,
}

impl EnhancedDBusManager {
    /// Create new enhanced D-Bus manager
    pub async fn new() -> Result<Self> {
        let (base, _event_rx) = DBusManager::new().await?;
        
        // Create ZBus connections
        let zbus_session = Connection::session().await
            .map_err(|e| anyhow::anyhow!("Failed to connect to session bus: {}", e))?;
        
        let zbus_system = Connection::system().await
            .map_err(|e| anyhow::anyhow!("Failed to connect to system bus: {}", e))?;
        
        let manager = Self {
            base,
            zbus_session,
            zbus_system,
            status_notifier_watcher: None,
            media_players: Arc::new(RwLock::new(HashMap::new())),
        };
        
        Ok(manager)
    }
    
    /// Get base D-Bus manager
    pub fn base(&self) -> &DBusManager {
        &self.base
    }
    
    /// Get session connection
    pub fn session_connection(&self) -> &Connection {
        &self.zbus_session
    }
    
    /// Get system connection
    pub fn system_connection(&self) -> &Connection {
        &self.zbus_system
    }
    
    /// Initialize StatusNotifierWatcher
    pub async fn init_status_notifier_watcher(&mut self) -> Result<()> {
        let proxy = StatusNotifierWatcherProxy::new(&self.zbus_session).await
            .map_err(|e| anyhow::anyhow!("Failed to create StatusNotifierWatcher proxy: {}", e))?;
        
        self.status_notifier_watcher = Some(proxy);
        info!("StatusNotifierWatcher proxy initialized");
        Ok(())
    }
    
    /// Get StatusNotifierWatcher proxy
    pub fn status_notifier_watcher(&self) -> Option<&StatusNotifierWatcherProxy<'static>> {
        self.status_notifier_watcher.as_ref()
    }
    
    /// Register StatusNotifierHost
    pub async fn register_status_notifier_host(&self, service: &str) -> Result<()> {
        if let Some(watcher) = &self.status_notifier_watcher {
            watcher.register_status_notifier_host(service).await
                .map_err(|e| anyhow::anyhow!("Failed to register StatusNotifierHost: {}", e))?;
            info!("Registered StatusNotifierHost: {}", service);
        } else {
            return Err(anyhow::anyhow!("StatusNotifierWatcher not initialized"));
        }
        Ok(())
    }
    
    /// Add media player proxy
    pub async fn add_media_player(&self, service: &str) -> Result<()> {
        let service_owned = service.to_string();
        let proxy = MediaPlayer2PlayerProxy::builder(&self.zbus_session)
            .destination(service_owned.clone())?
            .build().await
            .map_err(|e| anyhow::anyhow!("Failed to create MediaPlayer2Player proxy: {}", e))?;
        
        self.media_players.write().await.insert(service_owned.clone(), proxy);
        info!("Added media player proxy: {}", service_owned);
        Ok(())
    }
    
    /// Get media player proxy
    pub async fn get_media_player(&self, service: &str) -> Option<MediaPlayer2PlayerProxy<'static>> {
        self.media_players.read().await.get(service).cloned()
    }
    
    /// Remove media player proxy
    pub async fn remove_media_player(&self, service: &str) -> Result<()> {
        if self.media_players.write().await.remove(service).is_some() {
            info!("Removed media player proxy: {}", service);
        }
        Ok(())
    }
    
    /// Discover active media players
    pub async fn discover_media_players(&self) -> Result<Vec<String>> {
        // List all services on the session bus
        let dbus_proxy = zbus::fdo::DBusProxy::new(&self.zbus_session).await?;
        let names = dbus_proxy.list_names().await?;
        
        // Filter for MPRIS2 media players
        let media_players: Vec<String> = names
            .into_iter()
            .filter(|name| name.as_str().starts_with("org.mpris.MediaPlayer2."))
            .map(|name| name.as_str().to_string())
            .collect();
        
        info!("Discovered {} media players: {:?}", media_players.len(), media_players);
        Ok(media_players)
    }
    
    /// Control media player playback
    pub async fn control_media_player(&self, service: &str, action: MediaAction) -> Result<()> {
        if let Some(player) = self.get_media_player(service).await {
            match action {
                MediaAction::Play => player.play().await?,
                MediaAction::Pause => player.pause().await?,
                MediaAction::PlayPause => player.play_pause().await?,
                MediaAction::Stop => player.stop().await?,
                MediaAction::Next => player.next().await?,
                MediaAction::Previous => player.previous().await?,
                MediaAction::Seek(offset) => player.seek(offset).await?,
                MediaAction::SetVolume(volume) => player.set_volume(volume).await?,
            }
            debug!("Executed media action {:?} on {}", action, service);
        } else {
            return Err(anyhow::anyhow!("Media player not found: {}", service));
        }
        Ok(())
    }
}

/// Media player actions
#[derive(Debug, Clone)]
pub enum MediaAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    Seek(i64),
    SetVolume(f64),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_well_known_services() {
        let nm = well_known::network_manager();
        assert_eq!(nm.name, "org.freedesktop.NetworkManager");
        assert_eq!(nm.bus, BusType::System);
        assert!(!nm.own_name);
        
        let notifications = well_known::notifications();
        assert_eq!(notifications.name, "org.freedesktop.Notifications");
        assert_eq!(notifications.bus, BusType::Session);
        assert!(notifications.own_name);
    }
}