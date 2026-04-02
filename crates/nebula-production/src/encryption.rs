//! Encryption utilities for data at rest (AES-256) and in transit (TLS).
//!
//! Provides symmetric encryption, key derivation, and TLS configuration helpers.

use crate::security::{TlsConfig, TlsVersion};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Encryption errors.
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("TLS error: {0}")]
    Tls(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Encryption algorithm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum EncryptionAlgorithm {
    #[default]
    Aes256Gcm,
}

/// Encryption configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encryption algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Base64-encoded encryption key (32 bytes for AES-256)
    pub key: String,
    /// Key derivation function salt (for password-based key derivation)
    pub salt: Option<String>,
    /// Key version for rotation
    pub key_version: u32,
    /// Enable key rotation
    pub rotation_enabled: bool,
    /// Rotation interval in days
    pub rotation_interval_days: Option<u32>,
}

impl EncryptionConfig {
    /// Create a new encryption config with a random key.
    pub fn generate() -> Result<Self, EncryptionError> {
        let mut key_bytes = [0u8; 32];
        let rng = SystemRandom::new();
        rng.fill(&mut key_bytes)
            .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;

        Ok(Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key: base64::engine::general_purpose::STANDARD.encode(&key_bytes),
            salt: None,
            key_version: 1,
            rotation_enabled: false,
            rotation_interval_days: None,
        })
    }

    /// Validate the encryption configuration.
    pub fn validate(&self) -> Result<(), EncryptionError> {
        let engine = base64::engine::general_purpose::STANDARD;
        let key_bytes = engine
            .decode(&self.key)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;

        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                if key_bytes.len() != 32 {
                    return Err(EncryptionError::InvalidKey(format!(
                        "AES-256 requires 32-byte key, got {} bytes",
                        key_bytes.len()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the key as bytes.
    pub fn key_bytes(&self) -> Result<Vec<u8>, EncryptionError> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.key)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))
    }
}

/// Encrypted data with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext (base64 encoded)
    pub ciphertext: String,
    /// Nonce/IV used for encryption (base64 encoded)
    pub nonce: String,
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Key version used for encryption
    pub key_version: u32,
    /// Additional authenticated data (base64 encoded)
    pub aad: Option<String>,
}

/// Encryption engine for symmetric encryption.
pub struct EncryptionEngine {
    config: EncryptionConfig,
    key: LessSafeKey,
}

impl EncryptionEngine {
    /// Create a new encryption engine with the given configuration.
    pub fn new(config: EncryptionConfig) -> Result<Self, EncryptionError> {
        config.validate()?;

        let key_bytes = config.key_bytes()?;
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;
        let key = LessSafeKey::new(unbound_key);

        Ok(Self { config, key })
    }

    /// Encrypt data using AES-256-GCM.
    pub fn encrypt(&self, plaintext: &[u8], aad: Option<&[u8]>) -> Result<EncryptedData, EncryptionError> {
        let engine = base64::engine::general_purpose::STANDARD;
        
        // Generate random nonce (12 bytes for AES-GCM)
        let mut nonce_bytes = [0u8; 12];
        let rng = SystemRandom::new();
        rng.fill(&mut nonce_bytes)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        let nonce = ring::aead::Nonce::assume_unique_for_key(nonce_bytes);

        // Encrypt using seal_in_place_separate_tag
        let mut in_out = plaintext.to_vec();
        let aad_data = Aad::from(aad.unwrap_or(&[]));
        let tag = self.key
            .seal_in_place_separate_tag(nonce, aad_data, &mut in_out)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        // Combine ciphertext and tag
        let mut result = in_out;
        result.extend_from_slice(tag.as_ref());

        Ok(EncryptedData {
            ciphertext: engine.encode(&result),
            nonce: engine.encode(&nonce_bytes),
            algorithm: self.config.algorithm,
            key_version: self.config.key_version,
            aad: aad.map(|data| engine.encode(data)),
        })
    }

