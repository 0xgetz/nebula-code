//! Authentication middleware and authorization.
//!
//! This module provides comprehensive authentication and authorization capabilities:
//!
//! # Features
//!
//! - **Authentication Middleware** - Support for API keys, JWT tokens, and basic auth
//! - **RBAC Authorization** - Role-based access control with role inheritance
//! - **Password Hashing** - Argon2-based secure password hashing and verification
//! - **JWT Token Management** - Token generation, validation, and claims handling
//! - **In-Memory Authorization** - Fast in-memory permission caching
//!
//! # Quick Start
//!
//! ```rust
//! use nebula_production::{
//!     RbacConfig, RoleDefinition, InMemoryAuthorization, Authorization,
//!     AuthMiddleware, ApiKeyConfig, JwtConfig, JwtAlgorithm,
//! };
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! // Set up RBAC
//! let mut roles = HashMap::new();
//! roles.insert("admin".to_string(), RoleDefinition {
//!     name: "admin".to_string(),
//!     description: "Administrator".to_string(),
//!     permissions: vec!["*".to_string()],
//!     inherits: vec![],
//! });
//! roles.insert("user".to_string(), RoleDefinition {
//!     name: "user".to_string(),
//!     description: "Regular user".to_string(),
//!     permissions: vec!["read:self".to_string()],
//!     inherits: vec![],
//! });
//!
//! let rbac = RbacConfig {
//!     roles,
//!     permissions: HashMap::new(),
//!     default_role: "user".to_string(),
//!     admin_role: "admin".to_string(),
//! };
//!
//! let auth: Arc<dyn Authorization> = Arc::new(InMemoryAuthorization::new(rbac));
//! ```
//!
//! # Password Hashing
//!
//! ```rust
//! use nebula_production::{hash_password, verify_password};
//!
//! let password = "my-secure-password";
//! let hash = hash_password(password).expect("Failed to hash password");
//!
//! // Verify the password
//! assert!(verify_password(password, &hash).expect("Verification failed"));
//! assert!(!verify_password("wrong-password", &hash).expect("Verification failed"));
//! ```
//!
//! # JWT Token Generation
//!
//! ```rust,ignore
//! use nebula_production::{AuthMiddleware, JwtConfig, JwtAlgorithm};
//!
//! let jwt_config = JwtConfig {
//!     enabled: true,
//!     algorithm: JwtAlgorithm::Hs256,
//!     secret: Some(base64::encode("your-secret-key")),
//!     issuer: Some("my-service".to_string()),
//!     ..Default::default()
//! };
//!
//! // Create auth middleware (requires authorization implementation)
//! // let middleware = AuthMiddleware::new(None, Some(jwt_config), auth)?;
//!
//! // Generate a token for a user
//! // let token = middleware.generate_jwt("user-123", &["user".to_string()], 3600)?;
//! ```
//!
//! # Role Inheritance
//!
//! Roles can inherit permissions from other roles:
//!
//! ```rust
//! use nebula_production::{RbacConfig, RoleDefinition, InMemoryAuthorization};
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! let mut roles = HashMap::new();
//! roles.insert("moderator".to_string(), RoleDefinition {
//!     name: "moderator".to_string(),
//!     description: "Moderator with user permissions".to_string(),
//!     permissions: vec!["delete:posts".to_string()],
//!     inherits: vec!["user".to_string()], // Inherits from user
//! });
//! roles.insert("user".to_string(), RoleDefinition {
//!     name: "user".to_string(),
//!     description: "Regular user".to_string(),
//!     permissions: vec!["read:posts".to_string()],
//!     inherits: vec![],
//! });
//!
//! let rbac = RbacConfig {
//!     roles,
//!     permissions: HashMap::new(),
//!     default_role: "user".to_string(),
//!     admin_role: "admin".to_string(),
//! };
//!
//! let auth = InMemoryAuthorization::new(rbac);
//! // A user with "moderator" role will also have "user" permissions
//! ```


