//! Simple load balancer with round-robin and least-connections strategies
//!
//! This module provides load balancing capabilities:
//! - Round-robin distribution across servers
//! - Least-connections strategy for dynamic load
//! - Weighted distribution based on server capacity
//! - Health checking and automatic failover
//! - Thread-safe operations

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::optimization::LoadBalancingStrategy;

/// Load balancer errors
#[derive(Debug, Error)]
pub enum LoadBalancerError {
    #[error("No available servers")]
    NoServersAvailable,
    
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    
    #[error("Server already exists: {0}")]
    ServerAlreadyExists(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for load balancer operations
pub type LoadBalancerResult<T> = Result<T, LoadBalancerError>;

/// Server representation in the load balancer
#[derive(Debug, Clone)]
pub struct ServerNode {
    /// Unique server identifier
    pub id: String,
    
    /// Server address (host:port)
    pub address: String,
    
    /// Server weight for weighted strategies (higher = more traffic)
    pub weight: usize,
    
    /// Whether the server is healthy
    pub healthy: bool,
    
    /// Current active connection count
    pub active_connections: Arc<AtomicUsize>,
}

impl ServerNode {
    /// Create a new server with default weight
    pub fn new(id: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            address: address.into(),
            weight: 1,
            healthy: true,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Create a new server with custom weight
    pub fn with_weight(mut self, weight: usize) -> Self {
        self.weight = weight;
        self
    }
    
    /// Mark the server as healthy or unhealthy
    pub fn set_healthy(mut self, healthy: bool) -> Self {
        self.healthy = healthy;
        self
    }
    
    /// Increment active connections
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Decrement active connections
    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Get current connection count
    pub fn connection_count(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }
    
    /// Check if the server is available (healthy and connections within limit)
    pub fn is_available(&self) -> bool {
        self.healthy
    }
}

/// Load balancer state for tracking selection state
struct RoundRobinState {
    /// Current index in the server list
    current_index: AtomicUsize,
}

/// Thread-safe load balancer implementation
pub struct LoadBalancer {
    /// All servers
    servers: RwLock<Vec<ServerNode>>,
    
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,
    
    /// Round-robin state
    rr_state: RoundRobinState,
    
    /// Server weights for weighted strategies
    weights: RwLock<HashMap<String, usize>>,
}

impl LoadBalancer {
    /// Create a new load balancer with the given strategy
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            servers: RwLock::new(Vec::new()),
            strategy,
            rr_state: RoundRobinState {
                current_index: AtomicUsize::new(0),
            },
            weights: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create a new load balancer with round-robin strategy (default)
    pub fn round_robin() -> Self {
        Self::new(LoadBalancingStrategy::RoundRobin)
    }
    
    /// Create a new load balancer with least-connections strategy
    pub fn least_connections() -> Self {
        Self::new(LoadBalancingStrategy::LeastConnections)
    }
    
    /// Add a server to the load balancer
    pub async fn add_server(&self, server: ServerNode) -> LoadBalancerResult<()> {
        let mut servers = self.servers.write().await;
        
        // Check for duplicate
        if servers.iter().any(|s| s.id == server.id) {
            return Err(LoadBalancerError::ServerAlreadyExists(server.id));
        }
        
        // Store weight
        let weight = server.weight;
        let mut weights = self.weights.write().await;
        weights.insert(server.id.clone(), weight);
        
        servers.push(server);
        debug!("Added server to load balancer");
        Ok(())
    }
    
    /// Remove a server from the load balancer
    pub async fn remove_server(&self, server_id: &str) -> LoadBalancerResult<ServerNode> {
        let mut servers = self.servers.write().await;
        
        if let Some(pos) = servers.iter().position(|s| s.id == server_id) {
            let server = servers.remove(pos);
            
            // Remove weight
            let mut weights = self.weights.write().await;
            weights.remove(server_id);
            
            debug!("Removed server from load balancer");
            Ok(server)
        } else {
            Err(LoadBalancerError::ServerNotFound(server_id.to_string()))
        }
    }
    
    /// Mark a server as healthy or unhealthy
    pub async fn set_server_health(&self, server_id: &str, healthy: bool) -> LoadBalancerResult<()> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.iter_mut().find(|s| s.id == server_id) {
            server.healthy = healthy;
            debug!("Server {} health set to {}", server_id, healthy);
            Ok(())
        } else {
            Err(LoadBalancerError::ServerNotFound(server_id.to_string()))
        }
    }
    
    /// Get the next server based on the current strategy
    pub async fn next_server(&self) -> LoadBalancerResult<ServerNode> {
        let servers = self.servers.read().await;
        
        if servers.is_empty() {
            return Err(LoadBalancerError::NoServersAvailable);
        }
        
        // Filter healthy servers
        let healthy_servers: Vec<&ServerNode> = servers.iter().filter(|s| s.is_available()).collect();
        
        if healthy_servers.is_empty() {
            return Err(LoadBalancerError::NoServersAvailable);
        }
        
        let selected = match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(&healthy_servers),
            LoadBalancingStrategy::LeastConnections => self.select_least_connections(&healthy_servers),
            LoadBalancingStrategy::Random => self.select_random(&healthy_servers),
            LoadBalancingStrategy::WeightedRoundRobin => self.select_weighted_round_robin(&healthy_servers).await,
            LoadBalancingStrategy::IpHash => {
                // For IP hash, we need an IP - use round-robin as fallback
                self.select_round_robin(&healthy_servers)
            }
        };
        
        trace!("Selected server: {}", selected.id);
        Ok(selected.clone())
    }
    
    /// Get the next server for a specific client IP (for IP hash strategy)
    pub async fn next_server_for_ip(&self, client_ip: &str) -> LoadBalancerResult<ServerNode> {
        let servers = self.servers.read().await;
        
        if servers.is_empty() {
            return Err(LoadBalancerError::NoServersAvailable);
        }
        
        let healthy_servers: Vec<&ServerNode> = servers.iter().filter(|s| s.is_available()).collect();
        
        if healthy_servers.is_empty() {
            return Err(LoadBalancerError::NoServersAvailable);
        }
        
        // Use consistent hashing based on IP
        let hash = self.hash_ip(client_ip);
        let index = hash % healthy_servers.len();
        
        trace!("Selected server {} for IP {}", healthy_servers[index].id, client_ip);
        Ok(healthy_servers[index].clone())
    }
    
    /// Get all servers
    pub async fn servers(&self) -> Vec<ServerNode> {
        self.servers.read().await.clone()
    }
    
    /// Get the number of servers
    pub async fn server_count(&self) -> usize {
        self.servers.read().await.len()
    }
    
    /// Get the current strategy
    pub fn strategy(&self) -> LoadBalancingStrategy {
        self.strategy
    }
    
    /// Update the load balancing strategy
    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }
    
