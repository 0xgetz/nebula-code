# Nebula Production

Production-ready deployment infrastructure for Rust applications.

## Overview

Nebula Production provides essential components for deploying production-grade services with security, observability, and performance built in. It covers the full spectrum of production concerns:

- **Configuration Management** - Environment-aware configuration with validation
- **Health Checking** - Liveness and readiness probes for Kubernetes
- **Security** - TLS, API keys, JWT authentication, input validation, CSRF protection
- **Authentication & Authorization** - RBAC, password hashing, JWT token management
- **Observability** - Structured logging, metrics collection, distributed tracing
- **Performance** - Connection pooling, caching, load balancing, rate limiting
- **Alerting** - Severity-based alerting with notification channels

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
nebula-production = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use nebula_production::{
    ProductionConfig, HealthChecker, Server, SecurityConfig,
    init_logging, MetricsCollector,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = ProductionConfig::from_env()?;

    // Initialize logging
    init_logging(&config.logging.level, &config.logging.format);

    // Create health checker
    let health_checker = Arc::new(HealthChecker::new());

    // Create metrics collector
    let metrics = MetricsCollector::new();

    // Configure security
    let security = SecurityConfig::production();

    // Build and start the server
    let server = Server::builder()
        .config(config.clone())
        .health_checker(health_checker)
        .metrics(metrics)
        .security(security)
        .build()?;

    server.start().await?;
    Ok(())
}
```

## Configuration

Configuration is loaded from environment variables with sensible defaults:

| Variable | Description | Default |
|----------|-------------|---------|
| `NEBULA_ENV` | Environment (development/staging/production) | development |
| `NEBULA_HOST` | Host to bind to | 0.0.0.0 |
| `NEBULA_PORT` | Port to listen on | 8080 |
| `RUST_LOG` | Log level (trace/debug/info/warn/error) | info |
| `NEBULA_LOG_FORMAT` | Log format (json/pretty) | json |
| `NEBULA_APP_NAME` | Application name | nebula-production |
| `NEBULA_APP_VERSION` | Application version | 0.1.0 |
| `NEBULA_SHUTDOWN_TIMEOUT` | Graceful shutdown timeout in seconds | 30 |

### Programmatic Configuration

```rust
use nebula_production::{ProductionConfig, Environment, ServerConfigInner};

let config = ProductionConfig {
    environment: Environment::Production,
    app_name: "my-service".to_string(),
    app_version: "1.0.0".to_string(),
    server: ServerConfigInner {
        host: "0.0.0.0".to_string(),
        port: 8080,
        shutdown_timeout_secs: 30,
    },
    ..Default::default()
};
```

## Security

### TLS Configuration

```rust
use nebula_production::{SecurityConfig, TlsConfig, TlsVersion};

let security = SecurityConfig {
    tls: TlsConfig {
        enabled: true,
        cert_path: Some("/path/to/cert.pem".into()),
        key_path: Some("/path/to/key.pem".into()),
        min_version: TlsVersion::Tls13,
        ..Default::default()
    },
    ..SecurityConfig::production()
};
```

### API Key Authentication

```rust
use nebula_production::{ApiKeyConfig, ApiKeyEntry};

let api_key_config = ApiKeyConfig {
    enabled: true,
    header_name: "X-API-Key".to_string(),
    keys: vec![
        ApiKeyEntry {
            id: "service-1".to_string(),
            hash: "hashed-key-value".to_string(),
            name: "Service 1".to_string(),
            roles: vec!["admin".to_string()],
            expires_at: None,
            created_at: chrono::Utc::now().timestamp(),
        },
    ],
    rotation_interval_hours: Some(24 * 90), // 90 days
};
```

### JWT Authentication

```rust
use nebula_production::{JwtConfig, JwtAlgorithm};

let jwt_config = JwtConfig {
    enabled: true,
    algorithm: JwtAlgorithm::Hs256,
    secret: Some(base64::encode("your-secret-key")),
    issuer: Some("your-issuer".to_string()),
    audience: Some("your-audience".to_string()),
    expiration_leeway_seconds: 60,
    required_claims: vec!["sub".to_string(), "exp".to_string()],
    ..Default::default()
};
```

### Input Validation

```rust
use nebula_production::{RequestValidator, ValidationRule, InputSanitizer};

// Create validation rules
let validator = RequestValidator::new()
    .add_rule(ValidationRule::new("username").required().min_length(3).max_length(50))
    .add_rule(ValidationRule::new("email").required().pattern("*@*.*"))
    .add_rule(ValidationRule::new("role").allowed_values(vec!["user", "admin"]));

// Sanitize input
let sanitizer = InputSanitizer::new()
    .strip_html_tags(true)
    .check_sql_injection(true)
    .check_xss(true)
    .max_input_length(10000);

let sanitized = sanitizer.sanitize(user_input)?;
```

## Authentication & Authorization

### RBAC Setup

```rust
use nebula_production::{RbacConfig, RoleDefinition, InMemoryAuthorization};

