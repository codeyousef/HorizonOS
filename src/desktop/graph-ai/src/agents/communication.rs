//! Agent communication protocols
//! 
//! This module provides communication protocols for AI agents to interact with each other,
//! share information, coordinate tasks, and collaborate on complex problems.

use crate::AIError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, broadcast};
use tokio::time::Duration;
use log::{info, debug};
use uuid::Uuid;

/// Communication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    /// Enable agent communication
    pub enabled: bool,
    /// Maximum message size (bytes)
    pub max_message_size: usize,
    /// Message timeout (seconds)
    pub message_timeout: u64,
    /// Maximum concurrent conversations
    pub max_concurrent_conversations: usize,
    /// Enable message encryption
    pub enable_encryption: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Message retention period (hours)
    pub message_retention_hours: u32,
    /// Enable broadcast messaging
    pub enable_broadcast: bool,
    /// Broadcast channel capacity
    pub broadcast_capacity: usize,
    /// Enable peer-to-peer messaging
    pub enable_p2p: bool,
    /// Enable group messaging
    pub enable_group: bool,
    /// Maximum group size
    pub max_group_size: usize,
    /// Enable message history
    pub enable_message_history: bool,
    /// Message history limit
    pub message_history_limit: usize,
    /// Enable presence tracking
    pub enable_presence: bool,
    /// Presence update interval (seconds)
    pub presence_update_interval: u64,
}

impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_message_size: 1024 * 1024, // 1MB
            message_timeout: 30,
            max_concurrent_conversations: 100,
            enable_encryption: false,
            enable_compression: true,
            message_retention_hours: 24,
            enable_broadcast: true,
            broadcast_capacity: 1000,
            enable_p2p: true,
            enable_group: true,
            max_group_size: 10,
            enable_message_history: true,
            message_history_limit: 1000,
            enable_presence: true,
            presence_update_interval: 30,
        }
    }
}

/// Agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message ID
    pub id: String,
    /// Sender agent ID
    pub sender: String,
    /// Recipient agent ID(s)
    pub recipients: Vec<String>,
    /// Message type
    pub message_type: MessageType,
    /// Message content
    pub content: MessageContent,
    /// Message priority
    pub priority: MessagePriority,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message expiration
    pub expires_at: Option<DateTime<Utc>>,
    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Reply to message ID
    pub reply_to: Option<String>,
    /// Conversation ID
    pub conversation_id: Option<String>,
    /// Message status
    pub status: MessageStatus,
}

/// Message type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// Direct message to specific agent
    Direct,
    /// Broadcast message to all agents
    Broadcast,
    /// Group message
    Group,
    /// Task request
    TaskRequest,
    /// Task response
    TaskResponse,
    /// Information sharing
    Information,
    /// Query/Question
    Query,
    /// Response to query
    QueryResponse,
    /// Notification
    Notification,
    /// Status update
    StatusUpdate,
    /// Heartbeat/Ping
    Heartbeat,
    /// Error message
    Error,
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    /// Content type
    pub content_type: ContentType,
    /// Text content
    pub text: Option<String>,
    /// Structured data
    pub data: Option<serde_json::Value>,
    /// Binary data
    pub binary: Option<Vec<u8>>,
    /// Content encoding
    pub encoding: Option<String>,
    /// Content metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Content type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    /// Plain text
    Text,
    /// JSON data
    Json,
    /// XML data
    Xml,
    /// Binary data
    Binary,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
    /// Custom format
    Custom(String),
}

/// Message priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Message status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    /// Message pending
    Pending,
    /// Message sent
    Sent,
    /// Message delivered
    Delivered,
    /// Message read
    Read,
    /// Message failed
    Failed,
    /// Message expired
    Expired,
}

/// Conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Conversation ID
    pub id: String,
    /// Conversation participants
    pub participants: Vec<String>,
    /// Conversation type
    pub conversation_type: ConversationType,
    /// Conversation subject
    pub subject: Option<String>,
    /// Conversation messages
    pub messages: Vec<AgentMessage>,
    /// Conversation created time
    pub created_at: DateTime<Utc>,
    /// Conversation last activity
    pub last_activity: DateTime<Utc>,
    /// Conversation status
    pub status: ConversationStatus,
    /// Conversation metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Conversation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationType {
    /// Direct conversation (2 participants)
    Direct,
    /// Group conversation (3+ participants)
    Group,
    /// Task-based conversation
    TaskBased,
    /// Information sharing
    Information,
    /// Collaboration
    Collaboration,
}