use crate::security::{ApiKeyConfig, JwtConfig, SecurityError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use argon2::password_hash::rand_core::OsRng;
use thiserror::Error;
use tokio::sync::RwLock;

/// Authentication errors.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Authentication required")]
    AuthenticationRequired,
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Token invalid: {0}")]
    TokenInvalid(String),
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("User not found")]
    UserNotFound,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
}

/// Authentication result.
#[derive(Debug, Clone)]
pub struct AuthenticationResult {
    /// User ID
    pub user_id: String,
    /// User roles
    pub roles: Vec<String>,
    /// User metadata
    pub metadata: HashMap<String, JsonValue>,
    /// Authentication method used
    pub method: AuthMethod,
    /// Token expiration time (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
}

/// Authentication method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// API key authentication
    ApiKey,
    /// JWT token authentication
    Jwt,
    /// Basic authentication
    Basic,
    /// OAuth2 token
    OAuth2,
    /// Custom authentication
    Custom(String),
}

/// Authorization trait for checking permissions.
#[async_trait]
pub trait Authorization: Send + Sync + 'static {
    /// Check if the authenticated user has the required permission.
    async fn has_permission(&self, user_id: &str, permission: &str) -> bool;

    /// Check if the authenticated user has any of the required roles.
    async fn has_role(&self, user_id: &str, role: &str) -> bool;

    /// Check if the authenticated user has all required roles.
    async fn has_all_roles(&self, user_id: &str, roles: &[&str]) -> bool;

    /// Get user permissions.
    async fn get_permissions(&self, user_id: &str) -> Vec<String>;

    /// Get user roles.
    async fn get_roles(&self, user_id: &str) -> Vec<String>;
}

/// Role-based access control configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Role definitions
    pub roles: HashMap<String, RoleDefinition>,
    /// Permission definitions
    pub permissions: HashMap<String, PermissionDefinition>,
    /// Default role for new users
    pub default_role: String,
    /// Admin role name
    pub admin_role: String,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            roles: HashMap::new(),
            permissions: HashMap::new(),
            default_role: "user".to_string(),
            admin_role: "admin".to_string(),
        }
    }
}

/// Role definition with associated permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// Role name
    pub name: String,
    /// Description
    pub description: String,
    /// Permissions granted to this role
    pub permissions: Vec<String>,
    /// Inherited roles
    pub inherits: Vec<String>,
}

/// Permission definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefinition {
    /// Permission name (e.g., "resource:action")
    pub name: String,
    /// Description
    pub description: String,
    /// Resource type
    pub resource: String,
    /// Action type (read, write, delete, admin)
    pub action: String,
}

