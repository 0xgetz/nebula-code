# Deployment Guide

This guide covers deploying Nebula to production environments, including Docker containers, Kubernetes clusters, and major cloud providers.

## Prerequisites

- Rust 1.75+
- Docker (for container deployments)
- kubectl (for Kubernetes deployments)
- Cloud CLI tools (aws, gcloud, az) for cloud deployments

## Building for Production

### Release Build

```bash
# Build optimized release binary
cargo build --release --package nebula-production

# Binary location
./target/release/nebula-production
```

### Build Optimization

For the smallest binary size:

```toml
# In Cargo.toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

For fastest compilation during development:

```toml
[profile.dev]
opt-level = 1
```

## Docker Deployment

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim-bullseye AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Build release binary
RUN cargo build --release --package nebula-production

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/nebula-production /usr/local/bin/

# Create non-root user
RUN useradd -r -u 1000 nebula
USER nebula

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health/live || exit 1

# Run
CMD ["nebula-production"]
```

### Building the Image

```bash
# Build
docker build -t nebula-production:latest .

# Run locally
docker run -d \
    -p 8080:8080 \
    -e NEBULA_ENV=production \
    -e RUST_LOG=info \
    --name nebula \
    nebula-production:latest
```

### Docker Compose

```yaml
version: '3.8'

services:
  nebula:
    image: nebula-production:latest
    ports:
      - "8080:8080"
    environment:
      NEBULA_ENV: production
      RUST_LOG: info
      NEBULA_HOST: 0.0.0.0
      NEBULA_PORT: 8080
    volumes:
      - ./config:/app/config
      - ./certs:/app/certs:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health/live"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s

  # Optional: Redis for caching
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    restart: unless-stopped

volumes:
  redis-data:
```

### Multi-Stage Build with Cross-Compilation

For ARM64 (Apple Silicon, AWS Graviton):

```bash
# Install cross-compilation tools
rustup target add aarch64-unknown-linux-musl

# Build with cross
docker run --rm -v $(pwd):/app -w /app clux/muslrust \
    cargo build --release --target aarch64-unknown-linux-musl

# Build multi-arch image
docker buildx build --platform linux/amd64,linux/arm64 \
    -t nebula-production:latest --push .
```

## Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: nebula
  labels:
    name: nebula
---
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: nebula-config
  namespace: nebula
data:
  NEBULA_ENV: "production"
  RUST_LOG: "info"
  NEBULA_LOG_FORMAT: "json"
  NEBULA_HOST: "0.0.0.0"
  NEBULA_PORT: "8080"
  NEBULA_METRICS_ENABLED: "true"
  NEBULA_HEALTH_ENABLED: "true"
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nebula
  namespace: nebula
  labels:
    app: nebula
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
      - name: nebula
        image: nebula-production:latest
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        env:
        - name: NEBULA_ENV
          valueFrom:
            configMapKeyRef:
              name: nebula-config
              key: NEBULA_ENV
        - name: RUST_LOG
          valueFrom:
            configMapKeyRef:
              name: nebula-config
              key: RUST_LOG
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          readOnlyRootFilesystem: true
          allowPrivilegeEscalation: false
          capabilities:
            drop:
              - ALL
```

### Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: nebula
  namespace: nebula
  labels:
    app: nebula
spec:
  type: ClusterIP
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
    name: http
  selector:
    app: nebula
```

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: nebula-hpa
  namespace: nebula
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: nebula
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Ingress (NGINX)

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: nebula-ingress
  namespace: nebula
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "60"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - nebula.example.com
    secretName: nebula-tls
  rules:
  - host: nebula.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: nebula
            port:
              number: 80
```

### Deploy to Kubernetes

```bash
# Apply all resources
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f hpa.yaml
kubectl apply -f ingress.yaml

# Check deployment status
kubectl get pods -n nebula
kubectl get deployment -n nebula
kubectl get hpa -n nebula

# View logs
kubectl logs -f deployment/nebula -n nebula
```

## Cloud Provider Deployments

### AWS

#### ECS with Fargate

```yaml
# task-definition.json
{
  "family": "nebula",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "executionRoleArn": "arn:aws:iam::ACCOUNT:role/ecsTaskExecutionRole",
  "containerDefinitions": [
    {
      "name": "nebula",
      "image": "ACCOUNT.dkr.ecr.REGION.amazonaws.com/nebula-production:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "NEBULA_ENV", "value": "production"},
        {"name": "RUST_LOG", "value": "info"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/nebula",
          "awslogs-region": "REGION",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/health/live || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 10
      }
    }
  ]
}
```

#### EKS

Use the Kubernetes manifests above with EKS. Additional IAM roles for service accounts (IRSA) may be needed for AWS integrations.

#### App Runner

```bash
# Deploy directly from ECR
aws apprunner create-service \
  --service-name nebula \
  --source-configuration ImageRepository={ImageIdentifier=ACCOUNT.dkr.ecr.REGION.amazonaws.com/nebula-production:latest,ImageConfiguration={Port=8080,RuntimeEnvironmentSecrets={}}} \
  --instance-configuration Cpu=1 vCPU,Memory=2 GB \
  --auto-scaling-configuration-arn arn:aws:apprunner:REGION:ACCOUNT:autoscalingconfiguration/HighThroughput/1a6f35e0
