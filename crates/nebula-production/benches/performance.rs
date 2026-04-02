//! Performance benchmarks for nebula-production crate
//!
//! Benchmarks cover:
//! - Cache operations (LRU, TTL, eviction)
//! - Connection pool operations
//! - Load balancer strategies

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use nebula_production::cache::{CacheConfig, EvictionPolicy, InMemoryCache};
use nebula_production::load_balancer::{LoadBalancer, Server};
use nebula_production::optimization::LoadBalancingStrategy;
use nebula_production::pool::{ConnectionFactory, PoolConfig, PoolError, PoolResult, PooledConnection, ConnectionPool};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Simple in-memory cache benchmarks
fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");
    
    // Benchmark insert operations
    group.bench_function("insert", |b| {
        b.iter(|| {
            let mut cache: InMemoryCache<String, String> = InMemoryCache::new(1000);
            for i in 0..100 {
                cache.insert(format!("key_{}", i), format!("value_{}", i));
            }
        });
    });
    
    // Benchmark get operations (high hit rate)
    group.bench_function("get_high_hit_rate", |b| {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(1000);
        for i in 0..100 {
            cache.insert(format!("key_{}", i), format!("value_{}", i));
        }
        
        b.iter(|| {
            // 90% hit rate - mostly accessing existing keys
            for i in 0..100 {
                let key = if i < 90 {
                    format!("key_{}", i % 100)
                } else {
                    format!("missing_{}", i)
                };
                cache.get(&key);
            }
        });
    });
    
    // Benchmark get operations (low hit rate)
    group.bench_function("get_low_hit_rate", |b| {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(1000);
        for i in 0..100 {
            cache.insert(format!("key_{}", i), format!("value_{}", i));
        }
        
        b.iter(|| {
            // 10% hit rate - mostly accessing missing keys
            for i in 0..100 {
                let key = if i < 10 {
                    format!("key_{}", i)
                } else {
                    format!("missing_{}", i)
                };
                cache.get(&key);
            }
        });
    });
    
    // Benchmark eviction policies with different capacities
    for capacity in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("lru_eviction", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let mut cache: InMemoryCache<i32, String> = InMemoryCache::new(cap)
                        .with_policy(EvictionPolicy::LRU);
                    
                    // Fill beyond capacity to trigger evictions
                    for i in 0..(cap * 2) {
                        cache.insert(i, format!("value_{}", i));
                        // Access some keys to update LRU
                        if i % 3 == 0 {
                            cache.get(&(i / 2));
                        }
                    }
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("lfu_eviction", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let mut cache: InMemoryCache<i32, String> = InMemoryCache::new(cap)
                        .with_policy(EvictionPolicy::LFU);
                    
                    for i in 0..(cap * 2) {
                        cache.insert(i, format!("value_{}", i));
                        // Access some keys multiple times to update LFU
                        for _ in 0..(i % 5) {
                            cache.get(&(i / 2));
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Connection pool benchmarks
fn bench_pool_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool");
    
    // Benchmark pool creation with different sizes
    for (min, max) in [(2, 10), (5, 20), (10, 50)] {
        group.bench_with_input(
            BenchmarkId::new("pool_creation", format!("{}x{}", min, max)),
            &(min, max),
            |b, &(min, max)| {
                b.iter(|| {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let factory = MockConnectionFactory::new();
                        let config = PoolConfig::new(min, max);
                        let _pool = ConnectionPool::new(config, factory).await.unwrap();
                    });
                });
            },
        );
    }
    
    // Benchmark connection borrowing
    group.bench_function("borrow_connection", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let pool = runtime.block_on(async {
            let factory = MockConnectionFactory::new();
            let config = PoolConfig::new(5, 20);
            ConnectionPool::new(config, factory).await.unwrap()
        });
        
        b.iter(|| {
            runtime.block_on(async {
                let _conn = pool.get().await.unwrap();
                // Connection returned on drop
            });
        });
    });
    
    // Benchmark concurrent borrowing
    group.bench_function("concurrent_borrow", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let pool = runtime.block_on(async {
            let factory = MockConnectionFactory::new();
            let config = PoolConfig::new(5, 50);
            ConnectionPool::new(config, factory).await.unwrap()
        });
        
        b.iter(|| {
            runtime.block_on(async {
                let mut handles = vec![];
                for _ in 0..10 {
                    let pool = pool.clone();
                    handles.push(tokio::spawn(async move {
                        let _conn = pool.get().await.unwrap();
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });
    
    group.finish();
}

/// Load balancer benchmarks
fn bench_load_balancer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_balancer");
    
    // Benchmark server selection with different strategies
    for strategy in [
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::Random,
        LoadBalancingStrategy::WeightedRoundRobin,
    ] {
        group.bench_with_input(
            BenchmarkId::new("select_server", format!("{:?}", strategy)),
            &strategy,
            |b, &strategy| {
                b.iter(|| {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let lb = LoadBalancer::new(strategy);
                        
                        // Add 10 servers
                        for i in 0..10 {
                            let server = Server::new(
                                format!("server_{}", i),
                                format!("192.168.1.{}:8080", i + 1),
                            ).with_weight((i % 3) + 1);
                            lb.add_server(server).await.unwrap();
                        }
                        
                        // Select server multiple times
                        for _ in 0..100 {
                            let _server = lb.next_server().await.unwrap();
                        }
                    });
                });
            },
        );
    }
    
    // Benchmark with different numbers of servers
    for num_servers in [5, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("many_servers_round_robin", num_servers),
            &num_servers,
            |b, &n| {
                b.iter(|| {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let lb = LoadBalancer::round_robin();
                        
                        for i in 0..n {
                            let server = Server::new(
                                format!("server_{}", i),
                                format!("192.168.1.{}:8080", i + 1),
                            );
                            lb.add_server(server).await.unwrap();
                        }
                        
                        for _ in 0..100 {
                            let _server = lb.next_server().await.unwrap();
                        }
                    });
                });
            },
        );
    }
    
    // Benchmark least connections with varying load
    group.bench_function("least_connections_varying_load", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let lb = LoadBalancer::least_connections();
                
                for i in 0..10 {
                    let mut server = Server::new(
                        format!("server_{}", i),
                        format!("192.168.1.{}:8080", i + 1),
                    );
                    // Simulate different load levels
                    server.active_connections.store(i * 5, Ordering::SeqCst);
                    lb.add_server(server).await.unwrap();
                }
                
                for _ in 0..100 {
                    let _server = lb.next_server().await.unwrap();
                }
            });
        });
    });
    
    group.finish();
}

// Mock connection for pool benchmarks
struct MockConnection {
    id: String,
    valid: Arc<AtomicU64>,
}

impl MockConnection {
    fn new(id: String, valid: Arc<AtomicU64>) -> Self {
        Self { id, valid }
    }
}

#[async_trait::async_trait]
impl PooledConnection for MockConnection {
    async fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst) == 1
    }
    
    async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    
    fn id(&self) -> String {
        self.id.clone()
    }
}

struct MockConnectionFactory {
    valid: Arc<AtomicU64>,
    counter: AtomicU64,
}

impl MockConnectionFactory {
    fn new() -> Self {
        Self {
            valid: Arc::new(AtomicU64::new(1)),
            counter: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl ConnectionFactory for MockConnectionFactory {
    type Connection = MockConnection;
    
    async fn create(&self) -> Result<Self::Connection, Box<dyn std::error::Error + Send + Sync>> {
        let id = format!("conn-{}", self.counter.fetch_add(1, Ordering::SeqCst));
        Ok(MockConnection::new(id, Arc::clone(&self.valid)))
    }
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_pool_operations,
    bench_load_balancer_operations,
);
criterion_main!(benches);