/// Conversation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationStatus {
    /// Active conversation
    Active,
    /// Paused conversation
    Paused,
    /// Closed conversation
    Closed,
    /// Archived conversation
    Archived,
}

/// Agent presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPresence {
    /// Agent ID
    pub agent_id: String,
    /// Presence status
    pub status: PresenceStatus,
    /// Status message
    pub status_message: Option<String>,
    /// Current capabilities
    pub capabilities: Vec<String>,
    /// Current load
    pub load: f32,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Presence metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Presence status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresenceStatus {
    /// Agent is online and available
    Online,
    /// Agent is busy
    Busy,
    /// Agent is away
    Away,
    /// Agent is offline
    Offline,
    /// Agent is in do not disturb mode
    DoNotDisturb,
}

/// Message handler
pub type MessageHandler = Box<dyn Fn(AgentMessage) -> Result<Option<AgentMessage>, AIError> + Send + Sync>;

/// Communication manager
pub struct CommunicationManager {
    /// Configuration
    config: Arc<RwLock<CommunicationConfig>>,
    /// Message channels per agent
    agent_channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<AgentMessage>>>>,
    /// Broadcast channel
    broadcast_sender: broadcast::Sender<AgentMessage>,
    /// Active conversations
    conversations: Arc<RwLock<HashMap<String, Conversation>>>,
    /// Agent presence
    presence: Arc<RwLock<HashMap<String, AgentPresence>>>,
    /// Message history
    message_history: Arc<RwLock<Vec<AgentMessage>>>,
    /// Message handlers
    message_handlers: Arc<RwLock<HashMap<MessageType, MessageHandler>>>,
    /// Communication statistics
    stats: Arc<RwLock<CommunicationStats>>,
}

/// Communication statistics
#[derive(Debug, Default)]
pub struct CommunicationStats {
    /// Total messages sent
    total_messages_sent: u64,
    /// Total messages received
    total_messages_received: u64,
    /// Messages by type
    messages_by_type: HashMap<String, u64>,
    /// Active conversations
    active_conversations: u64,
    /// Total conversations
    total_conversations: u64,
    /// Online agents
    online_agents: u64,
    /// Average response time
    avg_response_time: f64,
    /// Last message time
    last_message_time: Option<DateTime<Utc>>,
}