/// In-memory authorization implementation.
pub struct InMemoryAuthorization {
    config: RbacConfig,
    /// User to roles mapping
    user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// User to permissions mapping (cached)
    user_permissions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl InMemoryAuthorization {
    /// Create a new in-memory authorization with the given configuration.
    pub fn new(config: RbacConfig) -> Self {
        Self {
            config,
            user_roles: Arc::new(RwLock::new(HashMap::new())),
            user_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Assign a role to a user.
    pub async fn assign_role(&self, user_id: &str, role: &str) -> Result<(), AuthError> {
        if !self.config.roles.contains_key(role) {
            return Err(AuthError::Internal(format!("Role {} not found", role)));
        }

        let mut user_roles = self.user_roles.write().await;
        let roles = user_roles.entry(user_id.to_string()).or_default();
        if !roles.contains(&role.to_string()) {
            roles.push(role.to_string());
        }

        // Clear cached permissions
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.remove(user_id);

        Ok(())
    }

    /// Remove a role from a user.
    pub async fn remove_role(&self, user_id: &str, role: &str) -> Result<(), AuthError> {
        let mut user_roles = self.user_roles.write().await;
        if let Some(roles) = user_roles.get_mut(user_id) {
            roles.retain(|r| r != role);
        }

        // Clear cached permissions
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.remove(user_id);

        Ok(())
    }

    /// Get all roles for a user including inherited roles.
    async fn get_all_roles(&self, user_id: &str) -> Vec<String> {
        let user_roles = self.user_roles.read().await;
        let direct_roles = user_roles.get(user_id).cloned().unwrap_or_default();

        let mut all_roles = direct_roles.clone();
        let mut to_process = direct_roles;

        // Resolve role inheritance
        while let Some(role_name) = to_process.pop() {
            if let Some(role_def) = self.config.roles.get(&role_name) {
                for inherited in &role_def.inherits {
                    if !all_roles.contains(inherited) {
                        all_roles.push(inherited.clone());
                        to_process.push(inherited.clone());
                    }
                }
            }
        }

        all_roles
    }

    /// Get all permissions for a user.
    async fn compute_permissions(&self, user_id: &str) -> Vec<String> {
        let all_roles = self.get_all_roles(user_id).await;
        let mut permissions = std::collections::HashSet::new();

        for role_name in all_roles {
            if let Some(role_def) = self.config.roles.get(&role_name) {
                for perm in &role_def.permissions {
                    permissions.insert(perm.clone());
                }
            }
        }

        permissions.into_iter().collect()
    }
}

#[async_trait]
impl Authorization for InMemoryAuthorization {
    async fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        // Admin always has all permissions
        if self.has_role(user_id, &self.config.admin_role).await {
            return true;
        }

        // Check cache first
        {
            let user_permissions = self.user_permissions.read().await;
            if let Some(perms) = user_permissions.get(user_id) {
                return perms.contains(&permission.to_string());
            }
        }

        // Compute and cache
        let permissions = self.compute_permissions(user_id).await;
        let has_perm = permissions.contains(&permission.to_string());

        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.insert(user_id.to_string(), permissions);

        has_perm
    }

    async fn has_role(&self, user_id: &str, role: &str) -> bool {
        let all_roles = self.get_all_roles(user_id).await;
        all_roles.contains(&role.to_string())
    }

    async fn has_all_roles(&self, user_id: &str, roles: &[&str]) -> bool {
        let all_roles = self.get_all_roles(user_id).await;
        roles.iter().all(|r| all_roles.contains(&r.to_string()))
    }

    async fn get_permissions(&self, user_id: &str) -> Vec<String> {
        // Check cache first
        {
            let user_permissions = self.user_permissions.read().await;
            if let Some(perms) = user_permissions.get(user_id) {
                return perms.clone();
            }
        }

        // Compute and cache
        let permissions = self.compute_permissions(user_id).await;

        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.insert(user_id.to_string(), permissions.clone());

        permissions
    }

    async fn get_roles(&self, user_id: &str) -> Vec<String> {
        self.get_all_roles(user_id).await
    }
}

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// Not before
    pub nbf: Option<i64>,
    /// JWT ID
    pub jti: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// Custom claims
    #[serde(flatten)]
    pub extra: HashMap<String, JsonValue>,
}

/// Authentication middleware.
pub struct AuthMiddleware {
    api_key_config: Option<ApiKeyConfig>,
    jwt_config: Option<JwtConfig>,
    authorization: Arc<dyn Authorization>,
    /// Decoding key for JWT validation
    decoding_key: Option<DecodingKey>,
    /// JWT validation configuration
    jwt_validation: Option<Validation>,
}

impl AuthMiddleware {
    /// Create a new auth middleware with the given configurations.
    pub fn new(
        api_key_config: Option<ApiKeyConfig>,
        jwt_config: Option<JwtConfig>,
        authorization: Arc<dyn Authorization>,
    ) -> Result<Self, AuthError> {
        let mut decoding_key = None;
        let mut jwt_validation = None;

        if let Some(ref jwt_cfg) = jwt_config {
            if jwt_cfg.enabled {
                // Set up JWT validation
                let alg = match jwt_cfg.algorithm {
                    crate::security::JwtAlgorithm::Hs256 => Algorithm::HS256,
                    crate::security::JwtAlgorithm::Hs384 => Algorithm::HS384,
                    crate::security::JwtAlgorithm::Hs512 => Algorithm::HS512,
                    crate::security::JwtAlgorithm::Rs256 => Algorithm::RS256,
                    crate::security::JwtAlgorithm::Rs384 => Algorithm::RS384,
                    crate::security::JwtAlgorithm::Rs512 => Algorithm::RS512,
                    crate::security::JwtAlgorithm::Es256 => Algorithm::ES256,
                    crate::security::JwtAlgorithm::Es384 => Algorithm::ES384,
                };

                let mut validation = Validation::new(alg);

                // Set issuer and audience validation
                if let Some(ref issuer) = jwt_cfg.issuer {
                    validation.set_issuer(&[issuer]);
                }
                if let Some(ref audience) = jwt_cfg.audience {
                    validation.set_audience(&[audience]);
                }

                jwt_validation = Some(validation);

                // Set up decoding key
                match jwt_cfg.algorithm {
                    crate::security::JwtAlgorithm::Hs256
                    | crate::security::JwtAlgorithm::Hs384
                    | crate::security::JwtAlgorithm::Hs512 => {
                        if let Some(ref secret) = jwt_cfg.secret {
                            let secret_bytes = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                secret,
                            )
                            .map_err(|e| AuthError::Internal(e.to_string()))?;
                            decoding_key = Some(DecodingKey::from_secret(&secret_bytes));
                        }
                    }
                    _ => {
                        if let Some(ref path) = jwt_cfg.public_key_path {
                            let key_data = std::fs::read(path).map_err(|e| {
                                AuthError::Internal(format!("Failed to read public key: {}", e))
                            })?;
                            decoding_key = match jwt_cfg.algorithm {
                                crate::security::JwtAlgorithm::Rs256
                                | crate::security::JwtAlgorithm::Rs384
                                | crate::security::JwtAlgorithm::Rs512 => {
                                    Some(DecodingKey::from_rsa_pem(&key_data).map_err(|e| {
                                        AuthError::Internal(format!("Invalid RSA key: {}", e))
                                    })?)
                                }
                                crate::security::JwtAlgorithm::Es256
                                | crate::security::JwtAlgorithm::Es384 => {
                                    Some(DecodingKey::from_ec_pem(&key_data).map_err(|e| {
                                        AuthError::Internal(format!("Invalid EC key: {}", e))
                                    })?)
                                }
                                _ => None,
                            };
                        }
                    }
                }
            }
        }

