//! Multi-Agent Collaboration Example
//!
//! This example demonstrates how to set up a multi-agent system with the Nebula framework.
//! It shows how to create multiple specialized agents, set up an orchestrator, schedule tasks
//! with dependencies, and run the collaboration loop to process tasks.
//!
//! The scenario: A software development pipeline with three agents:
//! - Architect: Designs system architecture
//! - Developer: Implements the code
//! - Reviewer: Reviews and approves the code
//!
//! The tasks have dependencies: Architect -> Developer -> Reviewer

use nebula_agents::orchestration::{Orchestrator, Priority, Task, TaskStatus};
use nebula_agents::types::{Agent, AgentCapability, AgentId, AgentMetadata, AgentState};
use std::time::Duration;

/// Creates an agent with the given name and capabilities.
fn create_agent(name: &str, capabilities: &[&str]) -> Agent {
    let id = AgentId::new();
    let mut agent = Agent::new(id.clone(), AgentMetadata::new().with_name(name));
    agent.set_state(AgentState::Idle);
    for cap in capabilities {
        agent.add_capability(AgentCapability::new(*cap, "Agent capability", "1.0"));
    }
    agent
}

/// Simulates an agent executing a task.
/// In a real system, this would involve actual work.
fn simulate_agent_work(agent_name: &str, task: &Task) -> Result<String, String> {
    println!(
        "  [{}] Starting task: {} (priority: {:?})",
        agent_name, task.description, task.priority
    );

    // Simulate work with a small delay
    std::thread::sleep(Duration::from_millis(100));

    println!(
        "  [{}] Completed task: {}",
        agent_name, task.description
    );

    Ok(format!("{} completed by {}", task.description, agent_name))
}