impl CommunicationManager {
    /// Create a new communication manager
    pub async fn new(config: CommunicationConfig) -> Result<Self, AIError> {
        let (broadcast_sender, _) = broadcast::channel(config.broadcast_capacity);
        
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            agent_channels: Arc::new(RwLock::new(HashMap::new())),
            broadcast_sender,
            conversations: Arc::new(RwLock::new(HashMap::new())),
            presence: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CommunicationStats::default())),
        };
        
        info!("Communication manager initialized");
        Ok(manager)
    }
    
    /// Register an agent for communication
    pub async fn register_agent(&self, agent_id: String) -> Result<mpsc::UnboundedReceiver<AgentMessage>, AIError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Store agent channel
        self.agent_channels.write().insert(agent_id.clone(), sender);
        
        // Initialize presence
        let presence = AgentPresence {
            agent_id: agent_id.clone(),
            status: PresenceStatus::Online,
            status_message: None,
            capabilities: Vec::new(),
            load: 0.0,
            last_seen: Utc::now(),
            metadata: HashMap::new(),
        };
        
        self.presence.write().insert(agent_id.clone(), presence);
        
        // Update statistics
        self.stats.write().online_agents += 1;
        
        info!("Agent registered for communication: {}", agent_id);
        Ok(receiver)
    }
    
    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), AIError> {
        // Remove agent channel
        self.agent_channels.write().remove(agent_id);
        
        // Update presence
        if let Some(presence) = self.presence.write().get_mut(agent_id) {
            presence.status = PresenceStatus::Offline;
            presence.last_seen = Utc::now();
        }
        
        // Update statistics
        self.stats.write().online_agents = self.stats.write().online_agents.saturating_sub(1);
        
        info!("Agent unregistered from communication: {}", agent_id);
        Ok(())
    }
    
    /// Send a message
    pub async fn send_message(&self, message: AgentMessage) -> Result<(), AIError> {
        if !self.config.read().enabled {
            return Err(AIError::Configuration("Communication is disabled".to_string()));
        }
        
        // Validate message size
        let message_size = serde_json::to_string(&message)
            .map_err(|e| AIError::Serialization(e))?
            .len();
        
        if message_size > self.config.read().max_message_size {
            return Err(AIError::Configuration("Message too large".to_string()));
        }
        
        // Check expiration
        if let Some(expires_at) = message.expires_at {
            if expires_at < Utc::now() {
                return Err(AIError::Configuration("Message expired".to_string()));
            }
        }
        
        // Route message based on type
        match message.message_type {
            MessageType::Direct => self.send_direct_message(message).await?,
            MessageType::Broadcast => self.send_broadcast_message(message).await?,
            MessageType::Group => self.send_group_message(message).await?,
            _ => self.send_direct_message(message).await?,
        }
        
        Ok(())
    }
    
    /// Send a direct message
    async fn send_direct_message(&self, message: AgentMessage) -> Result<(), AIError> {
        let agent_channels = self.agent_channels.read();
        
        for recipient in &message.recipients {
            if let Some(sender) = agent_channels.get(recipient) {
                if let Err(e) = sender.send(message.clone()) {
                    log::warn!("Failed to send message to {}: {}", recipient, e);
                }
            } else {
                log::warn!("Agent not found: {}", recipient);
            }
        }
        
        // Store in history
        if self.config.read().enable_message_history {
            self.add_to_history(message.clone()).await;
        }
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_messages_sent += 1;
        stats.last_message_time = Some(message.timestamp);
        
        let message_type_key = format!("{:?}", message.message_type);
        stats.messages_by_type.entry(message_type_key).and_modify(|c| *c += 1).or_insert(1);
        
        debug!("Direct message sent: {} -> {:?}", message.id, message.recipients);
        Ok(())
    }
    
    /// Send a broadcast message
    async fn send_broadcast_message(&self, message: AgentMessage) -> Result<(), AIError> {
        if !self.config.read().enable_broadcast {
            return Err(AIError::Configuration("Broadcast messaging is disabled".to_string()));
        }
        
        if let Err(e) = self.broadcast_sender.send(message.clone()) {
            log::warn!("Failed to send broadcast message: {}", e);
        }
        
        // Store in history
        if self.config.read().enable_message_history {
            self.add_to_history(message.clone()).await;
        }
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_messages_sent += 1;
        stats.last_message_time = Some(message.timestamp);
        
        debug!("Broadcast message sent: {}", message.id);
        Ok(())
    }
    
    /// Send a group message
    async fn send_group_message(&self, message: AgentMessage) -> Result<(), AIError> {
        if !self.config.read().enable_group {
            return Err(AIError::Configuration("Group messaging is disabled".to_string()));
        }
        
        if message.recipients.len() > self.config.read().max_group_size {
            return Err(AIError::Configuration("Group too large".to_string()));
        }
        
        // Save message ID and recipients for logging
        let message_id = message.id.clone();
        let recipients = message.recipients.clone();
        
        // Send to all recipients
        self.send_direct_message(message).await?;
        
        debug!("Group message sent: {} -> {:?}", message_id, recipients);
        Ok(())
    }
    
    /// Create a conversation
    pub async fn create_conversation(&self, participants: Vec<String>, conversation_type: ConversationType) -> Result<String, AIError> {
        let conversation_id = Uuid::new_v4().to_string();
        
        let conversation = Conversation {
            id: conversation_id.clone(),
            participants,
            conversation_type,
            subject: None,
            messages: Vec::new(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            status: ConversationStatus::Active,
            metadata: HashMap::new(),
        };
        
        self.conversations.write().insert(conversation_id.clone(), conversation);
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_conversations += 1;
        stats.active_conversations += 1;
        
        info!("Conversation created: {}", conversation_id);
        Ok(conversation_id)
    }
    
    /// Add message to conversation
    pub async fn add_message_to_conversation(&self, conversation_id: &str, message: AgentMessage) -> Result<(), AIError> {
        if let Some(conversation) = self.conversations.write().get_mut(conversation_id) {
            conversation.messages.push(message);
            conversation.last_activity = Utc::now();
            
            // Limit conversation history
            let limit = self.config.read().message_history_limit;
            if conversation.messages.len() > limit {
                conversation.messages.remove(0);
            }
            
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Conversation not found: {}", conversation_id)))
        }
    }
    
    /// Get conversation
    pub async fn get_conversation(&self, conversation_id: &str) -> Option<Conversation> {
        self.conversations.read().get(conversation_id).cloned()
    }
    
    /// Update agent presence
    pub async fn update_presence(&self, agent_id: &str, status: PresenceStatus, status_message: Option<String>) -> Result<(), AIError> {
        if let Some(presence) = self.presence.write().get_mut(agent_id) {
            presence.status = status;
            presence.status_message = status_message;
            presence.last_seen = Utc::now();
            
            debug!("Agent presence updated: {} -> {:?}", agent_id, presence.status);
            Ok(())
        } else {
            Err(AIError::Configuration(format!("Agent not found: {}", agent_id)))
        }
    }
    
    /// Get agent presence
    pub async fn get_presence(&self, agent_id: &str) -> Option<AgentPresence> {
        self.presence.read().get(agent_id).cloned()
    }
    
    /// List online agents
    pub async fn list_online_agents(&self) -> Vec<String> {
        self.presence.read()
            .iter()
            .filter(|(_, presence)| matches!(presence.status, PresenceStatus::Online))
            .map(|(agent_id, _)| agent_id.clone())
            .collect()
    }
    
    /// Register message handler
    pub async fn register_message_handler<F>(&self, message_type: MessageType, handler: F) -> Result<(), AIError>
    where
        F: Fn(AgentMessage) -> Result<Option<AgentMessage>, AIError> + Send + Sync + 'static,
    {
        self.message_handlers.write().insert(message_type, Box::new(handler));
        Ok(())
    }
    
    /// Process message with handlers
    pub async fn process_message(&self, message: AgentMessage) -> Result<Option<AgentMessage>, AIError> {
        if let Some(handler) = self.message_handlers.read().get(&message.message_type) {
            handler(message)
        } else {
            Ok(None)
        }
    }
    
    /// Get message history
    pub async fn get_message_history(&self, limit: Option<usize>) -> Vec<AgentMessage> {
        let history = self.message_history.read();
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Clear message history
    pub async fn clear_message_history(&self) -> Result<(), AIError> {
        self.message_history.write().clear();
        info!("Message history cleared");
        Ok(())
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: CommunicationConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Communication configuration updated");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> CommunicationStats {
        self.stats.read().clone()
    }
    
    /// Add message to history
    async fn add_to_history(&self, message: AgentMessage) {
        let mut history = self.message_history.write();
        history.push(message);
        
        // Limit history size
        let limit = self.config.read().message_history_limit;
        if history.len() > limit {
            history.remove(0);
        }
    }
}

impl Clone for CommunicationStats {
    fn clone(&self) -> Self {
        Self {
            total_messages_sent: self.total_messages_sent,
            total_messages_received: self.total_messages_received,
            messages_by_type: self.messages_by_type.clone(),
            active_conversations: self.active_conversations,
            total_conversations: self.total_conversations,
            online_agents: self.online_agents,
            avg_response_time: self.avg_response_time,
            last_message_time: self.last_message_time,
        }
    }
}

/// Helper function to create a simple text message
pub fn create_text_message(
    sender: String,
    recipients: Vec<String>,
    text: String,
    message_type: MessageType,
) -> AgentMessage {
    AgentMessage {
        id: Uuid::new_v4().to_string(),
        sender,
        recipients,
        message_type,
        content: MessageContent {
            content_type: ContentType::Text,
            text: Some(text),
            data: None,
            binary: None,
            encoding: None,
            metadata: HashMap::new(),
        },
        priority: MessagePriority::Normal,
        timestamp: Utc::now(),
        expires_at: None,
        metadata: HashMap::new(),
        reply_to: None,
        conversation_id: None,
        status: MessageStatus::Pending,
    }
}

/// Helper function to create a data message
pub fn create_data_message(
    sender: String,
    recipients: Vec<String>,
    data: serde_json::Value,
    message_type: MessageType,
) -> AgentMessage {
    AgentMessage {
        id: Uuid::new_v4().to_string(),
        sender,
        recipients,
        message_type,
        content: MessageContent {
            content_type: ContentType::Json,
            text: None,
            data: Some(data),
            binary: None,
            encoding: None,
            metadata: HashMap::new(),
        },
        priority: MessagePriority::Normal,
        timestamp: Utc::now(),
        expires_at: None,
        metadata: HashMap::new(),
        reply_to: None,
        conversation_id: None,
        status: MessageStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_communication_config_default() {
        let config = CommunicationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_message_size, 1024 * 1024);
        assert_eq!(config.message_timeout, 30);
        assert_eq!(config.max_concurrent_conversations, 100);
        assert!(config.enable_broadcast);
        assert!(config.enable_p2p);
        assert!(config.enable_group);
    }
    
    #[test]
    fn test_create_text_message() {
        let message = create_text_message(
            "agent1".to_string(),
            vec!["agent2".to_string()],
            "Hello, world!".to_string(),
            MessageType::Direct,
        );
        
        assert_eq!(message.sender, "agent1");
        assert_eq!(message.recipients, vec!["agent2".to_string()]);
        assert!(matches!(message.message_type, MessageType::Direct));
        assert!(matches!(message.content.content_type, ContentType::Text));
        assert_eq!(message.content.text, Some("Hello, world!".to_string()));
    }
    
    #[test]
    fn test_create_data_message() {
        let data = serde_json::json!({"key": "value"});
        let message = create_data_message(
            "agent1".to_string(),
            vec!["agent2".to_string()],
            data.clone(),
            MessageType::TaskRequest,
        );
        
        assert_eq!(message.sender, "agent1");
        assert!(matches!(message.message_type, MessageType::TaskRequest));
        assert!(matches!(message.content.content_type, ContentType::Json));
        assert_eq!(message.content.data, Some(data));
    }
    
    #[test]
    fn test_agent_presence() {
        let presence = AgentPresence {
            agent_id: "agent1".to_string(),
            status: PresenceStatus::Online,
            status_message: Some("Available".to_string()),
            capabilities: vec!["reasoning".to_string(), "web_search".to_string()],
            load: 0.5,
            last_seen: Utc::now(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(presence.agent_id, "agent1");
        assert!(matches!(presence.status, PresenceStatus::Online));
        assert_eq!(presence.status_message, Some("Available".to_string()));
        assert_eq!(presence.capabilities.len(), 2);
        assert_eq!(presence.load, 0.5);
    }
    
    #[test]
    fn test_conversation_creation() {
        let conversation = Conversation {
            id: "conv1".to_string(),
            participants: vec!["agent1".to_string(), "agent2".to_string()],
            conversation_type: ConversationType::Direct,
            subject: Some("Test conversation".to_string()),
            messages: Vec::new(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            status: ConversationStatus::Active,
            metadata: HashMap::new(),
        };
        
        assert_eq!(conversation.id, "conv1");
        assert_eq!(conversation.participants.len(), 2);
        assert!(matches!(conversation.conversation_type, ConversationType::Direct));
        assert!(matches!(conversation.status, ConversationStatus::Active));
        assert_eq!(conversation.messages.len(), 0);
    }
    
    #[test]
    fn test_message_priority_ordering() {
        let mut priorities = vec![
            MessagePriority::Normal,
            MessagePriority::Critical,
            MessagePriority::Low,
            MessagePriority::High,
        ];
        
        priorities.sort();
        
        assert_eq!(priorities, vec![
            MessagePriority::Low,
            MessagePriority::Normal,
            MessagePriority::High,
            MessagePriority::Critical,
        ]);
    }
}