        Ok(Self {
            api_key_config,
            jwt_config,
            authorization,
            decoding_key,
            jwt_validation,
        })
    }

    /// Authenticate a request with an API key.
    pub async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthenticationResult, AuthError> {
        let config = self
            .api_key_config
            .as_ref()
            .ok_or(AuthError::AuthenticationRequired)?;

        for entry in &config.keys {
            if api_key == &entry.hash {
                if let Some(expires_at) = entry.expires_at {
                    if Utc::now().timestamp() > expires_at {
                        return Err(AuthError::TokenExpired);
                    }
                }

                return Ok(AuthenticationResult {
                    user_id: entry.id.clone(),
                    roles: entry.roles.clone(),
                    metadata: HashMap::new(),
                    method: AuthMethod::ApiKey,
                    expires_at: entry.expires_at.map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
                });
            }
        }

        Err(AuthError::InvalidCredentials("Invalid API key".to_string()))
    }

    /// Authenticate a request with a JWT token.
    pub async fn authenticate_jwt(&self, token: &str) -> Result<AuthenticationResult, AuthError> {
        let _config = self
            .jwt_config
            .as_ref()
            .ok_or(AuthError::AuthenticationRequired)?;

        let decoding_key = self
            .decoding_key
            .as_ref()
            .ok_or(AuthError::Internal("JWT decoding key not configured".to_string()))?;

        let validation = self
            .jwt_validation
            .as_ref()
            .ok_or(AuthError::Internal("JWT validation not configured".to_string()))?;

        let token_data = decode::<JwtClaims>(token, decoding_key, validation)
            .map_err(|e| AuthError::TokenInvalid(e.to_string()))?;

        let claims = token_data.claims;

        Ok(AuthenticationResult {
            user_id: claims.sub.clone(),
            roles: claims.roles.clone(),
            metadata: claims.extra,
            method: AuthMethod::Jwt,
            expires_at: DateTime::from_timestamp(claims.exp, 0),
        })
    }