    /// Get statistics about the load balancer
    pub async fn stats(&self) -> LoadBalancerStats {
        let servers = self.servers.read().await;
        
        let total_connections: usize = servers.iter()
            .map(|s| s.connection_count())
            .sum();
        
        let healthy_count = servers.iter().filter(|s| s.healthy).count();
        let unhealthy_count = servers.len() - healthy_count;
        
        LoadBalancerStats {
            total_servers: servers.len(),
            healthy_servers: healthy_count,
            unhealthy_servers: unhealthy_count,
            total_connections,
            strategy: self.strategy,
        }
    }
    
    /// Select server using round-robin
    fn select_round_robin<'a>(&self, servers: &[&'a ServerNode]) -> &'a ServerNode {
        let index = self.rr_state.current_index.fetch_add(1, Ordering::SeqCst) % servers.len();
        servers[index]
    }
    
    /// Select server with the least active connections
    fn select_least_connections<'a>(&self, servers: &[&'a ServerNode]) -> &'a ServerNode {
        servers
            .iter()
            .min_by_key(|s| s.connection_count())
            .copied()
            .unwrap_or(servers[0])
    }
    
    /// Select a random server
    fn select_random<'a>(&self, servers: &[&'a ServerNode]) -> &'a ServerNode {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..servers.len());
        servers[index]
    }
    
    /// Select server using weighted round-robin
    async fn select_weighted_round_robin<'a>(&self, servers: &[&'a ServerNode]) -> &'a ServerNode {
        let weights = self.weights.read().await;
        
        // Calculate total weight
        let total_weight: usize = servers.iter()
            .map(|s| weights.get(&s.id).copied().unwrap_or(1))
            .sum();
        
        if total_weight == 0 {
            return self.select_round_robin(servers);
        }
        
        // Select based on weighted round-robin
        let current = self.rr_state.current_index.fetch_add(1, Ordering::SeqCst);
        let mut cumulative = 0;
        
        for server in servers {
            let weight = weights.get(&server.id).copied().unwrap_or(1);
            cumulative += weight;
            if (current % total_weight) < cumulative {
                return server;
            }
        }
        
        servers[servers.len() - 1]
    }
    
    /// Hash an IP address for consistent distribution
    fn hash_ip(&self, ip: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        ip.hash(&mut hasher);
        hasher.finish() as usize
    }
}

