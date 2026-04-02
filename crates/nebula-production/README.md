# Nebula Production

Production-ready deployment infrastructure for Rust applications.

## Overview

Nebula Production provides essential components for deploying production-grade services with security, observability, and performance built in.

### Features

- **Configuration Management** - Environment-aware configuration with validation
- **Health Checking** - Liveness and readiness probes for Kubernetes
- **Metrics** - Prometheus-compatible metrics collection
- **Logging** - Structured JSON logging with tracing integration
- **Security** - TLS, API keys, JWT authentication, input validation, CSRF protection
- **Authentication & Authorization** - RBAC, password hashing, JWT token management
- **Encryption** - HMAC, key derivation, and encryption utilities
- **TLS** - Modern TLS configuration with secure defaults
- **Cache** - In-memory and Redis-backed caching with eviction policies
- **Connection Pool** - Generic connection pooling for databases and HTTP clients
- **Load Balancer** - Load balancing strategies for distributed systems
- **Rate Limiting** - Token bucket rate limiting with tiered limits
- **Distributed Tracing** - OpenTelemetry-compatible tracing
- **Alerting** - Severity-based alerting with notification channels
- **Server** - HTTP server with graceful shutdown

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
nebula-production = "0.1"
tokio = { version = "1", features = ["full"] }
```

Basic usage:

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

    // Configure security with production-safe defaults
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

### Environment-Specific Defaults

```rust
use nebula_production::{ProductionConfig, Environment};

// Production environment with strict security
let config = ProductionConfig::builder()
    .environment(Environment::Production)
    .host("0.0.0.0")
    .port(443)
    .tls_enabled(true)
    .build()?;

// Development environment with relaxed settings
let dev_config = ProductionConfig::builder()
    .environment(Environment::Development)
    .host("127.0.0.1")
    .port(8080)
    .tls_enabled(false)
    .build()?;
```

### Security Configuration

```rust
use nebula_production::SecurityConfig;

// Use production-safe defaults (TLS 1.3, secure headers, etc.)
let config = SecurityConfig::production();

// Custom TLS configuration
let tls_config = nebula_production::TlsConfig {
    enabled: true,
    cert_path: Some("/etc/ssl/certs/server.crt".into()),
    key_path: Some("/etc/ssl/private/server.key".into()),
    min_version: nebula_production::TlsVersion::Tls13,
};
```

### JWT Authentication

```rust
use nebula_production::{JwtConfig, JwtAlgorithm};

let jwt_config = JwtConfig {
    secret: "your-secret-key".to_string(),
    algorithm: JwtAlgorithm::HS256,
    expiration_seconds: 3600,
    issuer: Some("nebula-production".to_string()),
};
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p nebula-production

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/nebula-production /usr/local/bin/
EXPOSE 8080
CMD ["nebula-production"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nebula-production
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nebula-production
  template:
    metadata:
      labels:
        app: nebula-production
    spec:
      containers:
      - name: nebula-production
        image: your-registry/nebula-production:latest
        ports:
        - containerPort: 8080
        env:
        - name: NEBULA_ENV
          value: "production"
        - name: NEBULA_HOST
          value: "0.0.0.0"
        - name: NEBULA_PORT
          value: "8080"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 3
---
apiVersion: v1
kind: Service
metadata:
  name: nebula-production
spec:
  selector:
    app: nebula-production
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### Health Endpoints

The server exposes standard health check endpoints:

- `GET /health/live` - Liveness probe (is the process running?)
- `GET /health/ready` - Readiness probe (is the service ready to accept traffic?)

### Metrics

Prometheus metrics are available at `GET /metrics` by default. Configure the metrics endpoint via `ObservabilityConfig`.

## License

MIT
