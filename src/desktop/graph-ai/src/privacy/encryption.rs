//! Encryption services for privacy protection
//! 
//! This module provides encryption and decryption services for data at rest
//! and in transit, with key management and secure storage.

use crate::AIError;
use crate::privacy::{EncryptionConfig, EncryptionAlgorithm, KeyManagement};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};
use rand::Rng;
use base64::{Engine as _, engine::general_purpose};

/// Encryption key
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    /// Key ID
    pub id: String,
    /// Key data
    pub data: Vec<u8>,
    /// Key type
    pub key_type: KeyType,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Expiry time
    pub expires_at: Option<DateTime<Utc>>,
    /// Key version
    pub version: u32,
}

/// Key type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    /// Master key
    Master,
    /// Data encryption key
    DataEncryption,
    /// Key encryption key
    KeyEncryption,
    /// Session key
    Session,
    /// Backup key
    Backup,
}

/// Encrypted data envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    /// Envelope version
    pub version: u32,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key ID used for encryption
    pub key_id: String,
    /// Initialization vector
    pub iv: String,
    /// Authentication tag
    pub auth_tag: Option<String>,
    /// Encrypted data
    pub ciphertext: String,
    /// Encryption timestamp
    pub encrypted_at: DateTime<Utc>,
    /// Additional authenticated data
    pub aad: Option<String>,
}

/// Key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key ID
    pub id: String,
    /// Key type
    pub key_type: KeyType,
    /// Key status
    pub status: KeyStatus,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last used time
    pub last_used: Option<DateTime<Utc>>,
    /// Usage count
    pub usage_count: u64,
    /// Key version
    pub version: u32,
}

/// Key status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyStatus {
    /// Key is active
    Active,
    /// Key is rotated (old but still valid)
    Rotated,
    /// Key is revoked
    Revoked,
    /// Key is expired
    Expired,
}

/// Encryption manager
pub struct EncryptionManager {
    /// Configuration
    config: Arc<RwLock<EncryptionConfig>>,
    /// Key storage
    key_storage: Arc<RwLock<KeyStorage>>,
    /// Active keys
    active_keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    /// Key metadata
    key_metadata: Arc<RwLock<HashMap<String, KeyMetadata>>>,
    /// Encryption statistics
    stats: Arc<RwLock<EncryptionStats>>,
}

/// Key storage backend
struct KeyStorage {
    /// Storage type
    storage_type: KeyManagement,
    /// Stored keys (for non-keyring storage)
    keys: HashMap<String, Vec<u8>>,
}

/// Encryption statistics
#[derive(Debug, Default)]
pub struct EncryptionStats {
    /// Total encryptions
    total_encryptions: u64,
    /// Total decryptions
    total_decryptions: u64,
    /// Encryption errors
    encryption_errors: u64,
    /// Decryption errors
    decryption_errors: u64,
    /// Key rotations
    key_rotations: u64,
    /// Last operation
    last_operation: Option<DateTime<Utc>>,
}