let mut roles = std::collections::HashMap::new();
roles.insert("admin".to_string(), RoleDefinition {
    name: "admin".to_string(),
    description: "Administrator with full access".to_string(),
    permissions: vec!["*".to_string()],
    inherits: vec![],
});
roles.insert("user".to_string(), RoleDefinition {
    name: "user".to_string(),
    description: "Regular user".to_string(),
    permissions: vec!["read:self".to_string()],
    inherits: vec![],
});

let config = RbacConfig {
    roles,
    permissions: std::collections::HashMap::new(),
    default_role: "user".to_string(),
    admin_role: "admin".to_string(),
};

let auth = InMemoryAuthorization::new(config);
```

### Password Hashing

```rust
use nebula_production::{hash_password, verify_password};

let password = "secure_password_123";
let hash = hash_password(password)?;
assert!(verify_password(password, &hash)?);
```

## Health Checking

### Custom Health Checks

```rust
use nebula_production::{HealthCheck, HealthChecker, HealthCheckResult};
use std::sync::Arc;

struct DatabaseHealthCheck {
    connection_string: String,
}

#[async_trait::async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        "database"
    }

    async fn check(&self) -> HealthCheckResult {
        // Implement your database connectivity check
        match self.check_connection().await {
            Ok(_) => HealthCheckResult::healthy("database"),
            Err(e) => HealthCheckResult::unhealthy("database", e.to_string()),
        }
    }
}

// Register health checks
let checker = Arc::new(HealthChecker::new());
checker.register(Arc::new(DatabaseHealthCheck { 
    connection_string: "...".to_string() 
})).await;
```

### Kubernetes Integration

The health endpoints are compatible with Kubernetes probes:

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

## Connection Pooling

```rust
use nebula_production::{ConnectionPool, PoolConfig, ConnectionFactory, PooledConnection};

struct MyConnectionFactory {
    // connection parameters
}

#[async_trait::async_trait]
impl ConnectionFactory for MyConnectionFactory {
    type Connection = MyConnection;

    async fn create(&self) -> Result<Self::Connection, Box<dyn std::error::Error + Send + Sync>> {
        // Create and return a new connection
        Ok(MyConnection::new(/* ... */).await?)
    }
}

#[async_trait::async_trait]
impl PooledConnection for MyConnection {
    async fn is_valid(&self) -> bool {
        // Check if connection is still valid
        self.ping().await.is_ok()
    }

    async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean up the connection
        Ok(())
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

// Create and use the pool
let config = PoolConfig::new(5, 20)
    .with_connection_timeout(5000)
    .with_max_lifetime(3600)
    .with_max_idle_time(600);

let pool = ConnectionPool::new(config, MyConnectionFactory { /* ... */ }).await?;

// Get a connection
let conn = pool.get().await?;
// Use conn...
// Connection automatically returned when dropped
```

## Rate Limiting

```rust
use nebula_production::{RateLimiter, RateLimiterConfig, RateLimitTier};

let config = RateLimiterConfig {
    default_tier: RateLimitTier {
        requests_per_second: 100,
        burst_size: 200,
    },
    ..Default::default()
};

let limiter = RateLimiter::new(config);

// Check rate limit
if limiter.check_limit("client-ip").is_ok() {
    // Process request
} else {
    // Return 429 Too Many Requests
}
```

## Deployment Guide

### Docker

```dockerfile
FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --package nebula-production

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/target/release/nebula-production /usr/local/bin/
EXPOSE 8080
CMD ["nebula-production"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  app:
    image: your-app:latest
    environment:
      - NEBULA_ENV=production
      - NEBULA_HOST=0.0.0.0
      - NEBULA_PORT=8080
      - RUST_LOG=info
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health/live"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nebula-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nebula
  template:
    metadata:
      labels:
        app: nebula
    spec:
      containers:
      - name: app
        image: your-app:latest
        ports:
        - containerPort: 8080
        env:
        - name: NEBULA_ENV
          value: "production"
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
```

## API Reference

### Core Types

- **`ProductionConfig`** - Main configuration struct
- **`SecurityConfig`** - Security settings (TLS, API keys, JWT)
- **`HealthChecker`** - Health check manager
- **`MetricsCollector`** - Prometheus metrics
- **`Server`** - HTTP server with graceful shutdown

### Security

- **`TlsConfig`** - TLS configuration
- **`JwtConfig`** - JWT authentication
- **`ApiKeyConfig`** - API key authentication
- **`RequestValidator`** - Input validation
- **`InputSanitizer`** - Input sanitization (XSS, SQL injection)
- **`CsrfManager`** - CSRF token management

### Authentication

- **`AuthMiddleware`** - Authentication middleware
- **`Authorization`** - Authorization trait
- **`InMemoryAuthorization`** - In-memory RBAC implementation
- **`RbacConfig`** - Role-based access control configuration

### Performance

- **`ConnectionPool`** - Generic connection pooling
- **`PoolConfig`** - Pool configuration
- **`RateLimiter`** - Token bucket rate limiting
- **`CacheConfig`** - Caching configuration

### Observability

- **`TracingConfig`** - Distributed tracing configuration
- **`ObservabilityConfig`** - Unified observability stack
- **`AlertManager`** - Alert management

## License

MIT