    /// Authenticate a request based on available credentials.
    pub async fn authenticate(
        &self,
        api_key: Option<&str>,
        jwt_token: Option<&str>,
    ) -> Result<AuthenticationResult, AuthError> {
        if let Some(key) = api_key {
            if self.api_key_config.is_some() {
                return self.authenticate_api_key(key).await;
            }
        }

        if let Some(token) = jwt_token {
            if self.jwt_config.is_some() {
                return self.authenticate_jwt(token).await;
            }
        }

        Err(AuthError::AuthenticationRequired)
    }

    /// Check if a user has a specific permission.
    pub async fn check_permission(&self, user_id: &str, permission: &str) -> bool {
        self.authorization.has_permission(user_id, permission).await
    }

    /// Check if a user has a specific role.
    pub async fn check_role(&self, user_id: &str, role: &str) -> bool {
        self.authorization.has_role(user_id, role).await
    }

    /// Generate a new JWT token for a user.
    pub fn generate_jwt(
        &self,
        user_id: &str,
        roles: &[String],
        expires_in_seconds: i64,
    ) -> Result<String, AuthError> {
        let jwt_config = self
            .jwt_config
            .as_ref()
            .ok_or(AuthError::Internal("JWT not configured".to_string()))?;

        let encoding_key = self.get_encoding_key()?;
        let header = self.get_header()?;

        let now = Utc::now();
        let claims = JwtClaims {
            sub: user_id.to_string(),
            iss: jwt_config.issuer.clone(),
            aud: jwt_config.audience.clone(),
            exp: (now + chrono::Duration::seconds(expires_in_seconds)).timestamp(),
            iat: now.timestamp(),
            nbf: Some(now.timestamp()),
            jti: Some(uuid::Uuid::new_v4().to_string()),
            roles: roles.to_vec(),
            extra: HashMap::new(),
        };

        encode(&header, &claims, &encoding_key)
            .map_err(|e| AuthError::Internal(format!("Failed to encode JWT: {}", e)))
    }

    fn get_encoding_key(&self) -> Result<EncodingKey, AuthError> {
        let jwt_config = self.jwt_config.as_ref().unwrap();

        match jwt_config.algorithm {
            crate::security::JwtAlgorithm::Hs256
            | crate::security::JwtAlgorithm::Hs384
            | crate::security::JwtAlgorithm::Hs512 => {
                let secret = jwt_config
                    .secret
                    .as_ref()
                    .ok_or(AuthError::Internal("JWT secret not configured".to_string()))?;
                let secret_bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    secret,
                )
                .map_err(|e| AuthError::Internal(e.to_string()))?;
                Ok(EncodingKey::from_secret(&secret_bytes))
            }
            crate::security::JwtAlgorithm::Rs256
            | crate::security::JwtAlgorithm::Rs384
            | crate::security::JwtAlgorithm::Rs512 => {
                let path = jwt_config
                    .private_key_path
                    .as_ref()
                    .ok_or(AuthError::Internal("Private key path not configured".to_string()))?;
                let key_data = std::fs::read(path)
                    .map_err(|e| AuthError::Internal(format!("Failed to read private key: {}", e)))?;
                EncodingKey::from_rsa_pem(&key_data)
                    .map_err(|e| AuthError::Internal(format!("Invalid private key: {}", e)))
            }
            crate::security::JwtAlgorithm::Es256
            | crate::security::JwtAlgorithm::Es384 => {
                let path = jwt_config
                    .private_key_path
                    .as_ref()
                    .ok_or(AuthError::Internal("Private key path not configured".to_string()))?;
                let key_data = std::fs::read(path)
                    .map_err(|e| AuthError::Internal(format!("Failed to read private key: {}", e)))?;
                EncodingKey::from_ec_pem(&key_data)
                    .map_err(|e| AuthError::Internal(format!("Invalid private key: {}", e)))
            }
            _ => Err(AuthError::Internal("Unsupported algorithm".to_string())),
        }
    }

    fn get_header(&self) -> Result<Header, AuthError> {
        let jwt_config = self.jwt_config.as_ref().unwrap();
        let alg = match jwt_config.algorithm {
            crate::security::JwtAlgorithm::Hs256 => Algorithm::HS256,
            crate::security::JwtAlgorithm::Hs384 => Algorithm::HS384,
            crate::security::JwtAlgorithm::Hs512 => Algorithm::HS512,
            crate::security::JwtAlgorithm::Rs256 => Algorithm::RS256,
            crate::security::JwtAlgorithm::Rs384 => Algorithm::RS384,
            crate::security::JwtAlgorithm::Rs512 => Algorithm::RS512,
            crate::security::JwtAlgorithm::Es256 => Algorithm::ES256,
            crate::security::JwtAlgorithm::Es384 => Algorithm::ES384,
        };
        Ok(Header::new(alg))
    }
}

