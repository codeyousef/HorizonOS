//! Configuration file watcher for hot-reload functionality

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, Context};
use notify::{
    Watcher, RecursiveMode, Event, EventKind,
    RecommendedWatcher,
};
use tokio::sync::mpsc;

/// Configuration file watcher
pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    _watcher_task: tokio::task::JoinHandle<()>,
}

impl ConfigWatcher {
    /// Create new configuration watcher
    pub fn new<F>(config_dir: &Path, handler: F) -> Result<Self>
    where
        F: Fn(PathBuf) + Send + Sync + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let config_dir = config_dir.to_path_buf();
        let handler = Arc::new(handler);
        
        // Create file watcher
        let mut watcher = notify::recommended_watcher(move |event: Result<Event, notify::Error>| {
            if let Ok(event) = event {
                let _ = tx.send(event);
            }
        })?;
        
        // Watch configuration directory
        watcher.watch(&config_dir, RecursiveMode::Recursive)?;
        log::info!("Watching configuration directory: {:?}", config_dir);
        
        // Spawn handler task
        let watcher_task = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Some(path) = Self::should_handle_event(&event) {
                    log::debug!("Configuration file changed: {:?}", path);
                    handler(path);
                }
            }
        });
        
        Ok(Self {
            watcher,
            _watcher_task: watcher_task,
        })
    }
    
    /// Check if event should be handled
    fn should_handle_event(event: &Event) -> Option<PathBuf> {
        match &event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Check if it's a configuration file
                event.paths.iter()
                    .find(|path| Self::is_config_file(path))
                    .cloned()
            }
            _ => None,
        }
    }
    
    /// Check if file is a configuration file
    fn is_config_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "toml" | "json" | "yaml" | "yml"))
            .unwrap_or(false)
    }
    
    /// Add additional path to watch
    pub fn watch_path(&mut self, path: &Path) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::NonRecursive)
            .context("Failed to add watch path")
    }
    
    /// Remove path from watching
    pub fn unwatch_path(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path)
            .context("Failed to remove watch path")
    }
}

/// Configuration watcher with debouncing
pub struct DebouncedWatcher {
    watcher: ConfigWatcher,
    debounce_duration: Duration,
}

impl DebouncedWatcher {
    /// Create new debounced watcher
    pub fn new<F>(
        config_dir: &Path,
        debounce_duration: Duration,
        handler: F,
    ) -> Result<Self>
    where
        F: Fn(PathBuf) + Send + Sync + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let handler = Arc::new(handler);
        
        // Create base watcher that sends to channel
        let watcher = ConfigWatcher::new(config_dir, move |path| {
            let _ = tx.send(path);
        })?;
        
        // Spawn debounced handler
        tokio::spawn(async move {
            let mut pending_path: Option<PathBuf> = None;
            let mut timer = tokio::time::interval(debounce_duration);
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                tokio::select! {
                    Some(path) = rx.recv() => {
                        pending_path = Some(path);
                    }
                    _ = timer.tick() => {
                        if let Some(path) = pending_path.take() {
                            handler(path);
                        }
                    }
                }
            }
        });
        
        Ok(Self {
            watcher,
            debounce_duration,
        })
    }
}

/// Theme watcher specifically for theme files
pub struct ThemeWatcher {
    watcher: ConfigWatcher,
    theme_cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u64>>>,
}

impl ThemeWatcher {
    /// Create new theme watcher
    pub fn new<F>(themes_dir: &Path, handler: F) -> Result<Self>
    where
        F: Fn(String, PathBuf) + Send + Sync + 'static,
    {
        let theme_cache = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
        let cache_clone = theme_cache.clone();
        let handler = Arc::new(handler);
        
        let watcher = ConfigWatcher::new(themes_dir, move |path| {
            if let Some(theme_name) = path.file_stem().and_then(|s| s.to_str()) {
                let handler = handler.clone();
                let cache = cache_clone.clone();
                let theme_name = theme_name.to_string();
                let path_clone = path.clone();
                
                tokio::spawn(async move {
                    // Check file modification time to avoid duplicate events
                    if let Ok(metadata) = tokio::fs::metadata(&path_clone).await {
                        let modified = metadata.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        
                        let mut cache = cache.write().await;
                        let last_modified = cache.get(&theme_name).copied().unwrap_or(0);
                        
                        if modified > last_modified {
                            cache.insert(theme_name.clone(), modified);
                            drop(cache);
                            handler(theme_name, path_clone);
                        }
                    }
                });
            }
        })?;
        
        Ok(Self {
            watcher,
            theme_cache,
        })
    }
}

/// Multi-path configuration watcher
pub struct MultiPathWatcher {
    watchers: Vec<ConfigWatcher>,
}

impl MultiPathWatcher {
    /// Create new multi-path watcher
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }
    
    /// Add path to watch
    pub fn add_path<F>(&mut self, path: &Path, handler: F) -> Result<()>
    where
        F: Fn(PathBuf) + Send + Sync + 'static,
    {
        let watcher = ConfigWatcher::new(path, handler)?;
        self.watchers.push(watcher);
        Ok(())
    }
    
    /// Watch system config directories
    pub fn watch_system_configs<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(PathBuf) + Send + Sync + Clone + 'static,
    {
        // Watch user config directory
        if let Some(config_home) = dirs::config_dir() {
            let horizonos_config = config_home.join("horizonos");
            if horizonos_config.exists() {
                self.add_path(&horizonos_config, handler.clone())?;
            }
        }
        
        // Watch system config directory
        let system_config = Path::new("/etc/horizonos");
        if system_config.exists() {
            self.add_path(system_config, handler.clone())?;
        }
        
        // Watch local config directory
        let local_config = Path::new("./config");
        if local_config.exists() {
            self.add_path(local_config, handler)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_file_detection() {
        assert!(ConfigWatcher::is_config_file(Path::new("config.toml")));
        assert!(ConfigWatcher::is_config_file(Path::new("theme.json")));
        assert!(ConfigWatcher::is_config_file(Path::new("settings.yaml")));
        assert!(!ConfigWatcher::is_config_file(Path::new("readme.md")));
        assert!(!ConfigWatcher::is_config_file(Path::new("script.sh")));
    }
    
    #[tokio::test]
    async fn test_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        
        let result = ConfigWatcher::new(temp_dir.path(), move |path| {
            let _ = tx.send(path);
        });
        
        assert!(result.is_ok());
    }
}