    /// Decrypt data using AES-256-GCM.
    pub fn decrypt(&self, encrypted: &EncryptedData, aad: Option<&[u8]>) -> Result<Vec<u8>, EncryptionError> {
        let engine = base64::engine::general_purpose::STANDARD;

        if encrypted.key_version != self.config.key_version {
            return Err(EncryptionError::Decryption(format!(
                "Key version mismatch: expected {}, got {}",
                self.config.key_version, encrypted.key_version
            )));
        }

        if encrypted.algorithm != self.config.algorithm {
            return Err(EncryptionError::Decryption(format!(
                "Algorithm mismatch: expected {:?}, got {:?}",
                self.config.algorithm, encrypted.algorithm
            )));
        }

        let ciphertext = engine
            .decode(&encrypted.ciphertext)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;
        let nonce_bytes = engine
            .decode(&encrypted.nonce)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

        if nonce_bytes.len() != 12 {
            return Err(EncryptionError::Decryption(format!(
                "Invalid nonce length: {} (expected 12)",
                nonce_bytes.len()
            )));
        }

        let nonce = ring::aead::Nonce::assume_unique_for_key(
            nonce_bytes.try_into().map_err(|_| {
                EncryptionError::Decryption("Failed to convert nonce to array".to_string())
            })?,
        );

        let aad_data = Aad::from(aad.unwrap_or(&[]));

        // Decrypt in place - open_in_place returns the plaintext portion
        let mut buffer = ciphertext;
        let plaintext = self
            .key
            .open_in_place(nonce, aad_data, &mut buffer)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

        // The returned slice is the plaintext (ring handles the tag internally)
        Ok(plaintext.to_vec())
    }

    /// Encrypt a string and return base64-encoded encrypted data.
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, EncryptionError> {
        let encrypted = self.encrypt(plaintext.as_bytes(), None)?;
        serde_json::to_string(&encrypted).map_err(|e| EncryptionError::Encryption(e.to_string()))
    }

    /// Decrypt a base64-encoded encrypted string.
    pub fn decrypt_string(&self, encrypted_json: &str) -> Result<String, EncryptionError> {
        let encrypted: EncryptedData =
            serde_json::from_str(encrypted_json).map_err(|e| EncryptionError::Decryption(e.to_string()))?;
        let plaintext = self.decrypt(&encrypted, None)?;
        String::from_utf8(plaintext).map_err(|e| EncryptionError::Decryption(e.to_string()))
    }

    /// Get the configuration.
    pub fn config(&self) -> &EncryptionConfig {
        &self.config
    }
}

/// TLS configuration builder.
pub struct TlsConfigBuilder {
    cert_path: Option<String>,
    key_path: Option<String>,
    min_version: TlsVersion,
    ca_path: Option<String>,
    client_auth: bool,
}

impl Default for TlsConfigBuilder {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            min_version: TlsVersion::Tls12,
            ca_path: None,
            client_auth: false,
        }
    }
}

impl TlsConfigBuilder {
    /// Create a new TLS config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the certificate path.
    pub fn cert_path(mut self, path: &str) -> Self {
        self.cert_path = Some(path.to_string());
        self
    }

    /// Set the private key path.
    pub fn key_path(mut self, path: &str) -> Self {
        self.key_path = Some(path.to_string());
        self
    }

    /// Set the minimum TLS version.
    pub fn min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Set the CA certificate path for client verification.
    pub fn ca_path(mut self, path: &str) -> Self {
        self.ca_path = Some(path.to_string());
        self.client_auth = true;
        self
    }

    /// Build the TLS configuration.
    pub fn build(self) -> Result<TlsConfig, EncryptionError> {
        Ok(TlsConfig {
            enabled: true,
            cert_path: self.cert_path.map(PathBuf::from),
            key_path: self.key_path.map(PathBuf::from),
            min_version: self.min_version,
            cipher_suites: None,
        })
    }
}

/// Generate a secure random key.
pub fn generate_random_key() -> Result<Vec<u8>, EncryptionError> {
    let mut key = [0u8; 32];
    let rng = SystemRandom::new();
    rng.fill(&mut key)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(key.to_vec())
}

/// Generate a secure random nonce.
pub fn generate_nonce() -> Result<Vec<u8>, EncryptionError> {
    let mut nonce = [0u8; 12];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(nonce.to_vec())
}

/// Derive a key from a password using PBKDF2.
pub fn derive_key_from_password(
    password: &str,
    salt: &[u8],
    iterations: u32,
) -> Result<Vec<u8>, EncryptionError> {
    use ring::pbkdf2::{derive, PBKDF2_HMAC_SHA256};

    let mut key = [0u8; 32];
    derive(
        PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(iterations).ok_or_else(|| {
            EncryptionError::KeyDerivation("Invalid iteration count".to_string())
        })?,
        salt,
        password.as_bytes(),
        &mut key,
    );

    Ok(key.to_vec())
}