```

### Google Cloud Platform

#### Cloud Run

```bash
# Build and push to GCR
gcloud builds submit --tag gcr.io/PROJECT/nebula-production

# Deploy to Cloud Run
gcloud run deploy nebula \
  --image gcr.io/PROJECT/nebula-production \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --memory 512Mi \
  --cpu 1 \
  --max-instances 10 \
  --set-env-vars NEBULA_ENV=production,RUST_LOG=info
```

#### GKE

Use the Kubernetes manifests above with GKE. Enable Workload Identity for secure access to GCP services.

### Microsoft Azure

#### Azure Container Instances

```bash
az container create \
  --resource-group my-resource-group \
  --name nebula \
  --image myregistry.azurecr.io/nebula-production:latest \
  --dns-name-label nebula \
  --ports 8080 \
  --environment-variables NEBULA_ENV=production RUST_LOG=info \
  --cpu 1 \
  --memory 1.5
```

#### Azure Kubernetes Service (AKS)

Use the Kubernetes manifests above with AKS. Enable managed identity for secure access to Azure services.

## Configuration Management

### Environment-Based Configuration

```bash
# Development
NEBULA_ENV=development RUST_LOG=debug cargo run

# Staging
NEBULA_ENV=staging RUST_LOG=info ./nebula-production

# Production
NEBULA_ENV=production RUST_LOG=warn ./nebula-production
```

### Secrets Management

Never hardcode secrets. Use:

- **Docker**: Docker secrets or environment files
- **Kubernetes**: Secrets and External Secrets Operator
- **AWS**: AWS Secrets Manager or SSM Parameter Store
- **GCP**: Secret Manager
- **Azure**: Key Vault

Example with Kubernetes secrets:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: nebula-secrets
  namespace: nebula
type: Opaque
stringData:
  jwt-secret: "your-jwt-secret-here"
  api-key: "your-api-key-here"
```

## Monitoring and Observability

### Health Endpoints

- `/health` - Overall health status
- `/health/live` - Liveness probe (is the process running?)
- `/health/ready` - Readiness probe (is the service ready for traffic?)

### Metrics Endpoint

- `/metrics` - Prometheus-compatible metrics

### Log Aggregation

Configure structured JSON logging for production:

```bash
NEBULA_LOG_FORMAT=json RUST_LOG=info ./nebula-production
```

Ship logs to:
- **AWS**: CloudWatch Logs
- **GCP**: Cloud Logging
- **Azure**: Azure Monitor
- **Self-hosted**: ELK Stack, Loki

### Distributed Tracing

Enable OpenTelemetry tracing:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317 \
OTEL_SERVICE_NAME=nebula \
./nebula-production
```

## Scaling

### Horizontal Scaling

- **Docker**: Use Docker Swarm or multiple containers behind a load balancer
- **Kubernetes**: Use HPA (Horizontal Pod Autoscaler) based on CPU/memory or custom metrics
- **Cloud Run**: Automatic scaling based on request count

### Vertical Scaling

Adjust resource limits based on workload:

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

### Database Scaling

- Use connection pooling (configured via `NEBULA_POOL_*` environment variables)
- Consider read replicas for read-heavy workloads
- Implement caching with Redis

## Security Hardening

### TLS Configuration

```bash
NEBULA_TLS_ENABLED=true \
NEBULA_TLS_CERT_PATH=/app/certs/cert.pem \
NEBULA_TLS_KEY_PATH=/app/certs/key.pem \
NEBULA_TLS_MIN_VERSION=1.3
```

### API Key Authentication

```bash
NEBULA_API_KEY_ENABLED=true
```

### Rate Limiting

```bash
NEBULA_RATE_LIMIT_ENABLED=true \
NEBULA_RATE_LIMIT_REQUESTS=100 \
NEBULA_RATE_LIMIT_WINDOW_SECS=60
```

### Security Headers

All security headers are enabled by default:
- Strict-Transport-Security (HSTS)
- Content-Security-Policy (CSP)
- X-Content-Type-Options: nosniff
- X-Frame-Options: DENY
- X-XSS-Protection

## CI/CD Integration

### GitHub Actions

```yaml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Build and push Docker image
      uses: docker/build-push-action@v5
      with:
        context: .
        push: true
        tags: ${{ secrets.REGISTRY }}/nebula-production:${{ github.sha }}

    - name: Deploy to Kubernetes
      run: |
        kubectl apply -f k8s/
        kubectl rollout restart deployment/nebula -n nebula
```

## Troubleshooting

### Common Issues

1. **Port already in use**: Change `NEBULA_PORT` to a different value
2. **TLS certificate errors**: Verify certificate paths and permissions
3. **High memory usage**: Reduce cache size or connection pool limits
4. **Slow startup**: Check if TLS certificate loading is slow; consider pre-warming

### Debug Mode

Enable debug logging for troubleshooting:

```bash
RUST_LOG=debug NEBULA_LOG_FORMAT=pretty ./nebula-production
```

### Health Check Failures

Check the health endpoint response:

```bash
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready
```

## Performance Tuning

See [PERFORMANCE.md](./PERFORMANCE.md) for detailed performance optimization strategies.

## Support

For issues and questions:
- GitHub Issues: https://github.com/0xgetz/nebula-code/issues
- Discord: https://discord.gg/nebula-code