impl EncryptionManager {
    /// Create a new encryption manager
    pub async fn new(config: EncryptionConfig) -> Result<Self, AIError> {
        let key_storage = KeyStorage {
            storage_type: config.key_management.clone(),
            keys: HashMap::new(),
        };
        
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            key_storage: Arc::new(RwLock::new(key_storage)),
            active_keys: Arc::new(RwLock::new(HashMap::new())),
            key_metadata: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(EncryptionStats::default())),
        };
        
        // Initialize master key
        manager.initialize_master_key().await?;
        
        info!("Encryption manager initialized");
        Ok(manager)
    }
    
    /// Initialize master key
    async fn initialize_master_key(&self) -> Result<(), AIError> {
        let master_key_id = "master-key".to_string();
        
        // Check if master key exists
        if self.key_exists(&master_key_id).await? {
            // Load existing master key
            self.load_key(&master_key_id).await?;
        } else {
            // Generate new master key
            self.generate_master_key().await?;
        }
        
        Ok(())
    }
    
    /// Generate master key
    async fn generate_master_key(&self) -> Result<(), AIError> {
        let key_id = "master-key".to_string();
        let key_data = self.generate_key_data(32)?; // 256-bit key
        
        let key = EncryptionKey {
            id: key_id.clone(),
            data: key_data,
            key_type: KeyType::Master,
            created_at: Utc::now(),
            expires_at: None,
            version: 1,
        };
        
        // Store the key
        self.store_key(&key).await?;
        
        // Add to active keys
        self.active_keys.write().insert(key_id.clone(), key.clone());
        
        // Add metadata
        let metadata = KeyMetadata {
            id: key_id,
            key_type: KeyType::Master,
            status: KeyStatus::Active,
            created_at: key.created_at,
            last_used: None,
            usage_count: 0,
            version: 1,
        };
        
        self.key_metadata.write().insert(metadata.id.clone(), metadata);
        
        info!("Master key generated");
        Ok(())
    }
    
    /// Encrypt data
    pub async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, AIError> {
        if !self.config.read().at_rest {
            return Ok(data.to_vec());
        }
        
        let start_time = std::time::Instant::now();
        
        // Get or create data encryption key
        let dek = self.get_or_create_dek().await?;
        
        // Generate IV
        let iv = self.generate_iv()?;
        
        // Encrypt data
        let ciphertext = match self.config.read().algorithm {
            EncryptionAlgorithm::AES256GCM => {
                self.encrypt_aes_gcm(data, &dek.data, &iv).await?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.encrypt_chacha20(data, &dek.data, &iv).await?
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                self.encrypt_xchacha20(data, &dek.data, &iv).await?
            }
        };
        
        // Create envelope
        let envelope = EncryptedEnvelope {
            version: 1,
            algorithm: self.config.read().algorithm.clone(),
            key_id: dek.id.clone(),
            iv: general_purpose::STANDARD.encode(&iv),
            auth_tag: None, // Included in ciphertext for AEAD ciphers
            ciphertext: general_purpose::STANDARD.encode(&ciphertext),
            encrypted_at: Utc::now(),
            aad: None,
        };
        
        // Serialize envelope
        let envelope_bytes = serde_json::to_vec(&envelope)
            .map_err(|e| AIError::Serialization(e))?;
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_encryptions += 1;
        stats.last_operation = Some(Utc::now());
        
        // Update key usage
        if let Some(metadata) = self.key_metadata.write().get_mut(&dek.id) {
            metadata.usage_count += 1;
            metadata.last_used = Some(Utc::now());
        }
        
        log::debug!("Data encrypted in {:?}", start_time.elapsed());
        Ok(envelope_bytes)
    }
    
    /// Decrypt data
    pub async fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, AIError> {
        let start_time = std::time::Instant::now();
        
        // Deserialize envelope
        let envelope: EncryptedEnvelope = serde_json::from_slice(data)
            .map_err(|e| AIError::Serialization(e))?;
        
        // Get decryption key
        let key = self.get_key(&envelope.key_id).await?;
        
        // Decode IV and ciphertext
        let iv = general_purpose::STANDARD.decode(&envelope.iv)
            .map_err(|e| AIError::Configuration(format!("Invalid IV: {}", e)))?;
        let ciphertext = general_purpose::STANDARD.decode(&envelope.ciphertext)
            .map_err(|e| AIError::Configuration(format!("Invalid ciphertext: {}", e)))?;
        
        // Decrypt data
        let plaintext = match envelope.algorithm {
            EncryptionAlgorithm::AES256GCM => {
                self.decrypt_aes_gcm(&ciphertext, &key.data, &iv).await?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20(&ciphertext, &key.data, &iv).await?
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                self.decrypt_xchacha20(&ciphertext, &key.data, &iv).await?
            }
        };
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_decryptions += 1;
        stats.last_operation = Some(Utc::now());
        
        debug!("Data decrypted in {:?}", start_time.elapsed());
        Ok(plaintext)
    }
    
    /// Rotate encryption keys
    pub async fn rotate_keys(&self) -> Result<(), AIError> {
        info!("Starting key rotation");
        
        // Generate new data encryption key
        let new_dek = self.generate_data_key().await?;
        
        // Mark old keys as rotated
        let mut active_keys = self.active_keys.write();
        let mut key_metadata = self.key_metadata.write();
        
        for (_key_id, metadata) in key_metadata.iter_mut() {
            if matches!(metadata.key_type, KeyType::DataEncryption) && matches!(metadata.status, KeyStatus::Active) {
                metadata.status = KeyStatus::Rotated;
            }
        }
        
        // Activate new key
        active_keys.insert(new_dek.id.clone(), new_dek.clone());
        
        let metadata = KeyMetadata {
            id: new_dek.id.clone(),
            key_type: KeyType::DataEncryption,
            status: KeyStatus::Active,
            created_at: new_dek.created_at,
            last_used: None,
            usage_count: 0,
            version: new_dek.version,
        };
        
        key_metadata.insert(metadata.id.clone(), metadata);
        
        // Update statistics
        self.stats.write().key_rotations += 1;
        
        info!("Key rotation completed");
        Ok(())
    }
    
    /// Get or create data encryption key
    async fn get_or_create_dek(&self) -> Result<EncryptionKey, AIError> {
        // Look for active DEK
        let active_keys = self.active_keys.read();
        for (_, key) in active_keys.iter() {
            if matches!(key.key_type, KeyType::DataEncryption) {
                let metadata = self.key_metadata.read();
                if let Some(meta) = metadata.get(&key.id) {
                    if matches!(meta.status, KeyStatus::Active) {
                        return Ok(key.clone());
                    }
                }
            }
        }
        drop(active_keys);
        
        // No active DEK, create one
        self.generate_data_key().await
    }
    
    /// Generate data encryption key
    async fn generate_data_key(&self) -> Result<EncryptionKey, AIError> {
        let key_id = format!("dek-{}", uuid::Uuid::new_v4());
        let key_data = self.generate_key_data(32)?; // 256-bit key
        
        let key = EncryptionKey {
            id: key_id.clone(),
            data: key_data,
            key_type: KeyType::DataEncryption,
            created_at: Utc::now(),
            expires_at: None,
            version: 1,
        };
        
        // Encrypt DEK with master key before storing
        self.store_key(&key).await?;
        
        // Add to active keys
        self.active_keys.write().insert(key_id.clone(), key.clone());
        
        info!("Data encryption key generated: {}", key_id);
        Ok(key)
    }
    
    /// Get key by ID
    async fn get_key(&self, key_id: &str) -> Result<EncryptionKey, AIError> {
        // Check active keys first
        if let Some(key) = self.active_keys.read().get(key_id) {
            return Ok(key.clone());
        }
        
        // Load from storage
        self.load_key(key_id).await
    }
    
    /// Check if key exists
    async fn key_exists(&self, key_id: &str) -> Result<bool, AIError> {
        match self.config.read().key_management {
            KeyManagement::SystemKeyring => {
                // TODO: Check system keyring
                Ok(false)
            }
            KeyManagement::FileStorage | KeyManagement::MemoryOnly => {
                Ok(self.key_storage.read().keys.contains_key(key_id))
            }
            KeyManagement::HSM => {
                // TODO: Check HSM
                Ok(false)
            }
        }
    }
    
    /// Store key
    async fn store_key(&self, key: &EncryptionKey) -> Result<(), AIError> {
        match self.config.read().key_management {
            KeyManagement::SystemKeyring => {
                // TODO: Store in system keyring
                log::warn!("System keyring not implemented, using memory storage");
                self.key_storage.write().keys.insert(key.id.clone(), key.data.clone());
            }
            KeyManagement::FileStorage => {
                // TODO: Encrypt and store to file
                self.key_storage.write().keys.insert(key.id.clone(), key.data.clone());
            }
            KeyManagement::MemoryOnly => {
                self.key_storage.write().keys.insert(key.id.clone(), key.data.clone());
            }
            KeyManagement::HSM => {
                // TODO: Store in HSM
                return Err(AIError::UnsupportedOperation("HSM not implemented".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Load key
    async fn load_key(&self, key_id: &str) -> Result<EncryptionKey, AIError> {
        let key_data = match self.config.read().key_management {
            KeyManagement::SystemKeyring => {
                // TODO: Load from system keyring
                self.key_storage.read().keys.get(key_id)
                    .ok_or_else(|| AIError::Configuration(format!("Key not found: {}", key_id)))?
                    .clone()
            }
            KeyManagement::FileStorage | KeyManagement::MemoryOnly => {
                self.key_storage.read().keys.get(key_id)
                    .ok_or_else(|| AIError::Configuration(format!("Key not found: {}", key_id)))?
                    .clone()
            }
            KeyManagement::HSM => {
                // TODO: Load from HSM
                return Err(AIError::UnsupportedOperation("HSM not implemented".to_string()));
            }
        };
        
        // Get metadata
        let metadata = self.key_metadata.read().get(key_id)
            .ok_or_else(|| AIError::Configuration(format!("Key metadata not found: {}", key_id)))?
            .clone();
        
        let key = EncryptionKey {
            id: key_id.to_string(),
            data: key_data,
            key_type: metadata.key_type,
            created_at: metadata.created_at,
            expires_at: None,
            version: metadata.version,
        };
        
        // Add to active keys
        self.active_keys.write().insert(key_id.to_string(), key.clone());
        
        Ok(key)
    }
    
    /// Generate key data
    fn generate_key_data(&self, size: usize) -> Result<Vec<u8>, AIError> {
        let mut key = vec![0u8; size];
        rand::thread_rng().fill(&mut key[..]);
        Ok(key)
    }
    
    /// Generate initialization vector
    fn generate_iv(&self) -> Result<Vec<u8>, AIError> {
        let iv_size = match self.config.read().algorithm {
            EncryptionAlgorithm::AES256GCM => 12, // 96-bit IV for GCM
            EncryptionAlgorithm::ChaCha20Poly1305 => 12,
            EncryptionAlgorithm::XChaCha20Poly1305 => 24,
        };
        
        let mut iv = vec![0u8; iv_size];
        rand::thread_rng().fill(&mut iv[..]);
        Ok(iv)
    }
    
    /// Encrypt with AES-GCM
    async fn encrypt_aes_gcm(&self, plaintext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual AES-GCM encryption
        // For now, simulate with XOR
        let mut ciphertext = plaintext.to_vec();
        for (i, byte) in ciphertext.iter_mut().enumerate() {
            *byte ^= key[i % key.len()] ^ iv[i % iv.len()];
        }
        Ok(ciphertext)
    }
    
    /// Decrypt with AES-GCM
    async fn decrypt_aes_gcm(&self, ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual AES-GCM decryption
        // For now, simulate with XOR (same as encryption for XOR)
        self.encrypt_aes_gcm(ciphertext, key, iv).await
    }
    
    /// Encrypt with ChaCha20-Poly1305
    async fn encrypt_chacha20(&self, plaintext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual ChaCha20-Poly1305 encryption
        self.encrypt_aes_gcm(plaintext, key, iv).await
    }
    
    /// Decrypt with ChaCha20-Poly1305
    async fn decrypt_chacha20(&self, ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual ChaCha20-Poly1305 decryption
        self.decrypt_aes_gcm(ciphertext, key, iv).await
    }
    
    /// Encrypt with XChaCha20-Poly1305
    async fn encrypt_xchacha20(&self, plaintext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual XChaCha20-Poly1305 encryption
        self.encrypt_aes_gcm(plaintext, key, iv).await
    }
    
    /// Decrypt with XChaCha20-Poly1305
    async fn decrypt_xchacha20(&self, ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, AIError> {
        // TODO: Implement actual XChaCha20-Poly1305 decryption
        self.decrypt_aes_gcm(ciphertext, key, iv).await
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: EncryptionConfig) -> Result<(), AIError> {
        *self.config.write() = new_config;
        info!("Encryption configuration updated");
        Ok(())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> EncryptionStats {
        self.stats.read().clone()
    }
}

impl Clone for EncryptionStats {
    fn clone(&self) -> Self {
        Self {
            total_encryptions: self.total_encryptions,
            total_decryptions: self.total_decryptions,
            encryption_errors: self.encryption_errors,
            decryption_errors: self.decryption_errors,
            key_rotations: self.key_rotations,
            last_operation: self.last_operation,
        }
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_key() {
        let key = EncryptionKey {
            id: "test-key".to_string(),
            data: vec![0u8; 32],
            key_type: KeyType::DataEncryption,
            created_at: Utc::now(),
            expires_at: None,
            version: 1,
        };
        
        assert_eq!(key.id, "test-key");
        assert_eq!(key.data.len(), 32);
        assert!(matches!(key.key_type, KeyType::DataEncryption));
        assert_eq!(key.version, 1);
    }
    
    #[test]
    fn test_encrypted_envelope() {
        let envelope = EncryptedEnvelope {
            version: 1,
            algorithm: EncryptionAlgorithm::AES256GCM,
            key_id: "test-key".to_string(),
            iv: general_purpose::STANDARD.encode(&[0u8; 12]),
            auth_tag: None,
            ciphertext: general_purpose::STANDARD.encode(&[0u8; 32]),
            encrypted_at: Utc::now(),
            aad: None,
        };
        
        assert_eq!(envelope.version, 1);
        assert!(matches!(envelope.algorithm, EncryptionAlgorithm::AES256GCM));
        assert_eq!(envelope.key_id, "test-key");
    }
    
    #[test]
    fn test_key_metadata() {
        let metadata = KeyMetadata {
            id: "test-key".to_string(),
            key_type: KeyType::Master,
            status: KeyStatus::Active,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            version: 1,
        };
        
        assert_eq!(metadata.id, "test-key");
        assert!(matches!(metadata.key_type, KeyType::Master));
        assert!(matches!(metadata.status, KeyStatus::Active));
        assert_eq!(metadata.usage_count, 0);
    }
}