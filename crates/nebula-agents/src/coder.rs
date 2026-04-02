use async_trait::async_trait;
use crate::agent::{Agent, AgentContext, AgentError, AgentOutput, AgentResult, AgentFile, FileAction};

/// Coder Agent - writes production-ready code following best practices
pub struct CoderAgent;

impl CoderAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for CoderAgent {
    fn agent_type(&self) -> crate::AgentType {
        crate::AgentType::Coder
    }
    
    fn description(&self) -> &'static str {
        "Writes production-ready code following best practices and design patterns"
    }
    
    async fn execute(&self, input: &str, _context: &AgentContext) -> AgentResult<AgentOutput> {
        // TODO: Implement actual LLM integration with skill cards
        let code = format!(
            "// Generated code for: {}\n\n            fn main() {{\n            \    println!(\"Hello from Nebula Code!\");\n            }}\n            \n            #[cfg(test)]\n            mod tests {{\n            \    use super::*;\n            \n            \    #[test]\n            \    fn test_main() {{\n            \        // TODO: Add tests\n            \    }}\n            }}",
            input
        );
        
        Ok(AgentOutput {
            content: code.clone(),
            files: vec![
                AgentFile {
                    path: "src/main.rs".to_string(),
                    content: code,
                    action: FileAction::Create,
                }
            ],
            reasoning: "Generated basic Rust program with test structure".to_string(),
            confidence: 0.8,
        })
    }
    
    fn can_handle(&self, _task: &str) -> bool {
        true // Coder can handle any coding task
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_coder_execution() {
        let agent = CoderAgent::new();
        let context = AgentContext {
            project_dir: "/tmp/test".to_string(),
            skills: vec![],
            model: "test".to_string(),
            preferences: Default::default(),
        };
        
        let result = agent.execute("Create a hello world app", &context).await.unwrap();
        assert!(result.content.contains("fn main()"));
        assert_eq!(result.files.len(), 1);
    }
}
