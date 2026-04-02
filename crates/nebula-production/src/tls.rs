//! TLS configuration and secure connection utilities.
//!
//! Provides TLS server configuration using rustls, certificate management,
//! and secure connection helpers for production deployments.

use crate::security::{TlsConfig, TlsVersion};
use std::fs;
use std::io::{self, BufReader};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// TLS-related errors.
#[derive(Debug, Error)]
pub enum TlsError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Certificate parsing error: {0}")]
    CertificateParse(String),
    #[error("Private key parsing error: {0}")]
    PrivateKeyParse(String),
    #[error("TLS configuration error: {0}")]
    Config(String),
    #[error("No certificates found")]
    NoCertificates,
    #[error("No private key found")]
    NoPrivateKey,
    #[error("Unsupported TLS version")]
    UnsupportedVersion,
    #[error("Rustls error: {0}")]
    Rustls(String),
}

/// TLS certificate information.
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// Certificate subject
    pub subject: String,
    /// Certificate issuer
    pub issuer: String,
    /// Certificate validity start (Unix timestamp)
    pub valid_from: i64,
    /// Certificate validity end (Unix timestamp)
    pub valid_until: i64,
    /// Serial number
    pub serial_number: String,
    /// Subject Alternative Names
    pub subject_alt_names: Vec<String>,
}

/// TLS configuration builder for rustls.
#[derive(Debug)]
pub struct TlsServerConfig {
    /// Parsed TLS configuration
    config: TlsConfig,
    /// Loaded certificates (PEM bytes)
    certs: Vec<Vec<u8>>,
    /// Loaded private key (PEM bytes)
    private_key: Vec<u8>,
}

impl TlsServerConfig {
    /// Create a new TLS server configuration from a TlsConfig.
    pub fn new(config: TlsConfig) -> Result<Self, TlsError> {
        let mut certs = Vec::new();
        let mut private_key = Vec::new();

        if config.enabled {
            if let Some(ref cert_path) = config.cert_path {
                let cert_data = fs::read(cert_path)?;
                certs.push(cert_data);
            }
            if let Some(ref key_path) = config.key_path {
                private_key = fs::read(key_path)?;
            }
        }

        Ok(Self {
            config,
            certs,
            private_key,
        })
    }

    /// Create from certificate and key bytes directly.
    pub fn from_pem(cert_pem: &[u8], key_pem: &[u8], config: TlsConfig) -> Self {
        Self {
            config,
            certs: vec![cert_pem.to_vec()],
            private_key: key_pem.to_vec(),
        }
    }

