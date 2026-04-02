//! Multi-Agent Collaboration Example
//!
//! This example demonstrates production patterns for building a multi-agent
//! collaboration system using nebula-production components. It showcases:
//!
//! - Configuration management with environment-aware settings
//! - Security hardening with TLS, API keys, and JWT
//! - Health checking for Kubernetes deployment
//! - Rate limiting and connection pooling
//! - Structured logging and metrics
//! - Authentication and authorization with RBAC
//!
//! Run with: `cargo run -p nebula-production --example multi_agent_collaboration`

use nebula_production::{ config::LoggingConfig,
    ApiKeyConfig, ApiKeyEntry, AuthMiddleware,
    Authorization, Environment, HealthCheck,
    HealthChecker, InMemoryAuthorization, JwtAlgorithm,
    JwtConfig, MetricsCollector, PoolConfig, ProductionConfig, RbacConfig,
    RateLimiter, RateLimiterConfig, RateLimitTier, RateLimitKeyStrategy, RoleDefinition,
    SecurityConfig, ServerConfigInner, 
    hash_password, verify_password, init_logging,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use base64::Engine;

/// Represents an agent in the collaboration system
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent identifier
    pub id: String,
    /// Agent name for display
    pub name: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Current status
    pub status: AgentStatus,
}

/// Agent status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    /// Agent is available
    Available,
    /// Agent is busy processing a task
    Busy,
    /// Agent is offline
    Offline,
    /// Agent is in maintenance mode
    Maintenance,
}

/// Task request between agents
#[derive(Debug, Clone)]
pub struct TaskRequest {
    /// Requesting agent ID
    pub from_agent: String,
    /// Target agent ID
    pub to_agent: String,
    /// Task description
    pub task: String,
    /// Priority level (1 = highest)
    pub priority: u8,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Agent registry for managing available agents
pub struct AgentRegistry {
    /// Registered agents
    agents: RwLock<HashMap<String, Agent>>,
    /// Authorization system
    auth: Arc<dyn Authorization>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new(auth: Arc<dyn Authorization>) -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            auth,
        }
    }

    /// Register a new agent
    pub async fn register_agent(&self, agent: Agent) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        if agents.contains_key(&agent.id) {
            return Err(format!("Agent {} already registered", agent.id));
        }
        agents.insert(agent.id.clone(), agent);
        info!("Agent registered successfully");
        Ok(())
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        self.agents.read().await.get(agent_id).cloned()
    }

    /// Get all available agents
    pub async fn get_available_agents(&self) -> Vec<Agent> {
        self.agents
            .read()
            .await
            .values()
            .filter(|a| a.status == AgentStatus::Available)
            .cloned()
            .collect()
    }

    /// Update agent status
    pub async fn update_status(&self, agent_id: &str, status: AgentStatus) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            let old_status = agent.status.clone();
            agent.status = status;
            info!(
                agent_id = agent_id,
                old_status = ?old_status,
                new_status = ?agent.status,
                "Agent status updated"
            );
            Ok(())
        } else {
            Err(format!("Agent {} not found", agent_id))
        }
    }

    /// Check if agent has permission to perform action
    pub async fn check_permission(&self, agent_id: &str, permission: &str) -> bool {
        self.auth.has_permission(agent_id, permission).await
    }
}

/// Health check for agent registry
pub struct RegistryHealthCheck {
    registry: Arc<AgentRegistry>,
}

impl RegistryHealthCheck {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait::async_trait]
impl HealthCheck for RegistryHealthCheck {
    fn name(&self) -> &str {
        "agent_registry"
    }

    async fn check(&self) -> nebula_production::health::HealthCheckResult {
        let available = self.registry.get_available_agents().await;
        
        if available.is_empty() {
            nebula_production::health::HealthCheckResult::unhealthy("agent_registry", "No agents available")
        } else {
            nebula_production::health::HealthCheckResult::healthy("agent_registry")
        }
    }
}

/// Configuration for the multi-agent system
#[derive(Debug, Clone)]
pub struct MultiAgentConfig {
    /// Production configuration
    pub production: ProductionConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Rate limiter configuration
    pub rate_limiter: RateLimiterConfig,
    /// Pool configuration for agent connections
    pub pool: PoolConfig,
}

