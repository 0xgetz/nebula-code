# Changelog - Phase 5: Ecosystem Expansion

## [5.0.0] - 2026-04-02

### Breaking Changes
- **Authentication**: Deprecated API key authentication in favor of OAuth 2.0 / OIDC
- **Database**: PostgreSQL 14 or higher required (15 recommended)
- **Configuration**: Moved from `.env` files to centralized config management (Vault integration)
- **API**: Changed base URL from `/api/v1` to `/v1` for all REST endpoints
- **Agent SDK**: Python agent runtime now requires Python 3.10+
- **Deployment**: Docker Compose files restructured; old format no longer supported

### Added
- **Enterprise SSO**: SAML 2.0 and OIDC integration with Okta, Azure AD, Ping Identity
- **SCIM Provisioning**: Automated user provisioning and deprovisioning
- **Advanced RBAC**: Fine-grained permissions with resource-level access control
- **Audit Logging**: Comprehensive logging of all authentication and authorization events
- **Multi-Factor Authentication**: Enforce MFA for sensitive operations
- **Observability**: Prometheus metrics, Grafana dashboards, OpenTelemetry tracing
- **Multi-Cloud Support**: Native deployment on AWS, GCP, Azure, and on-premises
- **Kubernetes Operator**: Custom resource definitions for Nebula services
- **Edge Computing**: Lightweight agent runtime for edge deployments
- **Infrastructure as Code**: Terraform modules for all supported clouds
- **Encryption**: End-to-end encryption with TLS 1.3 and AES-256 at rest
- **Secrets Management**: HashiCorp Vault integration for secure secret storage
- **Compliance**: SOC 2 Type II, ISO 27001, GDPR readiness reports
- **Data Loss Prevention**: Configurable DLP policies for sensitive data
- **SIEM Integration**: Forward logs to Splunk, Datadog, or Elastic SIEM

### Changed
- **Performance**: 40% improvement in agent execution speed through Rust optimizations
- **Memory**: Reduced memory footprint by 30% with improved caching strategies
- **Startup**: 60% faster cold start times for serverless deployments
- **Rate Limiting**: Adaptive rate limiting based on system load
- **Error Handling**: More descriptive error messages with actionable suggestions
- **Documentation**: Complete rewrite with interactive examples

### Improved
- **Monitoring**: Real-time health checks and automated alerting
- **Resilience**: Circuit breaker pattern for external service calls
- **Deployment**: One-command Helm chart deployment
- **Upgrades**: Zero-downtime rolling upgrades
- **Backup**: Automated daily backups with point-in-time recovery
- **Testing**: Enhanced integration test suite with 95% coverage

### Fixed
- Memory leak in long-running agent processes
- Race condition in concurrent tool execution
- Incorrect timezone handling in scheduled triggers
- UI freezing during large file uploads
- Incomplete cleanup of temporary files
- Token refresh failures under high load
- Inconsistent state after network partitions
- Missing audit events for admin actions
