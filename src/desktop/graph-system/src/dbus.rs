//! D-Bus integration for system services

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use dbus::{
    nonblock::{SyncConnection, Proxy},
    Message,
};
use dbus_tokio::connection;
use log::{debug, info, warn, error};
use tokio::sync::{mpsc, RwLock};

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
            system_conn: Arc::new(system_conn),
            session_conn: Arc::new(session_conn),
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
            let conn = match service.bus {
                BusType::System => &self.system_conn,
                BusType::Session => &self.session_conn,
            };
            
            let proxy = Proxy::new(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                std::time::Duration::from_secs(5),
                conn,
            );
            
            let (result,): (u32,) = proxy
                .method_call("org.freedesktop.DBus", "RequestName", (name.as_str(), 0u32))
                .await?;
            
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
        let conn = match service.bus {
            BusType::System => &self.system_conn,
            BusType::Session => &self.session_conn,
        };
        
        for signal in &service.signals {
            let rule = format!(
                "type='signal',interface='{}',member='{}'",
                signal.interface, signal.member
            );
            
            let proxy = Proxy::new(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                std::time::Duration::from_secs(5),
                conn,
            );
            
            proxy
                .method_call("org.freedesktop.DBus", "AddMatch", (rule.as_str(),))
                .await?;
            
            debug!("Added signal match: {}", rule);
        }
        
        // Start signal handler
        let conn_clone = conn.clone();
        let event_tx = self.event_tx.clone();
        let service_name = service.name.clone();
        
        tokio::spawn(async move {
            conn_clone.add_match_no_cb(&rule).await.unwrap();
            
            let mut stream = conn_clone.incoming(1000);
            while let Some(msg) = stream.next().await {
                if let Some(signal_info) = Self::parse_signal(&msg) {
                    let _ = event_tx.send(DBusEvent::Signal {
                        service: service_name.clone(),
                        signal: signal_info,
                    });
                }
            }
        });
        
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
    fn extract_args(msg: &Message) -> Vec<serde_json::Value> {
        // TODO: Implement proper argument extraction
        vec![]
    }
    
    /// Call a D-Bus method
    pub async fn call_method(
        &self,
        service: &str,
        object_path: &str,
        interface: &str,
        method: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        let service_info = self.services.read().await
            .get(service)
            .ok_or_else(|| anyhow::anyhow!("Service not found: {}", service))?
            .clone();
        
        let conn = match service_info.bus {
            BusType::System => &self.system_conn,
            BusType::Session => &self.session_conn,
        };
        
        let proxy = Proxy::new(
            service,
            object_path,
            std::time::Duration::from_secs(5),
            conn,
        );
        
        // TODO: Implement proper method calling with argument marshaling
        
        Ok(vec![])
    }
    
    /// Get service proxy
    pub fn get_proxy<'a>(
        &'a self,
        service: &str,
        path: &str,
        bus: BusType,
    ) -> Proxy<'a, &'a SyncConnection> {
        let conn = match bus {
            BusType::System => &self.system_conn,
            BusType::Session => &self.session_conn,
        };
        
        Proxy::new(service, path, std::time::Duration::from_secs(5), conn)
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