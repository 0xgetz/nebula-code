use async_trait::async_trait;
use crate::agent::{Agent, AgentContext, AgentError, AgentOutput, AgentResult};
use crate::architect::ArchitectAgent;
use crate::coder::CoderAgent;
use crate::tester::TesterAgent;
use crate::reviewer::ReviewerAgent;
use crate::deployer::DeployerAgent;

/// Agent Orchestrator - coordinates multiple agents for complex workflows
pub struct AgentOrchestrator {
    agents: Vec<Box<dyn Agent>>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        let agents: Vec<Box<dyn Agent>> = vec![
            Box::new(ArchitectAgent::new()),
            Box::new(CoderAgent::new()),
            Box::new(TesterAgent::new()),
            Box::new(ReviewerAgent::new()),
            Box::new(DeployerAgent::new()),
        ];
        Self { agents }
    }
    
    /// Execute a workflow with multiple agents
    pub async fn execute_workflow(
        &self,
        task: &str,
        context: &AgentContext,
        workflow: &[crate::AgentType],
    ) -> AgentResult<Vec<AgentOutput>> {
        let mut outputs = Vec::new();
        
        for agent_type in workflow {
            if let Some(agent) = self.agents.iter().find(|a| a.agent_type() == *agent_type) {
                let output = agent.execute(task, context).await?;
                outputs.push(output);
            }
        }
        
        Ok(outputs)
    }
    
    /// Get available agents
    pub fn available_agents(&self) -> Vec<crate::AgentType> {
        self.agents.iter().map(|a| a.agent_type()).collect()
    }
    
    /// Find agent by type
    pub fn get_agent(&self, agent_type: crate::AgentType) -> Option<&dyn Agent> {
        self.agents.iter().find(|a| a.agent_type() == agent_type).map(|a| a.as_ref())
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_orchestrator_workflow() {
        let orchestrator = AgentOrchestrator::new();
        let context = AgentContext {
            project_dir: "/tmp/test".to_string(),
            skills: vec![],
            model: "test".to_string(),
            preferences: Default::default(),
        };
        
        let workflow = vec![
            crate::AgentType::Architect,
            crate::AgentType::Coder,
            crate::AgentType::Tester,
        ];
        
        let outputs = orchestrator.execute_workflow("Build a web app", &context, &workflow).await.unwrap();
        assert_eq!(outputs.len(), 3);
    }
}