/// Compute HMAC-SHA256 for data integrity verification.
pub fn compute_hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    use ring::hmac::{sign, Key as HmacKey};

    let hmac_key = HmacKey::new(ring::hmac::HMAC_SHA256, key);
    let signature = sign(&hmac_key, data);
    signature.as_ref().to_vec()
}

/// Verify HMAC-SHA256 signature.
pub fn verify_hmac(key: &[u8], data: &[u8], signature: &[u8]) -> bool {
    use ring::hmac::{Key as HmacKey, verify};

    let hmac_key = HmacKey::new(ring::hmac::HMAC_SHA256, key);
    verify(&hmac_key, data, signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config_generate() {
        let config = EncryptionConfig::generate().unwrap();
        assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert_eq!(config.key_version, 1);
        assert!(!config.rotation_enabled);
    }

    #[test]
    fn test_encryption_config_validate() {
        let config = EncryptionConfig::generate().unwrap();
        assert!(config.validate().is_ok());

        let invalid_config = EncryptionConfig {
            key: "short".to_string(),
            ..config.clone()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let config = EncryptionConfig::generate().unwrap();
        let engine = EncryptionEngine::new(config).unwrap();

        let plaintext = b"Hello, World!";
        let encrypted = engine.encrypt(plaintext, None).unwrap();
        let decrypted = engine.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_encryption_with_aad() {
        let config = EncryptionConfig::generate().unwrap();
        let engine = EncryptionEngine::new(config).unwrap();

        let plaintext = b"Secret message";
        let aad = b"additional data";

        let encrypted = engine.encrypt(plaintext, Some(aad)).unwrap();
        let decrypted = engine.decrypt(&encrypted, Some(aad)).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());

        let wrong_aad = b"wrong data";
        assert!(engine.decrypt(&encrypted, Some(wrong_aad)).is_err());
    }

    #[test]
    fn test_string_encryption_roundtrip() {
        let config = EncryptionConfig::generate().unwrap();
        let engine = EncryptionEngine::new(config).unwrap();

        let plaintext = "Hello, World! This is a test string.";
        let encrypted = engine.encrypt_string(plaintext).unwrap();
        let decrypted = engine.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_random_key_generation() {
        let key1 = generate_random_key().unwrap();
        let key2 = generate_random_key().unwrap();

        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hmac_compute_and_verify() {
        let key = generate_random_key().unwrap();
        let data = b"Hello, World!";

        let signature = compute_hmac(&key, data);
        assert!(verify_hmac(&key, data, &signature));
        assert!(!verify_hmac(&key, b"different data", &signature));
    }

    #[test]
    fn test_key_derivation() {
        let password = "my_secure_password";
        let salt = b"random_salt_1234";
        let iterations = 100_000;

        let key = derive_key_from_password(password, salt, iterations).unwrap();
        assert_eq!(key.len(), 32);

        let key2 = derive_key_from_password(password, salt, iterations).unwrap();
        assert_eq!(key, key2);

        let key3 = derive_key_from_password("different_password", salt, iterations).unwrap();
        assert_ne!(key, key3);
    }

    #[test]
    fn test_tls_config_builder() {
        let tls_config = TlsConfigBuilder::new()
            .cert_path("/path/to/cert.pem")
            .key_path("/path/to/key.pem")
            .min_version(TlsVersion::Tls13)
            .build()
            .unwrap();

        assert!(tls_config.enabled);
        assert!(tls_config.cert_path.is_some());
        assert!(tls_config.key_path.is_some());
        assert_eq!(tls_config.min_version, TlsVersion::Tls13);
    }

    #[test]
    fn test_encrypted_data_serialization() {
        let encrypted = EncryptedData {
            ciphertext: "abc123".to_string(),
            nonce: "xyz789".to_string(),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_version: 1,
            aad: None,
        };

        let json = serde_json::to_string(&encrypted).unwrap();
        let deserialized: EncryptedData = serde_json::from_str(&json).unwrap();

        assert_eq!(encrypted.ciphertext, deserialized.ciphertext);
        assert_eq!(encrypted.nonce, deserialized.nonce);
        assert_eq!(encrypted.algorithm, deserialized.algorithm);
        assert_eq!(encrypted.key_version, deserialized.key_version);
    }
}
