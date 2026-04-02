# Security Hardening Guide

This guide covers security best practices for deploying and operating Nebula in production environments.

## Table of Contents

- [Transport Security](#transport-security)
- [Authentication](#authentication)
- [Authorization](#authorization)
- [Input Validation](#input-validation)
- [Rate Limiting](#rate-limiting)
- [Data Encryption](#data-encryption)
- [Security Headers](#security-headers)
- [CORS Configuration](#cors-configuration)
- [Secrets Management](#secrets-management)
- [Security Monitoring](#security-monitoring)
- [Incident Response](#incident-response)

## Transport Security

### TLS Configuration

Always use TLS in production to encrypt data in transit.

```bash
# Enable TLS with modern settings
NEBULA_TLS_ENABLED=true
NEBULA_TLS_CERT_PATH=/path/to/cert.pem
NEBULA_TLS_KEY_PATH=/path/to/key.pem
NEBULA_TLS_MIN_VERSION=1.3
```

### Certificate Management

- Use certificates from trusted CAs (Let's Encrypt, DigiCert, etc.)
- Rotate certificates before expiration (set up automatic renewal)
- Store private keys with restricted permissions (0600)
- Use separate certificates for different environments

### Mutual TLS (mTLS)

For service-to-service communication, consider mutual TLS:

```rust
use nebula_production::{TlsConfig, TlsVersion};

let tls_config = TlsConfig {
    enabled: true,
    cert_path: Some("/path/to/server-cert.pem".into()),
    key_path: Some("/path/to/server-key.pem".into()),
    min_version: TlsVersion::Tls13,
    ..Default::default()
};
```

## Authentication

### API Key Authentication

API keys provide simple authentication for service access.

```bash
# Enable API key authentication
NEBULA_API_KEY_ENABLED=true
```

#### Best Practices

1. **Use hashed keys**: Store keys hashed with Argon2, never in plaintext
2. **Rotate regularly**: Set `rotation_interval_hours` for automatic rotation
3. **Scope permissions**: Assign specific roles to each API key
4. **Set expiration**: Use `expires_at` to limit key lifetime
5. **Audit usage**: Log all API key usage for security analysis

```rust
use nebula_production::{ApiKeyConfig, ApiKeyEntry};

let api_key_config = ApiKeyConfig {
    enabled: true,
    header_name: "X-API-Key".to_string(),
    keys: vec![
        ApiKeyEntry {
            id: "key-001".to_string(),
            hash: "$argon2id$v=19$m=19456,t=2,p=1$...".to_string(),
            name: "Production Service".to_string(),
            roles: vec!["service".to_string()],
            expires_at: Some(1735689600), // Unix timestamp
            created_at: 1704153600,
        }
    ],
    rotation_interval_hours: Some(24 * 90), // 90 days
};
```

### JWT Authentication

JWT tokens provide stateless authentication with claims-based authorization.

```bash
# Enable JWT validation
NEBULA_JWT_ENABLED=true
NEBULA_JWT_ALGORITHM=HS256
NEBULA_JWT_SECRET=your-base64-encoded-secret
```

#### Algorithm Selection

| Algorithm | Use Case | Key Management |
|-----------|----------|----------------|
| HS256 | Single service, simple setup | Shared secret |
| RS256 | Multiple services, external clients | Public/private key pair |
| ES256 | High security, efficient signatures | ECDSA key pair |

#### JWT Best Practices

1. **Set short expiration**: Access tokens should expire in 15-60 minutes
2. **Use refresh tokens**: Issue short-lived access tokens with longer-lived refresh tokens
3. **Validate all claims**: Check `iss`, `aud`, `exp`, and `nbf` claims
4. **Use HTTPS**: Never transmit JWTs over unencrypted connections
5. **Store securely**: Don't store JWTs in localStorage (use httpOnly cookies)

```rust
use nebula_production::{JwtConfig, JwtAlgorithm};

let jwt_config = JwtConfig {
    enabled: true,
    algorithm: JwtAlgorithm::Rs256,
    secret: None,
    public_key_path: Some("/path/to/public-key.pem".into()),
    issuer: Some("nebula.example.com".to_string()),
    audience: Some("api.nebula.example.com".to_string()),
    expiration_leeway_seconds: 30,
    required_claims: vec!["sub".to_string(), "exp".to_string(), "iat".to_string()],
};
```

### Password Hashing

For user authentication, use Argon2id for password hashing:

```rust
use nebula_production::{hash_password, verify_password};

// Hash a password
let hash = hash_password("user_password")?;

// Verify a password
let is_valid = verify_password("user_password", &hash)?;
```

## Authorization

### Role-Based Access Control (RBAC)

Define roles with specific permissions:

```rust
use nebula_production::{RbacConfig, RoleDefinition};

let rbac = RbacConfig::new()
    .add_role(RoleDefinition::new("admin")
        .with_permission("users:read")
        .with_permission("users:write")
        .with_permission("system:admin"))
    .add_role(RoleDefinition::new("user")
        .with_permission("users:read")
        .with_permission("profile:write"))
    .add_role(RoleDefinition::new("service")
        .with_permission("internal:call"));
```

#### Principle of Least Privilege

- Grant minimum required permissions
- Use separate roles for different functions
- Review and audit permissions regularly
- Revoke access immediately when no longer needed

## Input Validation

### Request Validation

Validate all incoming request data:

```rust
use nebula_production::{RequestValidator, ValidationRule};

let validator = RequestValidator::new()
    .add_rule(ValidationRule::new("username")
        .required()
        .min_length(3)
        .max_length(50)
        .pattern("^[a-zA-Z0-9_]+$"))
    .add_rule(ValidationRule::new("email")
        .required()
        .pattern("^[^@]+@[^@]+\\.[^@]+$"))
    .add_rule(ValidationRule::new("role")
        .allowed_values(vec!["user", "admin", "moderator"]));

// Validate request fields
validator.validate_fields(&request_fields)?;
```

### Input Sanitization

Sanitize user input to prevent injection attacks:

```rust
use nebula_production::InputSanitizer;

let sanitizer = InputSanitizer::new()
    .max_input_length(10_000)
    .strip_html_tags(true)
    .check_sql_injection(true)
    .check_xss(true)
    .trim_whitespace(true);

let clean_input = sanitizer.sanitize(user_input)?;
```

#### Sanitization Features

- **HTML stripping**: Removes all HTML tags
- **SQL injection detection**: Blocks common SQL injection patterns
- **XSS detection**: Identifies cross-site scripting attempts
- **Length limits**: Prevents buffer overflow and DoS attacks

### CSRF Protection

Protect against Cross-Site Request Forgery:

```rust
use nebula_production::CsrfManager;

let csrf_manager = CsrfManager::new(3600); // 1 hour expiration

// Generate token for forms
let token = csrf_manager.generate_token()?;

// Validate token on form submission
if csrf_manager.validate_token(&submitted_token)? {
    // Process request
    csrf_manager.remove_token(&submitted_token);
} else {
    // Reject request
}
```

## Rate Limiting

### Sliding Window Rate Limiting

Protect against abuse and DoS attacks:

```bash
# Enable rate limiting
NEBULA_RATE_LIMIT_ENABLED=true
NEBULA_RATE_LIMIT_REQUESTS=100
NEBULA_RATE_LIMIT_WINDOW_SECS=60
```

### Rate Limit Tiers

Define different limits for different user tiers:

```rust
use nebula_production::{RateLimiterConfig, RateLimitTier};

let config = RateLimiterConfig::new()
    .with_tier(RateLimitTier::new("free", 100, 3600))      // 100/hour
    .with_tier(RateLimitTier::new("basic", 1000, 3600))    // 1000/hour
    .with_tier(RateLimitTier::new("premium", 10000, 3600)) // 10000/hour
    .with_tier(RateLimitTier::new("enterprise", 100000, 3600)); // 100000/hour
```

### Rate Limit Best Practices

1. **Set appropriate limits**: Balance between usability and protection
2. **Return proper headers**: Include `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
3. **Use sliding window**: More accurate than fixed windows
4. **Implement backoff**: Exponential backoff for repeated violations
5. **Monitor patterns**: Detect and block abuse patterns

## Data Encryption

### Encryption at Rest

Encrypt sensitive data stored in databases:

```rust
use nebula_production::{EncryptionEngine, generate_random_key};

// Generate a secure key
let key = generate_random_key()?;

// Create encryption engine
let engine = EncryptionEngine::new(&key)?;

// Encrypt data
let encrypted = engine.encrypt(b"sensitive data")?;

// Decrypt data
let decrypted = engine.decrypt(&encrypted)?;
```

### Key Management

1. **Use strong keys**: 256-bit keys for AES-256
2. **Rotate keys regularly**: Implement key rotation procedures
3. **Store keys securely**: Use HSM or cloud KMS
4. **Separate keys from data**: Never store encryption keys with encrypted data
5. **Use key derivation**: Derive keys from passwords using PBKDF2 or Argon2

```rust
use nebula_production::derive_key_from_password;

// Derive a key from a password
let key = derive_key_from_password("user_password", salt)?;
```

### HMAC for Data Integrity

Verify data integrity with HMAC:

```rust
use nebula_production::{compute_hmac, verify_hmac};

// Compute HMAC
let mac = compute_hmac(data, secret_key)?;

// Verify HMAC
let is_valid = verify_hmac(data, &mac, secret_key)?;
```

## Security Headers

Security headers are enabled by default. Customize as needed:

```rust
use nebula_production::{SecurityHeaders};

let headers = SecurityHeaders {
    hsts: Some("max-age=31536000; includeSubDomains; preload".to_string()),
    csp: Some("default-src 'self'; script-src 'self'; style-src 'self'".to_string()),
    no_sniff: true,
    frame_options: Some("DENY".to_string()),
    xss_protection: true,
};
```

### Header Explanations

| Header | Purpose |
|--------|---------|
| Strict-Transport-Security | Force HTTPS connections |
| Content-Security-Policy | Prevent XSS and data injection |
| X-Content-Type-Options | Prevent MIME type sniffing |
| X-Frame-Options | Prevent clickjacking |
| X-XSS-Protection | Enable browser XSS filtering |

## CORS Configuration

Configure Cross-Origin Resource Sharing carefully:

```rust
use nebula_production::{CorsConfig};

let cors = CorsConfig {
    allowed_origins: vec![
        "https://app.example.com".to_string(),
        "https://admin.example.com".to_string(),
    ],
    allowed_methods: vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
    ],
    allowed_headers: vec![
        "Content-Type".to_string(),
        "Authorization".to_string(),
        "X-API-Key".to_string(),
    ],
    allow_credentials: true,
    max_age_seconds: 3600,
};
```

### CORS Best Practices

1. **Specify exact origins**: Never use `*` in production
2. **Limit methods**: Only allow necessary HTTP methods
3. **Restrict headers**: Only allow required headers
4. **Set appropriate max-age**: Balance caching with flexibility
5. **Test thoroughly**: Verify CORS behavior across browsers

## IP Access Control

### Allowlisting

Restrict access to specific IP addresses:

```rust
use nebula_production::IpAccessControl;

let control = IpAccessControl::new_allowlist();
control.allow_ip("10.0.0.1".parse().unwrap());
control.allow_ip("192.168.1.0/24".parse().unwrap()); // CIDR notation

// Check access
if control.is_allowed(client_ip) {
    // Allow access
} else {
    // Deny access
}
```

### Blocklisting

Block known malicious IPs:

```rust
let control = IpAccessControl::new_blocklist();
control.block_ip("192.168.1.100".parse().unwrap());

// Automatically block after failed attempts
// (Implement with rate limiting integration)
```

## Secrets Management

### Environment Variables

Never commit secrets to version control. Use environment variables:

```bash
# .env file (add to .gitignore)
NEBULA_JWT_SECRET=your-secret-key
DATABASE_URL=postgres://user:password@localhost/db
REDIS_URL=redis://:password@localhost:6379
```

### Cloud Secrets Management

Use cloud provider secrets services:

- **AWS**: Secrets Manager, SSM Parameter Store
- **GCP**: Secret Manager
- **Azure**: Key Vault

### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: nebula-secrets
type: Opaque
data:
  jwt-secret: <base64-encoded-secret>
  api-key: <base64-encoded-key>
```

Mount as environment variables:

```yaml
env:
- name: NEBULA_JWT_SECRET
  valueFrom:
    secretKeyRef:
      name: nebula-secrets
      key: jwt-secret
```

## Security Monitoring

### Alerting

Configure alerts for security events:

```rust
use nebula_production::{AlertManager, Severity, NotificationChannel};

let alert_manager = AlertManager::new()
    .add_channel(NotificationChannel::webhook("https://hooks.example.com/security"))
    .add_channel(NotificationChannel::pagerduty("your-pagerduty-key"));

// Alert on security events
alert_manager.trigger(
    "authentication_failure_spike",
    Severity::High,
    "Multiple authentication failures detected",
).await?;
```

### Log Security Events

Log all security-relevant events in structured format:

- Authentication attempts (success and failure)
- Authorization decisions
- Rate limit violations
- Input validation failures
- TLS certificate issues
- Configuration changes

### Distributed Tracing

Enable tracing for security investigation:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
OTEL_SERVICE_NAME=nebula
```

## Incident Response

### Preparation

1. **Document procedures**: Create incident response playbooks
2. **Define roles**: Assign incident response responsibilities
3. **Set up monitoring**: Configure alerts for security events
4. **Test regularly**: Conduct security drills and tabletop exercises

### Response Steps

1. **Identify**: Determine the nature and scope of the incident
2. **Contain**: Isolate affected systems to prevent spread
3. **Eradicate**: Remove the threat and remediate vulnerabilities
4. **Recover**: Restore systems and verify normal operation
5. **Learn**: Document lessons and update procedures

### Key Contacts

Maintain an up-to-date contact list:
- Security team leads
- Infrastructure engineers
- Legal counsel
- PR/Communications
- Executive leadership

## Compliance Considerations

### Data Protection

- **GDPR**: Implement data minimization, consent management, and right to deletion
- **CCPA**: Provide opt-out mechanisms and data access rights
- **HIPAA**: Ensure PHI encryption and access controls (if applicable)

### Audit Logging

Maintain audit logs for:
- User authentication and authorization
- Data access and modifications
- System configuration changes
- Security events and incidents

### Penetration Testing

- Conduct regular penetration tests
- Perform vulnerability assessments
- Address findings promptly
- Document remediation efforts

## Security Checklist

Before production deployment:

- [ ] TLS enabled with valid certificates
- [ ] API key or JWT authentication configured
- [ ] Rate limiting enabled
- [ ] Input validation implemented
- [ ] Security headers configured
- [ ] CORS properly configured
- [ ] Secrets managed securely
- [ ] Logging and monitoring enabled
- [ ] Incident response plan documented
- [ ] Dependencies scanned for vulnerabilities
- [ ] Container images scanned for vulnerabilities
- [ ] Network policies configured
- [ ] Backup and recovery procedures tested

## Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [Rust Security Guidelines](https://doc.rust-lang.org/nightly/edition-guide/rust-2021/security.html)

## Reporting Security Issues

If you discover a security vulnerability, please report it responsibly:

- Email: security@nebula-code.example.com
- Do not disclose publicly until a fix is available
- Include detailed reproduction steps
- Allow reasonable time for remediation