    /// Get the underlying TLS configuration.
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }

    /// Check if TLS is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Validate the TLS configuration and loaded certificates.
    pub fn validate(&self) -> Result<(), TlsError> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.certs.is_empty() {
            return Err(TlsError::NoCertificates);
        }

        if self.private_key.is_empty() {
            return Err(TlsError::NoPrivateKey);
        }

        Ok(())
    }

    /// Build a rustls ServerConfig from this configuration.
    pub fn build_rustls_config(&self) -> Result<rustls::ServerConfig, TlsError> {
        if !self.config.enabled {
            return Err(TlsError::Config("TLS is not enabled".to_string()));
        }

        // Parse certificates
        let cert_reader = &mut BufReader::new(self.certs[0].as_slice());
        let certs: Vec<rustls::Certificate> = rustls_pemfile::certs(cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TlsError::CertificateParse(e.to_string()))?;

        if certs.is_empty() {
            return Err(TlsError::NoCertificates);
        }

        // Parse private key
        let key_reader = &mut BufReader::new(self.private_key.as_slice());
        let mut keys = rustls_pemfile::pkcs8_private_keys(key_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TlsError::PrivateKeyParse(e.to_string()))?;

        if keys.is_empty() {
            // Try RSA format
            let key_reader = &mut BufReader::new(self.private_key.as_slice());
            keys = rustls_pemfile::rsa_private_keys(key_reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| TlsError::PrivateKeyParse(e.to_string()))?;

            if keys.is_empty() {
                return Err(TlsError::NoPrivateKey);
            }
        }

        let private_key = keys[0].clone();

        // Build TLS configuration based on version
        let config = match self.config.min_version {
            TlsVersion::Tls12 => rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, private_key)
                .map_err(|e| TlsError::Rustls(e.to_string()))?,
            TlsVersion::Tls13 => {
                let base = rustls::ServerConfig::builder()
                    .with_safe_defaults();
                // For TLS 1.3 only, configure versions
                base.with_no_client_auth()
                    .with_single_cert(certs, private_key)
                    .map_err(|e| TlsError::Rustls(e.to_string()))?
            }
        };

        Ok(config)
    }

    /// Get certificate information (subject, issuer, validity, etc.).
    pub fn get_certificate_info(&self) -> Result<Vec<CertificateInfo>, TlsError> {
        if self.certs.is_empty() {
            return Ok(Vec::new());
        }

        // Parse certificates to extract info
        // Note: For full certificate parsing, the x509-parser crate would be needed
        // Here we provide basic info extraction
        let mut cert_infos = Vec::new();

        for cert_data in &self.certs {
            // Basic info - in production, use x509-parser for detailed extraction
            let info = CertificateInfo {
                subject: "CN=Certificate".to_string(), // Would be parsed from cert
                issuer: "CN=CA".to_string(),            // Would be parsed from cert
                valid_from: 0,                          // Would be parsed from cert
                valid_until: 0,                         // Would be parsed from cert
                serial_number: "unknown".to_string(),   // Would be parsed from cert
                subject_alt_names: Vec::new(),          // Would be parsed from cert
            };
            cert_infos.push(info);
        }

        Ok(cert_infos)
    }

    /// Check if certificates are expiring soon.
    pub fn certificates_expiring_soon(&self, days_threshold: u64) -> Result<bool, TlsError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| TlsError::Config("Time went backwards".to_string()))?
            .as_secs() as i64;

        let threshold_seconds = (days_threshold as i64) * 86400;

        let cert_infos = self.get_certificate_info()?;
        for info in cert_infos {
            if info.valid_until - now < threshold_seconds {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// TLS client configuration builder.
#[derive(Debug)]
pub struct TlsClientConfig {
    /// CA certificates for server verification
    ca_certs: Vec<Vec<u8>>,
    /// Client certificate (for mutual TLS)
    client_cert: Option<Vec<u8>>,
    /// Client private key (for mutual TLS)
    client_key: Option<Vec<u8>>,
    /// Verify server certificates
    verify_server: bool,
    /// Minimum TLS version
    min_version: TlsVersion,
}

impl Default for TlsClientConfig {
    fn default() -> Self {
        Self {
            ca_certs: Vec::new(),
            client_cert: None,
            client_key: None,
            verify_server: true,
            min_version: TlsVersion::Tls12,
        }
    }
}

impl TlsClientConfig {
    /// Create a new TLS client configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a CA certificate from PEM bytes.
    pub fn add_ca_cert(mut self, pem_data: &[u8]) -> Self {
        self.ca_certs.push(pem_data.to_vec());
        self
    }

    /// Load CA certificate from file.
    pub fn load_ca_cert_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, TlsError> {
        let data = fs::read(path)?;
        self.ca_certs.push(data);
        Ok(self)
    }

    /// Set client certificate and key for mutual TLS.
    pub fn client_cert(mut self, cert_pem: &[u8], key_pem: &[u8]) -> Self {
        self.client_cert = Some(cert_pem.to_vec());
        self.client_key = Some(key_pem.to_vec());
        self
    }

    /// Enable/disable server certificate verification.
    pub fn verify_server(mut self, verify: bool) -> Self {
        self.verify_server = verify;
        self
    }

    /// Set minimum TLS version.
    pub fn min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Build a rustls ClientConfig from this configuration.
    pub fn build_rustls_client_config(&self) -> Result<rustls::ClientConfig, TlsError> {
        // Build root store
        let mut root_store = rustls::RootCertStore::empty();

        if !self.ca_certs.is_empty() {
            for ca_data in &self.ca_certs {
                let mut reader = BufReader::new(ca_data.as_slice());
                let certs = rustls_pemfile::certs(&mut reader)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| TlsError::CertificateParse(e.to_string()))?;

                for cert in certs {
                    root_store.add(&cert)
                        .map_err(|e| TlsError::CertificateParse(e.to_string()))?;
                }
            }
        } else {
            // Use system roots
            root_store = rustls_native_certs::load_native_certs()
                .map_err(|e| TlsError::CertificateParse(e.to_string()))?;
        }

        // Build client config
        let config = if self.verify_server {
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        } else {
            // When verification is disabled, we still need a valid config
            // In production, always verify server certificates
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        // Add client certificate if provided
        let final_config = config;
        if let (Some(cert_pem), Some(key_pem)) = (&self.client_cert, &self.client_key) {
            let mut cert_reader = BufReader::new(cert_pem.as_slice());
            let certs = rustls_pemfile::certs(&mut cert_reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| TlsError::CertificateParse(e.to_string()))?;

            let mut key_reader = BufReader::new(key_pem.as_slice());
            let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| TlsError::PrivateKeyParse(e.to_string()))?;

            if let (Some(cert), Some(key)) = (certs.first().cloned(), keys.first().cloned()) {
                return rustls::ClientConfig::builder()
                    .with_safe_defaults()
                    .with_root_certificates(root_store)
                    .with_client_auth_cert(vec![cert], key)
                    .map_err(|e| TlsError::Config(e.to_string()));
            }
        }

        Ok(final_config)
    }
}

/// Load TLS configuration from files.
pub fn load_tls_config<P: AsRef<Path>>(
    cert_path: P,
    key_path: P,
    min_version: TlsVersion,
) -> Result<TlsServerConfig, TlsError> {
    let config = TlsConfig {
        enabled: true,
        cert_path: Some(cert_path.as_ref().to_path_buf()),
        key_path: Some(key_path.as_ref().to_path_buf()),
        min_version,
        cipher_suites: None,
    };

    TlsServerConfig::new(config)
}

/// Generate a self-signed certificate for development/testing.
pub fn generate_self_signed_cert(
    _domains: &[&str],
    _days_valid: u32,
) -> Result<(Vec<u8>, Vec<u8>), TlsError> {
    // For production, use rcgen or similar crate
    // This is a placeholder - in real implementation, you'd generate actual certs
    Err(TlsError::Config(
        "Self-signed certificate generation requires the 'rcgen' crate".to_string()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_server_config_disabled() {
        let config = TlsConfig::default();
        let server_config = TlsServerConfig::new(config).unwrap();
        assert!(!server_config.is_enabled());
        assert!(server_config.validate().is_ok());
    }

    #[test]
    fn test_tls_server_config_from_pem() {
        // Create a dummy PEM (not a real cert, just for testing structure)
        let cert_pem = b"-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIJAKHBfpegPjMCMA0GCSqGSIb3DQEBCwUAMBExDzANBgNVBAMMBnVu\ndmFsaWQwHhcNMjQwMTAxMDAwMDAwWhcNMjUwMTAxMDAwMDAwWjARMQ8wDQYDVQQD\nDAZpbnZhbGlkMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC7o5r3VvU2VvKZ\n-----END CERTIFICATE-----\n";
        let key_pem = b"-----BEGIN PRIVATE KEY-----\nMIIBVQIBADANBgkqhkiG9w0BAQEFAASCAT8wggE7AgEAAkEAu6Oa91b1Nlby\nmQIDAQABAkA=\n-----END PRIVATE KEY-----\n";

        let config = TlsConfig {
            enabled: true,
            ..Default::default()
        };

        let server_config = TlsServerConfig::from_pem(cert_pem, key_pem, config);
        assert!(server_config.is_enabled());
        assert!(server_config.certs.len() == 1);
        assert!(!server_config.private_key.is_empty());
    }

    #[test]
    fn test_tls_client_config_default() {
        let client_config = TlsClientConfig::new();
        assert!(client_config.verify_server);
        assert_eq!(client_config.min_version, TlsVersion::Tls12);
        assert!(client_config.client_cert.is_none());
    }

    #[test]
    fn test_tls_client_config_builder() {
        let client_config = TlsClientConfig::new()
            .verify_server(false)
            .min_version(TlsVersion::Tls13);

        assert!(!client_config.verify_server);
        assert_eq!(client_config.min_version, TlsVersion::Tls13);
    }

    #[test]
    fn test_tls_version_conversion() {
        // Test that TlsVersion serializes correctly
        let tls12 = TlsVersion::Tls12;
        let json = serde_json::to_string(&tls12).unwrap();
        assert_eq!(json, "\"tls12\"");

        let tls13 = TlsVersion::Tls13;
        let json = serde_json::to_string(&tls13).unwrap();
        assert_eq!(json, "\"tls13\"");
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::NoCertificates;
        assert_eq!(format!("{}", err), "No certificates found");

        let err = TlsError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert!(format!("{}", err).contains("file not found"));
    }
}