/// Runs the collaboration loop until all tasks are processed.
fn run_collaboration_loop(orchestrator: &Orchestrator) -> Result<(), Box<dyn std::error::Error>> {
    let mut iteration = 0;
    let max_iterations = 100; // Safety limit

    while iteration < max_iterations {
        iteration += 1;

        // Get the next ready task (dependencies satisfied, highest priority)
        match orchestrator.get_next_ready_task()? {
            Some(task) => {
                println!(
                    "Iteration {}: Processing task '{}' (id: {})",
                    iteration, task.description, task.id
                );

                // Find an available agent with the required capability
                // For this example, we'll just pick any idle agent
                let agents = orchestrator.list_agents()?;
                if let Some(agent) = agents.iter().find(|a| a.state == AgentState::Idle) {
                    // Assign the task to the agent
                    orchestrator
                        .scheduler()
                        .assign_task(&task.id, &agent.id)?;

                    // Simulate the agent doing the work
                    match simulate_agent_work(&agent.metadata.name.as_deref().unwrap_or("Unknown"), &task) {
                        Ok(result) => {
                            orchestrator
                                .scheduler()
                                .complete_task(&task.id, Some(result))?;
                        }
                        Err(error) => {
                            orchestrator
                                .scheduler()
                                .fail_task(&task.id, error)?;
                        }
                    }
                } else {
                    println!("  No available agents at this time");
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            None => {
                // No more ready tasks - check if we're done
                let pending = orchestrator.scheduler().get_pending_tasks()?;
                let running = orchestrator
                    .scheduler()
                    .get_tasks_by_status(TaskStatus::Running)?;

                if pending.is_empty() && running.is_empty() {
                    println!("All tasks completed!");
                    break;
                } else {
                    println!(
                        "  Waiting for dependencies: {} pending, {} running",
                        pending.len(),
                        running.len()
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    Ok(())
}

/// Prints a summary of all tasks and their final status.
fn print_task_summary(orchestrator: &Orchestrator) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Task Summary ===");

    let all_tasks_status = [
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
        TaskStatus::Running,
        TaskStatus::Pending,
    ];

    for status in all_tasks_status {
        let tasks = orchestrator.scheduler().get_tasks_by_status(status)?;
        if !tasks.is_empty() {
            println!("{}: {} task(s)", format!("{:?}", status), tasks.len());
            for task in tasks {
                let agent_str = match &task.assigned_agent {
                    Some(_) => " (assigned)",
                    None => " (unassigned)",
                };
                println!("  - {}{}", task.description, agent_str);
                if let Some(result) = &task.result {
                    println!("    Result: {}", result);
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Agent Collaboration Example ===\n");

    // Step 1: Create the orchestrator
    println!("Step 1: Setting up orchestrator...");
    let orchestrator = Orchestrator::new("development-pipeline");

    // Step 2: Create and register specialized agents
    println!("Step 2: Creating and registering agents...");
    let architect = create_agent("Architect", &["architecture", "design"]);
    let developer = create_agent("Developer", &["coding", "rust", "python"]);
    let reviewer = create_agent("Reviewer", &["code-review", "quality-assurance"]);

    let architect_id = architect.id.clone();
    let developer_id = developer.id.clone();
    let reviewer_id = reviewer.id.clone();

    orchestrator.register_agent(architect)?;
    orchestrator.register_agent(developer)?;
    orchestrator.register_agent(reviewer)?;

    println!(
        "  Registered agents: Architect ({}), Developer ({}), Reviewer ({})",
        architect_id, developer_id, reviewer_id
    );

    // Step 3: Create tasks with dependencies
    println!("\nStep 3: Creating tasks with dependencies...");

    // Task 1: Design the system architecture (no dependencies)
    let design_task = Task::new("Design system architecture")
        .with_priority(Priority::Critical);
    let design_task_id = orchestrator.schedule_task(design_task)?;
    println!("  Scheduled: Design system architecture (id: {})", design_task_id);

    // Task 2: Implement core modules (depends on design)
    let implement_task = Task::new("Implement core modules")
        .with_priority(Priority::High)
        .with_dependencies(vec![design_task_id.clone()]);
    let implement_task_id = orchestrator.schedule_task(implement_task)?;
    println!(
        "  Scheduled: Implement core modules (id: {}, depends on: {})",
        implement_task_id, design_task_id
    );

    // Task 3: Write unit tests (depends on implementation)
    let test_task = Task::new("Write unit tests")
        .with_priority(Priority::Normal)
        .with_dependencies(vec![implement_task_id.clone()]);
    let test_task_id = orchestrator.schedule_task(test_task)?;
    println!(
        "  Scheduled: Write unit tests (id: {}, depends on: {})",
        test_task_id, implement_task_id
    );

    // Task 4: Code review (depends on implementation)
    let review_task = Task::new("Code review")
        .with_priority(Priority::High)
        .with_dependencies(vec![implement_task_id.clone()]);
    let review_task_id = orchestrator.schedule_task(review_task)?;
    println!(
        "  Scheduled: Code review (id: {}, depends on: {})",
        review_task_id, implement_task_id
    );

    // Task 5: Deploy to staging (depends on tests and review)
    let deploy_task = Task::new("Deploy to staging")
        .with_priority(Priority::Normal)
        .with_dependencies(vec![test_task_id.clone(), review_task_id.clone()]);
    let deploy_task_id = orchestrator.schedule_task(deploy_task)?;
    println!(
        "  Scheduled: Deploy to staging (id: {}, depends on: {}, {})",
        deploy_task_id, test_task_id, review_task_id
    );

    // Step 4: Run the collaboration loop
    println!("\nStep 4: Running collaboration loop...\n");
    run_collaboration_loop(&orchestrator)?;

    // Step 5: Print summary
    print_task_summary(&orchestrator)?;

    // Step 6: Show final agent states
    println!("\n=== Final Agent States ===");
    let agents = orchestrator.list_agents()?;
    for agent in agents {
        println!(
            "  {} ({}): {:?}",
            agent.metadata.name.as_deref().unwrap_or("Unknown"), agent.id, agent.state
        );
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
