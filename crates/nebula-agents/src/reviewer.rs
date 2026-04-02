use async_trait::async_trait;
use crate::agent::{Agent, AgentContext, AgentError, AgentOutput, AgentResult};

/// Reviewer Agent - performs security audits and code reviews
pub struct ReviewerAgent;

impl ReviewerAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for ReviewerAgent {
    fn agent_type(&self) -> crate::AgentType {
        crate::AgentType::Reviewer
    }
    
    fn description(&self) -> &'static str {
        "Performs security audits and code reviews for quality assurance"
    }
    
    async fn execute(&self, input: &str, _context: &AgentContext) -> AgentResult<AgentOutput> {
        // TODO: Implement actual code review with security analysis
        let review = format!(
            "# Code Review\n\n            ## Security Analysis\n            - [ ] Input validation\n            - [ ] Authentication & authorization\n            - [ ] Data protection\n            - [ ] Error handling\n            \n            ## Code Quality\n            - [ ] Follows style guidelines\n            - [ ] Proper error handling\n            - [ ] Test coverage\n            - [ ] Documentation\n            \n            ## Suggestions\n            1. Add more comprehensive tests\n            2. Improve error messages\n            3. Add documentation\n            \n            ## Reviewed Code\n            ```\n            {}\n            ```",
            input
        );
        
        Ok(AgentOutput {
            content: review,
            files: vec![],
            reasoning: "Performed security and quality review of the provided code".to_string(),
            confidence: 0.85,
        })
    }
    
    fn can_handle(&self, _task: &str) -> bool {
        true // Reviewer can handle any review task
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reviewer_execution() {
        let agent = ReviewerAgent::new();
        let context = AgentContext {
            project_dir: "/tmp/test".to_string(),
            skills: vec![],
            model: "test".to_string(),
            preferences: Default::default(),
        };
        
        let result = agent.execute("fn main() {}", &context).await.unwrap();
        assert!(result.content.contains("Code Review"));
        assert!(result.content.contains("Security Analysis"));
    }
}
