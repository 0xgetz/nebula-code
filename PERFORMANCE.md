# Performance Optimization Guide

This guide covers performance optimization strategies for Nebula, including caching, connection pooling, load balancing, and resource tuning.

## Table of Contents

- [Caching Strategies](#caching-strategies)
- [Connection Pooling](#connection-pooling)
- [Load Balancing](#load-balancing)
- [Memory Optimization](#memory-optimization)
- [CPU Optimization](#cpu-optimization)
- [Network Optimization](#network-optimization)
- [Database Performance](#database-performance)
- [Monitoring and Profiling](#monitoring-and-profiling)
- [Benchmarking](#benchmarking)

## Caching Strategies

### In-Memory Caching

Use the built-in LRU cache for frequently accessed data:

```rust
use nebula_production::{CacheConfig, InMemoryCache, EvictionPolicy};

let config = CacheConfig::new()
    .with_max_size(10_000)           // Maximum entries
    .with_ttl_seconds(3600)          // 1 hour default TTL
    .with_eviction_policy(EvictionPolicy::Lru);

let cache = InMemoryCache::new(config);

// Store data
cache.set("user:123", user_data, Some(1800))?; // 30 minute TTL

// Retrieve data
if let Some(user) = cache.get("user:123")? {
    // Use cached data
}
```

### Cache Configuration

| Parameter | Description | Recommended Value |
|-----------|-------------|-------------------|
| `max_size` | Maximum cache entries | 10,000 - 100,000 |
| `ttl_seconds` | Default time-to-live | 300 - 3600 |
| `eviction_policy` | LRU, LFU, or TTL | LRU for most cases |

### Cache Invalidation Strategies

1. **Time-based (TTL)**: Automatically expire entries after a set time
2. **Write-through**: Update cache when data is written
3. **Write-behind**: Async cache updates for better write performance
4. **Invalidation events**: Explicitly invalidate on data changes

### Redis Integration

For distributed caching across multiple instances:

```bash
# Enable Redis caching
NEBULA_CACHE_ENABLED=true
NEBULA_REDIS_URL=redis://localhost:6379
NEBULA_CACHE_TTL_SECS=3600
```

### Cache Warming

Pre-populate cache for predictable load patterns:

```rust
async fn warm_cache(cache: &InMemoryCache) -> Result<(), CacheError> {
    // Load frequently accessed data
    let popular_users = fetch_popular_users().await?;
    for user in popular_users {
        cache.set(format!("user:{}", user.id), user, Some(3600))?;
    }
    Ok(())
}
```

## Connection Pooling

### Database Connection Pool

Configure connection pooling for optimal database performance:

```rust
use nebula_production::{PoolConfig, ConnectionPool};

let config = PoolConfig::new()
    .with_min_connections(5)      // Minimum idle connections
    .with_max_connections(20)     // Maximum connections
    .with_timeout_seconds(30)     // Connection timeout
    .with_idle_timeout_seconds(300) // Close idle after 5 min
    .with_health_check_interval(60); // Health check every 60s

let pool = ConnectionPool::new(config, factory).await?;
```

### Pool Sizing

| Workload | Min Connections | Max Connections |
|----------|-----------------|-----------------|
| Low traffic | 2 | 10 |
| Medium traffic | 5 | 25 |
| High traffic | 10 | 50 |
| Batch processing | 1 | 100 |

### HTTP Client Pooling

Reuse HTTP connections for external API calls:

```rust
use nebula_production::{PoolConfig, ConnectionFactory};

struct HttpClientFactory;

impl ConnectionFactory for HttpClientFactory {
    type Connection = reqwest::Client;
    type Error = reqwest::Error;

    async fn create(&self) -> Result<Self::Connection, Self::Error> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
    }

    async fn is_valid(&self, conn: &Self::Connection) -> bool {
        true // reqwest clients are always valid
    }
}
```

## Load Balancing

### Load Balancing Strategies

```rust
use nebula_production::{LoadBalancingStrategy, PerformanceConfig};

let config = PerformanceConfig::new()
    .with_load_balancing(LoadBalancingStrategy::LeastConnections);
```

| Strategy | Use Case |
|----------|----------|
| RoundRobin | Uniform request distribution |
| LeastConnections | Variable request duration |
| Weighted | Heterogeneous server capacity |
| Random | Simple distribution with some variance |

### Horizontal Scaling

Scale horizontally by adding more instances:

```yaml
# Kubernetes HPA configuration
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: nebula-hpa
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
```

## Memory Optimization

### Memory-Efficient Data Structures

Use efficient data structures for large datasets:

```rust
// Use Box for large types to reduce stack usage
let large_data = Box::new(expensive_struct);

// Use Arc for shared ownership without cloning
let shared_data = Arc::new(data);

// Use Cow for copy-on-write semantics
let data: Cow<str> = Cow::Borrowed("initial");
```

### Memory Limits

Set memory limits to prevent OOM:

```bash
# Kubernetes memory limits
resources:
  limits:
    memory: "512Mi"
  requests:
    memory: "256Mi"
```

### Garbage Collection (Rust-specific)

Rust has no GC, but optimize memory allocation:

1. **Pre-allocate vectors**: `Vec::with_capacity(n)`
2. **Use iterators**: Avoid intermediate collections
3. **Reuse buffers**: Use `clear()` instead of reallocating
4. **Avoid unnecessary cloning**: Use references and `Copy` types

## CPU Optimization

### Async Runtime Tuning

Optimize Tokio runtime settings:

```rust
use tokio::runtime::Builder;

let runtime = Builder::new_multi_thread()
    .worker_threads(num_cpus::get())  // One thread per CPU core
    .max_blocking_threads(512)        // For blocking operations
    .thread_stack_size(2 * 1024 * 1024) // 2MB stack
    .enable_all()
    .build()?;
```

### Parallel Processing

Use parallel processing for CPU-intensive tasks:

```rust
use rayon::prelude::*;

let results: Vec<_> = data.par_iter()
    .map(|item| process_item(item))
    .collect();
```

### Compile-Time Optimizations

Enable release optimizations:

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
opt-level = 3        # Maximum optimization
strip = true         # Strip debug symbols
```

## Network Optimization

### HTTP Keep-Alive

Enable connection reuse:

```rust
// Axum/Tower configuration
let app = Router::new()
    // ... routes
    .layer(
        tower_http::trace::TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::default())
            .on_response(DefaultOnResponse::new()),
    );
```

### Compression

Enable response compression:

```rust
use tower_http::compression::CompressionLayer;

let app = Router::new()
    // ... routes
    .layer(CompressionLayer::new()
        .gzip(true)
        .deflate(true)
        .br(true));
```

### Connection Timeouts

Set appropriate timeouts:

```rust
use std::time::Duration;

let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(30))
    .pool_idle_timeout(Duration::from_secs(90))
    .pool_max_idle_per_host(10)
    .build()?;
```

## Database Performance

### Query Optimization

1. **Use indexes**: Add indexes for frequently queried columns
2. **Select only needed columns**: Avoid `SELECT *`
3. **Use prepared statements**: Prevent SQL injection and improve performance
4. **Batch operations**: Group multiple operations together

### Connection Pool Configuration

```bash
# Environment variables
NEBULA_POOL_MIN_CONNECTIONS=5
NEBULA_POOL_MAX_CONNECTIONS=20
NEBULA_POOL_TIMEOUT_SECS=30
NEBULA_POOL_IDLE_TIMEOUT_SECS=300
```

### Read Replicas

For read-heavy workloads, use read replicas:

```rust
// Route reads to replicas, writes to primary
let db = if is_read_operation {
    replica_pool.get().await?
} else {
    primary_pool.get().await?
};
```

## Monitoring and Profiling

### Metrics Collection

Enable Prometheus metrics:

```bash
NEBULA_METRICS_ENABLED=true
```

Key metrics to monitor:

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `http_requests_total` | Total HTTP requests | - |
| `http_request_duration_seconds` | Request latency | p99 > 1s |
| `active_connections` | Current connections | > 80% of max |
| `cache_hits_total` | Cache hit count | - |
| `cache_misses_total` | Cache miss count | Miss rate > 20% |
| `pool_available_connections` | Available pool connections | < 20% of max |
| `errors_total` | Error count | Error rate > 1% |

### Distributed Tracing

Enable tracing for performance analysis:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
OTEL_SERVICE_NAME=nebula
OTEL_TRACES_SAMPLER=parentbased_traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1  # Sample 10% of requests
```

### Profiling

Use profiling tools to identify bottlenecks:

```bash
# CPU profiling with perf
perf record -F 99 -p <pid> -- sleep 30
perf report

# Memory profiling with jemalloc
MALLOC_CONF="prof:true,prof_active:true,lg_prof_sample:19" ./nebula-production

# Flamegraph generation
cargo flamegraph --bin nebula-production
```

## Benchmarking

### Load Testing

Use tools like `wrk` or `vegeta` for load testing:

```bash
# Install wrk
apt-get install wrk

# Run benchmark
wrk -t12 -c400 -d30s http://localhost:8080/api/endpoint
```

### Benchmark Configuration

```yaml
# Example benchmark configuration
target: "http://localhost:8080"
concurrency: 100
duration: 60s
ramp_up: 10s
```

### Performance Baselines

Establish performance baselines:

| Metric | Target |
|--------|--------|
| p50 latency | < 50ms |
| p95 latency | < 200ms |
| p99 latency | < 500ms |
| Error rate | < 0.1% |
| Throughput | > 1000 req/s |

## Performance Checklist

Before production deployment:

- [ ] Caching configured with appropriate TTL
- [ ] Connection pool sized correctly
- [ ] Load balancing strategy selected
- [ ] Compression enabled
- [ ] Timeouts configured
- [ ] Metrics collection enabled
- [ ] Tracing configured
- [ ] Release build optimizations enabled
- [ ] Load testing completed
- [ ] Performance baselines established

## Common Performance Issues

### High Latency

**Causes:**
- Slow database queries
- Network latency
- Insufficient resources
- Lock contention

**Solutions:**
- Add database indexes
- Enable caching
- Scale horizontally
- Optimize critical paths

### High Memory Usage

**Causes:**
- Memory leaks
- Large cache size
- Inefficient data structures
- Insufficient garbage collection (not applicable to Rust)

**Solutions:**
- Profile memory usage
- Reduce cache size
- Use efficient data structures
- Set memory limits

### High CPU Usage

**Causes:**
- Inefficient algorithms
- Lack of parallelism
- Excessive logging
- Hot loops

**Solutions:**
- Profile CPU usage
- Use parallel processing
- Reduce log verbosity
- Optimize hot paths

### Connection Pool Exhaustion

**Causes:**
- Pool too small
- Connection leaks
- Long-running queries
- High concurrency

**Solutions:**
- Increase pool size
- Fix connection leaks
- Optimize queries
- Implement queuing

## Resources

- [Tokio Performance Tips](https://tokio.rs/tokio/topics/shutdown)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