impl MultiAgentConfig {
    /// Create production configuration
    pub fn production() -> Self {
        Self {
            production: ProductionConfig {
                environment: Environment::Production,
                app_name: "nebula-multi-agent".to_string(),
                app_version: "1.0.0".to_string(),
                server: ServerConfigInner {
                    host: "0.0.0.0".to_string(),
                    port: 8080,
                    shutdown_timeout_secs: 30,
                },
                ..Default::default()
            },
            security: SecurityConfig::production(),
            jwt: JwtConfig {
                enabled: true,
                algorithm: JwtAlgorithm::Hs256,
                secret: Some(base64::engine::general_purpose::STANDARD.encode("multi-agent-secret-key")),
                issuer: Some("nebula-multi-agent".to_string()),
                audience: Some("nebula-agents".to_string()),
                expiration_leeway_seconds: 60,
                required_claims: vec!["sub".to_string(), "exp".to_string(), "roles".to_string()],
                ..Default::default()
            },
            rate_limiter: RateLimiterConfig {
                default_tier: RateLimitTier {
                    name: "default".to_string(),
                    max_requests: 1000,
                    window_seconds: 60,
                    burst_capacity: Some(2000),
                },
                tiers: vec![],
                enabled: true,
                key_strategy: RateLimitKeyStrategy::IpAddress,
                bypass_clients: vec![],
            },
            pool: PoolConfig::new(10, 50)
                .with_connection_timeout(5000)
                .with_max_lifetime(3600)
                .with_max_idle_time(600),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.jwt.enabled && self.jwt.secret.is_none() {
            return Err("JWT enabled but no secret provided".to_string());
        }
        Ok(())
    }
}

/// Initialize the multi-agent collaboration system
pub async fn initialize_system() -> Result<(Arc<AgentRegistry>, MultiAgentConfig), Box<dyn std::error::Error>> {
    // Load configuration
    let config = MultiAgentConfig::production();
    config.validate()?;

    // Initialize logging from config
    let logging_config = LoggingConfig {
        level: config.production.logging.level.clone(),
        format: config.production.logging.format.clone(),
    };
    init_logging(&logging_config)?;

    // Set up RBAC
    let mut roles = HashMap::new();
    roles.insert("coordinator".to_string(), RoleDefinition {
        name: "coordinator".to_string(),
        description: "Agent coordinator with full permissions".to_string(),
        permissions: vec![
            "agent:register".to_string(),
            "agent:unregister".to_string(),
            "task:assign".to_string(),
            "task:cancel".to_string(),
            "system:health".to_string(),
            "*".to_string(),
        ],
        inherits: vec!["agent".to_string()],
    });
    roles.insert("agent".to_string(), RoleDefinition {
        name: "agent".to_string(),
        description: "Standard agent with basic permissions".to_string(),
        permissions: vec![
            "task:receive".to_string(),
            "task:execute".to_string(),
            "task:report".to_string(),
            "agent:status".to_string(),
        ],
        inherits: vec![],
    });
    roles.insert("observer".to_string(), RoleDefinition {
        name: "observer".to_string(),
        description: "Read-only access".to_string(),
        permissions: vec![
            "agent:list".to_string(),
            "task:view".to_string(),
            "system:health".to_string(),
        ],
        inherits: vec![],
    });

    let rbac_config = RbacConfig {
        roles,
        permissions: HashMap::new(),
        default_role: "agent".to_string(),
        admin_role: "coordinator".to_string(),
    };

    let auth: Arc<dyn Authorization> = Arc::new(InMemoryAuthorization::new(rbac_config));

    // Create agent registry
    let registry = Arc::new(AgentRegistry::new(auth));

    // Register some default agents
    let coordinator = Agent {
        id: "coordinator-1".to_string(),
        name: "Coordinator".to_string(),
        capabilities: vec!["orchestration".to_string(), "scheduling".to_string()],
        status: AgentStatus::Available,
    };

    let worker1 = Agent {
        id: "worker-1".to_string(),
        name: "Worker 1".to_string(),
        capabilities: vec!["computation".to_string(), "data-processing".to_string()],
        status: AgentStatus::Available,
    };

    let worker2 = Agent {
        id: "worker-2".to_string(),
        name: "Worker 2".to_string(),
        capabilities: vec!["io-operations".to_string(), "file-handling".to_string()],
        status: AgentStatus::Available,
    };

    registry.register_agent(coordinator).await?;
    registry.register_agent(worker1).await?;
    registry.register_agent(worker2).await?;

    info!("Multi-agent system initialized with 3 agents");

    Ok((registry, config))
}

/// Demonstrate security setup
pub async fn setup_security() -> Result<(SecurityConfig, JwtConfig, Arc<AuthMiddleware>), Box<dyn std::error::Error>> {
    let security = SecurityConfig::production();
    let jwt = JwtConfig {
        enabled: true,
        algorithm: JwtAlgorithm::Hs256,
        secret: Some(base64::engine::general_purpose::STANDARD.encode("agent-system-jwt-secret")),
        issuer: Some("nebula-multi-agent".to_string()),
        audience: Some("agents".to_string()),
        expiration_leeway_seconds: 300, // 5 minutes
        required_claims: vec!["sub".to_string(), "exp".to_string()],
        ..Default::default()
    };

    // Set up API key authentication
    let api_key_config = ApiKeyConfig {
        enabled: true,
        header_name: "X-Agent-API-Key".to_string(),
        keys: vec![
            ApiKeyEntry {
                id: "coordinator-key".to_string(),
                hash: base64::engine::general_purpose::STANDARD.encode("coordinator-secret-key"),
                name: "Coordinator API Key".to_string(),
                roles: vec!["coordinator".to_string()],
                expires_at: None,
                created_at: chrono::Utc::now().timestamp(),
            },
            ApiKeyEntry {
                id: "worker-key".to_string(),
                hash: base64::engine::general_purpose::STANDARD.encode("worker-secret-key"),
                name: "Worker API Key".to_string(),
                roles: vec!["agent".to_string()],
                expires_at: None,
                created_at: chrono::Utc::now().timestamp(),
            },
        ],
        rotation_interval_hours: Some(24 * 90), // 90 days
    };

    // Create auth middleware
    let auth = Arc::new(InMemoryAuthorization::new(RbacConfig::default()));
    let middleware = Arc::new(AuthMiddleware::new(
        Some(api_key_config),
        Some(jwt.clone()),
        auth,
    )?);

    Ok((security, jwt, middleware))
}

/// Demonstrate password hashing for agent credentials
pub fn demonstrate_password_security() -> Result<(), Box<dyn std::error::Error>> {
    let password = "agent-secure-password-123";
    let hash = hash_password(password)?;
    
    println!("Password hashed successfully");
    println!("Original password: {}", password);
    println!("Hash: {}", hash);
    
    assert!(verify_password(password, &hash)?);
    assert!(!verify_password("wrong-password", &hash)?);
    
    println!("Password verification: PASSED");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Nebula Multi-Agent Collaboration System ===\n");

    // Initialize the system
    println!("1. Initializing multi-agent system...");
    let (registry, config) = initialize_system().await?;
    println!("   System initialized with configuration:");
    println!("   - Environment: {:?}", config.production.environment);
    println!("   - App: {} v{}", config.production.app_name, config.production.app_version);
    println!("   - Server: {}:{}\n", config.production.server.host, config.production.server.port);

    // Demonstrate password security
    println!("2. Demonstrating password security...");
    demonstrate_password_security()?;
    println!();

    // Set up security
    println!("3. Setting up security...");
    let (security, jwt, _auth_middleware) = setup_security().await?;
    println!("   Security configured:");
    println!("   - TLS enabled: {}", security.tls.enabled);
    println!("   - TLS version: {:?}", security.tls.min_version);
    println!("   - JWT enabled: {}", jwt.enabled);
    println!("   - JWT algorithm: {:?}", jwt.algorithm);
    println!();

    // Check agent availability
    println!("4. Checking agent availability...");
    let available_agents = registry.get_available_agents().await;
    println!("   Available agents: {}", available_agents.len());
    for agent in &available_agents {
        println!("   - {} ({}) - Capabilities: {:?}", 
            agent.name, agent.id, agent.capabilities);
    }
    println!();

    // Set up health checking
    println!("5. Setting up health checks...");
    let health_checker = HealthChecker::new();
    health_checker.register(Arc::new(RegistryHealthCheck::new(registry.clone()))).await;
    
    let health_response = health_checker.check_all().await;
    println!("   Health status: {:?}", health_response.status);
    for check in &health_response.checks {
        println!("   - {}: {:?}", check.name, check.status);
    }
    println!();

    // Set up metrics
    println!("6. Setting up metrics collection...");
    let _metrics = MetricsCollector::new();
    println!("   Metrics collector initialized");
    println!();

    // Demonstrate rate limiting
    println!("7. Demonstrating rate limiting...");
    let rate_limiter = RateLimiter::new(config.rate_limiter.clone())?;
    
    for i in 1..=5 {
        let result = rate_limiter.check("test-client", None).await;
        if result.allowed {
            println!("   Request {}: Allowed (remaining: {})", i, result.remaining);
        } else {
            println!("   Request {}: Rate limited (retry after {:?}s)", i, result.retry_after);
        }
    }
    println!();

    // Permission checking demonstration
    println!("8. Checking agent permissions...");
    let coordinator = registry.get_agent("coordinator-1").await;
    if let Some(agent) = coordinator {
        let can_register = registry.check_permission(&agent.id, "agent:register").await;
        let can_execute = registry.check_permission(&agent.id, "task:execute").await;
        println!("   Coordinator permissions:");
        println!("   - agent:register: {}", can_register);
        println!("   - task:execute: {}", can_execute);
    }
    
    let worker = registry.get_agent("worker-1").await;
    if let Some(agent) = worker {
        let can_register = registry.check_permission(&agent.id, "agent:register").await;
        let can_execute = registry.check_permission(&agent.id, "task:execute").await;
        println!("   Worker permissions:");
        println!("   - agent:register: {}", can_register);
        println!("   - task:execute: {}", can_execute);
    }
    println!();

    println!("=== Multi-Agent System Demo Complete ===");
    println!();
    println!("Next steps:");
    println!("1. Configure TLS certificates for production deployment");
    println!("2. Set up proper JWT secrets and key rotation");
    println!("3. Configure database connection pooling");
    println!("4. Deploy to Kubernetes with health checks");
    println!("5. Set up Prometheus metrics scraping");

    Ok(())
}