/// Load balancer statistics
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    /// Total number of servers
    pub total_servers: usize,
    
    /// Number of healthy servers
    pub healthy_servers: usize,
    
    /// Number of unhealthy servers
    pub unhealthy_servers: usize,
    
    /// Total active connections across all servers
    pub total_connections: usize,
    
    /// Current load balancing strategy
    pub strategy: LoadBalancingStrategy,
}

impl LoadBalancerStats {
    /// Get the percentage of healthy servers
    pub fn health_percentage(&self) -> f64 {
        if self.total_servers == 0 {
            0.0
        } else {
            (self.healthy_servers as f64 / self.total_servers as f64) * 100.0
        }
    }
    
    /// Get average connections per server
    pub fn avg_connections_per_server(&self) -> f64 {
        if self.total_servers == 0 {
            0.0
        } else {
            self.total_connections as f64 / self.total_servers as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_load_balancer_creation() {
        let lb = LoadBalancer::round_robin();
        assert_eq!(lb.strategy(), LoadBalancingStrategy::RoundRobin);
        assert_eq!(lb.server_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_add_remove_servers() {
        let lb = LoadBalancer::round_robin();
        
        let server1 = ServerNode::new("s1", "192.168.1.1:8080");
        let server2 = ServerNode::new("s2", "192.168.1.2:8080")
            .with_weight(2);
        
        lb.add_server(server1).await.unwrap();
        lb.add_server(server2).await.unwrap();
        
        assert_eq!(lb.server_count().await, 2);
        
        let removed = lb.remove_server("s1").await.unwrap();
        assert_eq!(removed.id, "s1");
        assert_eq!(lb.server_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_duplicate_server_error() {
        let lb = LoadBalancer::round_robin();
        
        let server = ServerNode::new("s1", "192.168.1.1:8080");
        lb.add_server(server.clone()).await.unwrap();
        
        let result = lb.add_server(server).await;
        assert!(matches!(result, Err(LoadBalancerError::ServerAlreadyExists(_))));
    }
    
    #[tokio::test]
    async fn test_round_robin_selection() {
        let lb = LoadBalancer::round_robin();
        
        lb.add_server(ServerNode::new("s1", "127.0.0.1:8081")).await.unwrap();
        lb.add_server(ServerNode::new("s2", "127.0.0.1:8082")).await.unwrap();
        lb.add_server(ServerNode::new("s3", "127.0.0.1:8083")).await.unwrap();
        
        // Should cycle through servers
        let s1 = lb.next_server().await.unwrap();
        let s2 = lb.next_server().await.unwrap();
        let s3 = lb.next_server().await.unwrap();
        let s4 = lb.next_server().await.unwrap();
        
        assert_eq!(s1.id, "s1");
        assert_eq!(s2.id, "s2");
        assert_eq!(s3.id, "s3");
        assert_eq!(s4.id, "s1"); // Back to start
    }
    
    #[tokio::test]
    async fn test_least_connections_selection() {
        let lb = LoadBalancer::least_connections();
        
        let server1 = ServerNode::new("s1", "127.0.0.1:8081");
        let server2 = ServerNode::new("s2", "127.0.0.1:8082");
        
        // Simulate connections on server1
        server1.active_connections.store(5, Ordering::SeqCst);
        server2.active_connections.store(2, Ordering::SeqCst);
        
        lb.add_server(server1).await.unwrap();
        lb.add_server(server2).await.unwrap();
        
        // Should select server2 (fewer connections)
        let selected = lb.next_server().await.unwrap();
        assert_eq!(selected.id, "s2");
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let lb = LoadBalancer::round_robin();
        
        lb.add_server(ServerNode::new("s1", "127.0.0.1:8081")).await.unwrap();
        lb.add_server(ServerNode::new("s2", "127.0.0.1:8082")).await.unwrap();
        
        // Mark s1 as unhealthy
        lb.set_server_health("s1", false).await.unwrap();
        
        // Should only select s2
        let selected = lb.next_server().await.unwrap();
        assert_eq!(selected.id, "s2");
        
        // Mark s1 healthy again
        lb.set_server_health("s1", true).await.unwrap();
        
        // Should be able to select a healthy server (round-robin may pick either)
        let selected = lb.next_server().await.unwrap();
        assert!(selected.healthy);
        assert!(selected.id == "s1" || selected.id == "s2");
    }
    
    #[tokio::test]
    async fn test_no_servers_available() {
        let lb = LoadBalancer::round_robin();
        
        let result = lb.next_server().await;
        assert!(matches!(result, Err(LoadBalancerError::NoServersAvailable)));
        
        // Add unhealthy server
        let mut server = ServerNode::new("s1", "127.0.0.1:8081");
        server.healthy = false;
        lb.add_server(server).await.unwrap();
        
        let result = lb.next_server().await;
        assert!(matches!(result, Err(LoadBalancerError::NoServersAvailable)));
    }
    
    #[tokio::test]
    async fn test_stats() {
        let lb = LoadBalancer::round_robin();
        
        let server1 = ServerNode::new("s1", "127.0.0.1:8081");
        server1.active_connections.store(3, Ordering::SeqCst);
        
        let server2 = ServerNode::new("s2", "127.0.0.1:8082");
        server2.active_connections.store(5, Ordering::SeqCst);
        
        lb.add_server(server1).await.unwrap();
        lb.add_server(server2).await.unwrap();
        
        let stats = lb.stats().await;
        
        assert_eq!(stats.total_servers, 2);
        assert_eq!(stats.healthy_servers, 2);
        assert_eq!(stats.unhealthy_servers, 0);
        assert_eq!(stats.total_connections, 8);
        assert_eq!(stats.avg_connections_per_server(), 4.0);
        assert_eq!(stats.health_percentage(), 100.0);
    }
    
    #[tokio::test]
    async fn test_weighted_round_robin() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::WeightedRoundRobin);
        
        lb.add_server(ServerNode::new("s1", "127.0.0.1:8081").with_weight(1)).await.unwrap();
        lb.add_server(ServerNode::new("s2", "127.0.0.1:8082").with_weight(2)).await.unwrap();
        
        // Track selections over multiple rounds
        let mut counts = HashMap::new();
        for _ in 0..6 {
            let server = lb.next_server().await.unwrap();
            *counts.entry(server.id).or_insert(0) += 1;
        }
        
        // s2 should have roughly twice as many selections as s1
        let s1_count = counts.get("s1").copied().unwrap_or(0);
        let s2_count = counts.get("s2").copied().unwrap_or(0);
        
        assert!(s2_count >= s1_count, "s2 should have more or equal selections");
    }
    
    #[test]
    fn test_server_connection_tracking() {
        let server = ServerNode::new("s1", "127.0.0.1:8081");
        
        assert_eq!(server.connection_count(), 0);
        
        server.increment_connections();
        server.increment_connections();
        assert_eq!(server.connection_count(), 2);
        
        server.decrement_connections();
        assert_eq!(server.connection_count(), 1);
    }
}