/// Hash a password using Argon2.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::Internal(format!("Failed to hash password: {}", e)))?;

    Ok(hash.to_string())
}

/// Verify a password against its hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AuthError::Internal(format!("Invalid password hash: {}", e)))?;
    let argon2 = Argon2::default();
    let is_valid = argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_authorization_basic() {
        let mut roles = HashMap::new();
        roles.insert(
            "admin".to_string(),
            RoleDefinition {
                name: "admin".to_string(),
                description: "Administrator".to_string(),
                permissions: vec!["*".to_string()],
                inherits: vec![],
            },
        );
        roles.insert(
            "user".to_string(),
            RoleDefinition {
                name: "user".to_string(),
                description: "Regular user".to_string(),
                permissions: vec!["read:self".to_string()],
                inherits: vec![],
            },
        );

        let config = RbacConfig {
            roles,
            permissions: HashMap::new(),
            default_role: "user".to_string(),
            admin_role: "admin".to_string(),
        };

        let auth = InMemoryAuthorization::new(config);

        auth.assign_role("user1", "admin").await.unwrap();
        assert!(auth.has_permission("user1", "anything").await);
        assert!(auth.has_role("user1", "admin").await);

        auth.assign_role("user2", "user").await.unwrap();
        assert!(auth.has_permission("user2", "read:self").await);
        assert!(!auth.has_permission("user2", "write:other").await);
    }

    #[tokio::test]
    async fn test_role_inheritance() {
        let mut roles = HashMap::new();
        roles.insert(
            "moderator".to_string(),
            RoleDefinition {
                name: "moderator".to_string(),
                description: "Moderator".to_string(),
                permissions: vec!["delete:posts".to_string()],
                inherits: vec!["user".to_string()],
            },
        );
        roles.insert(
            "user".to_string(),
            RoleDefinition {
                name: "user".to_string(),
                description: "User".to_string(),
                permissions: vec!["read:posts".to_string()],
                inherits: vec![],
            },
        );

        let config = RbacConfig {
            roles,
            permissions: HashMap::new(),
            default_role: "user".to_string(),
            admin_role: "admin".to_string(),
        };

        let auth = InMemoryAuthorization::new(config);
        auth.assign_role("user1", "moderator").await.unwrap();

        assert!(auth.has_permission("user1", "delete:posts").await);
        assert!(auth.has_permission("user1", "read:posts").await);
        assert!(auth.has_role("user1", "user").await);
    }

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_jwt_claims_serialization() {
        let claims = JwtClaims {
            sub: "user123".to_string(),
            iss: Some("nebula".to_string()),
            aud: None,
            exp: 1234567890,
            iat: 1234567800,
            nbf: None,
            jti: None,
            roles: vec!["user".to_string()],
            extra: HashMap::new(),
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"sub\":\"user123\""));
        assert!(json.contains("\"roles\":[\"user\"]"));
    }
}
