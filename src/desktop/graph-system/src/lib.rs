//! System integration for HorizonOS graph desktop

pub mod dbus;
pub mod tray;
pub mod monitors;
pub mod power;
pub mod media;

pub use dbus::{EnhancedDBusManager, DBusManager, MediaAction};
pub use tray::{SystemTrayManager, TrayItem, GraphTrayIntegration};
pub use monitors::{MonitorManager, Monitor, MonitorLayout, GraphViewport};
pub use power::{PowerManager, PowerProfile, GraphPowerSettings, NodePowerManager};
pub use media::{MediaManager, MediaPlayer, VolumeControl, MediaControlWidget};