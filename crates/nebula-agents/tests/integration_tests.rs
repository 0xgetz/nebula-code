//! Integration tests for multi-agent collaboration scenarios.
//!
//! These tests verify that multiple agents can work together on complex task pipelines,
//! handle failures gracefully, and respect priority scheduling.

use nebula_agents::orchestration::{Orchestrator, Priority, Task, TaskId, TaskStatus};
use nebula_agents::types::{Agent, AgentCapability, AgentId, AgentMetadata, AgentState};
use std::thread;
use std::time::Duration;

// ===== Helper Functions =====

fn create_agent(name: &str, capabilities: &[&str]) -> Agent {
    let id = AgentId::new();
    let mut agent = Agent::new(id.clone(), AgentMetadata::new().with_name(name));
    agent.set_state(AgentState::Idle);
    for cap in capabilities {
        agent.add_capability(AgentCapability::new(*cap, "Test capability", "1.0"));
    }
    agent
}

fn create_task(description: &str, priority: Priority, dependencies: Vec<TaskId>) -> Task {
    Task::new(description).with_priority(priority).with_dependencies(dependencies)
}

// ===== Scenario 1: Multiple Agents Collaborating on a Task Pipeline =====

#[test]
fn test_multi_agent_collaboration_pipeline() {
    // Create an orchestrator with three specialized agents
    let orchestrator = Orchestrator::new("pipeline-orchestrator");

    // Register agents with different capabilities
    let planner = create_agent("Planner", &["planning", "architecture"]);
    let coder = create_agent("Coder", &["coding", "rust", "python"]);
    let tester = create_agent("Tester", &["testing", "unit-tests", "integration-tests"]);

    let planner_id = planner.id.clone();
    let coder_id = coder.id.clone();
    let tester_id = tester.id.clone();

    orchestrator.register_agent(planner).unwrap();
    orchestrator.register_agent(coder).unwrap();
    orchestrator.register_agent(tester).unwrap();

    // Create a pipeline: Plan -> Code -> Test
    let plan_task = create_task("Design system architecture", Priority::High, vec![]);
    let plan_task_id = orchestrator.schedule_task(plan_task).unwrap();

    let code_task = create_task(
        "Implement core modules",
        Priority::High,
        vec![plan_task_id.clone()],
    );
    let code_task_id = orchestrator.schedule_task(code_task).unwrap();

    let test_task = create_task(
        "Run comprehensive test suite",
        Priority::Normal,
        vec![code_task_id.clone()],
    );
    let test_task_id = orchestrator.schedule_task(test_task).unwrap();

    // Execute the pipeline step by step

    // Step 1: Planner works on architecture
    let next_task = orchestrator.get_next_ready_task().unwrap().unwrap();
    assert_eq!(next_task.id, plan_task_id);

    orchestrator
        .scheduler()
        .assign_task(&plan_task_id, &planner_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&plan_task_id, Some("Architecture designed".to_string()))
        .unwrap();

    // Step 2: Coder can now start (dependency satisfied)
    let next_task = orchestrator.get_next_ready_task().unwrap().unwrap();
    assert_eq!(next_task.id, code_task_id);

    orchestrator
        .scheduler()
        .assign_task(&code_task_id, &coder_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&code_task_id, Some("Code implemented".to_string()))
        .unwrap();

    // Step 3: Tester can now start
    let next_task = orchestrator.get_next_ready_task().unwrap().unwrap();
    assert_eq!(next_task.id, test_task_id);

    orchestrator
        .scheduler()
        .assign_task(&test_task_id, &tester_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&test_task_id, Some("All tests passed".to_string()))
        .unwrap();

    // Verify final state
    assert_eq!(
        orchestrator.scheduler().get_task_status(&plan_task_id).unwrap(),
        TaskStatus::Completed
    );
    assert_eq!(
        orchestrator.scheduler().get_task_status(&code_task_id).unwrap(),
        TaskStatus::Completed
    );
    assert_eq!(
        orchestrator.scheduler().get_task_status(&test_task_id).unwrap(),
        TaskStatus::Completed
    );
}

// ===== Scenario 2: Task Delegation with Complex Dependencies =====

#[test]
fn test_task_delegation_with_dependencies() {
    let orchestrator = Orchestrator::new("dependency-orchestrator");

    // Register multiple agents
    let agent_a = create_agent("AgentA", &["task-a"]);
    let agent_b = create_agent("AgentB", &["task-b"]);
    let agent_c = create_agent("AgentC", &["task-c"]);
    let agent_d = create_agent("AgentD", &["task-d"]);

    let agent_a_id = agent_a.id.clone();
    let agent_b_id = agent_b.id.clone();
    let agent_c_id = agent_c.id.clone();
    let agent_d_id = agent_d.id.clone();

    orchestrator.register_agent(agent_a).unwrap();
    orchestrator.register_agent(agent_b).unwrap();
    orchestrator.register_agent(agent_c).unwrap();
    orchestrator.register_agent(agent_d).unwrap();

    // Create a diamond dependency pattern:
    //     T1
    //    /  \
    //   T2  T3
    //    \  /
    //     T4

    let task1 = create_task("Initial setup", Priority::Critical, vec![]);
    let task1_id = orchestrator.schedule_task(task1).unwrap();

    let task2 = create_task("Parallel work A", Priority::Normal, vec![task1_id.clone()]);
    let task2_id = orchestrator.schedule_task(task2).unwrap();

    let task3 = create_task("Parallel work B", Priority::Normal, vec![task1_id.clone()]);
    let task3_id = orchestrator.schedule_task(task3).unwrap();

    let task4 = create_task(
        "Final integration",
        Priority::High,
        vec![task2_id.clone(), task3_id.clone()],
    );
    let task4_id = orchestrator.schedule_task(task4).unwrap();

    // Execute T1
    orchestrator
        .scheduler()
        .assign_task(&task1_id, &agent_a_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&task1_id, Some("Setup complete".to_string()))
        .unwrap();

    // Execute T2 and T3 in parallel (conceptually - we simulate by doing them sequentially)
    orchestrator
        .scheduler()
        .assign_task(&task2_id, &agent_b_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&task2_id, Some("Work A done".to_string()))
        .unwrap();

    orchestrator
        .scheduler()
        .assign_task(&task3_id, &agent_c_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&task3_id, Some("Work B done".to_string()))
        .unwrap();

    // Now T4 should be ready (both dependencies satisfied)
    let next_task = orchestrator.get_next_ready_task().unwrap().unwrap();
    assert_eq!(next_task.id, task4_id);

    orchestrator
        .scheduler()
        .assign_task(&task4_id, &agent_d_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&task4_id, Some("Integration complete".to_string()))
        .unwrap();

    // Verify all tasks completed
    let all_tasks = [task1_id, task2_id, task3_id, task4_id];
    for task_id in &all_tasks {
        assert_eq!(
            orchestrator.scheduler().get_task_status(task_id).unwrap(),
            TaskStatus::Completed
        );
    }
}

// ===== Scenario 3: Agent Failure and Recovery =====

#[test]
fn test_agent_failure_and_recovery() {
    let orchestrator = Orchestrator::new("recovery-orchestrator");

    let agent1 = create_agent("PrimaryAgent", &["primary-task"]);
    let agent2 = create_agent("BackupAgent", &["primary-task"]);

    let agent1_id = agent1.id.clone();
    let agent2_id = agent2.id.clone();

    orchestrator.register_agent(agent1).unwrap();
    orchestrator.register_agent(agent2).unwrap();

    // Create a critical task
    let task = create_task("Critical operation", Priority::Critical, vec![]);
    let task_id = orchestrator.schedule_task(task).unwrap();

    // Assign to primary agent
    orchestrator
        .scheduler()
        .assign_task(&task_id, &agent1_id)
        .unwrap();

    // Simulate agent failure
    orchestrator
        .scheduler()
        .fail_task(&task_id, "Primary agent crashed".to_string())
        .unwrap();

    assert_eq!(
        orchestrator.scheduler().get_task_status(&task_id).unwrap(),
        TaskStatus::Failed
    );

    // Recovery: Create a new task to retry (in a real system, we might have retry logic)
    let retry_task = create_task("Retry critical operation", Priority::Critical, vec![]);
    let retry_task_id = orchestrator.schedule_task(retry_task).unwrap();

    // Assign to backup agent
    orchestrator
        .scheduler()
        .assign_task(&retry_task_id, &agent2_id)
        .unwrap();
    orchestrator
        .scheduler()
        .complete_task(&retry_task_id, Some("Recovered successfully".to_string()))
        .unwrap();

    assert_eq!(
        orchestrator
            .scheduler()
            .get_task_status(&retry_task_id)
            .unwrap(),
        TaskStatus::Completed
    );
}

#[test]
fn test_agent_failure_updates_state() {
    let orchestrator = Orchestrator::new("failure-state-orchestrator");

    let agent = create_agent("Worker", &["work"]);
    let agent_id = agent.id.clone();
    orchestrator.register_agent(agent).unwrap();

    let task = create_task("Important work", Priority::High, vec![]);
    let task_id = orchestrator.schedule_task(task).unwrap();

    // Assign and then fail
    orchestrator
        .scheduler()
        .assign_task(&task_id, &agent_id)
        .unwrap();
    orchestrator
        .scheduler()
        .fail_task(&task_id, "Unexpected error".to_string())
        .unwrap();

    let task = orchestrator.scheduler().get_task(&task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.result, Some("Unexpected error".to_string()));
    assert_eq!(task.assigned_agent, Some(agent_id));
}

// ===== Scenario 4: Priority-Based Task Scheduling =====

#[test]
fn test_priority_based_scheduling() {
    let orchestrator = Orchestrator::new("priority-orchestrator");

    // Schedule tasks with different priorities in random order
    let low_task = create_task("Low priority cleanup", Priority::Low, vec![]);
    let _low_id = orchestrator.schedule_task(low_task).unwrap();

    let critical_task = create_task("Critical security patch", Priority::Critical, vec![]);
    let _critical_id = orchestrator.schedule_task(critical_task).unwrap();

    let normal_task = create_task("Regular maintenance", Priority::Normal, vec![]);
    let _normal_id = orchestrator.schedule_task(normal_task).unwrap();

    let high_task = create_task("Performance optimization", Priority::High, vec![]);
    let _high_id = orchestrator.schedule_task(high_task).unwrap();

    // Get pending tasks - should be sorted by priority (highest first)
    let pending = orchestrator.scheduler().get_pending_tasks().unwrap();
    assert_eq!(pending.len(), 4);
    assert_eq!(pending[0].priority, Priority::Critical);
    assert_eq!(pending[1].priority, Priority::High);
    assert_eq!(pending[2].priority, Priority::Normal);
    assert_eq!(pending[3].priority, Priority::Low);

    // Verify get_next_ready_task returns highest priority first
    let next = orchestrator.get_next_ready_task().unwrap().unwrap();
    assert_eq!(next.priority, Priority::Critical);
}

#[test]
fn test_priority_override_with_same_priority() {
    let scheduler = nebula_agents::orchestration::TaskScheduler::new();

    // Create two tasks with same priority but different creation times
    let task1 = Task::new("First task").with_priority(Priority::Normal);
    let task1_id = scheduler.schedule_task(task1).unwrap();

    // Small delay to ensure different timestamps
    thread::sleep(Duration::from_millis(10));

    let task2 = Task::new("Second task").with_priority(Priority::Normal);
    let task2_id = scheduler.schedule_task(task2).unwrap();

    let pending = scheduler.get_pending_tasks().unwrap();
    assert_eq!(pending.len(), 2);
    // Should be sorted by creation time (oldest first) when priority is equal
    assert_eq!(pending[0].id, task1_id);
    assert_eq!(pending[1].id, task2_id);
}

// ===== Scenario 5: Concurrent Task Execution Simulation =====

#[test]
fn test_concurrent_task_execution() {
    let orchestrator = Orchestrator::new("concurrent-orchestrator");

    // Register multiple agents that can work in parallel
    let agents: Vec<Agent> = (0..3)
        .map(|i| create_agent(&format!("Worker{}", i), &["parallel-work"]))
        .collect();

    let agent_ids: Vec<AgentId> = agents.iter().map(|a| a.id.clone()).collect();

    for agent in agents {
        orchestrator.register_agent(agent).unwrap();
    }

    // Create independent tasks that can run in parallel
    let mut task_ids = Vec::new();
    for i in 0..5 {
        let task = create_task(&format!("Parallel task {}", i), Priority::Normal, vec![]);
        let task_id = orchestrator.schedule_task(task).unwrap();
        task_ids.push(task_id.clone());
    }

    // Simulate concurrent execution by assigning tasks to available agents
    let mut assigned_count = 0;
    for agent_id in &agent_ids {
        if let Ok(Some(task)) = orchestrator.get_next_ready_task() {
            orchestrator
                .scheduler()
                .assign_task(&task.id, agent_id)
                .unwrap();
            assigned_count += 1;
        }
    }

    assert_eq!(assigned_count, 3); // Should assign to all 3 agents

    // Complete all running tasks
    for agent_id in &agent_ids {
        let tasks = orchestrator
            .scheduler()
            .get_tasks_by_agent(agent_id)
            .unwrap();
        for task in tasks {
            orchestrator
                .scheduler()
                .complete_task(&task.id, Some("Done".to_string()))
                .unwrap();
        }
    }

    // Now remaining tasks should be ready
    let remaining_pending = orchestrator.scheduler().get_pending_tasks().unwrap();
    assert_eq!(remaining_pending.len(), 2);

    // Complete remaining tasks
    for task in remaining_pending {
        if let Ok(agent_id) = orchestrator.find_available_agent("parallel-work") {
            orchestrator
                .scheduler()
                .assign_task(&task.id, &agent_id)
                .unwrap();
            orchestrator
                .scheduler()
                .complete_task(&task.id, Some("Done".to_string()))
                .unwrap();
        }
    }

    // Verify all tasks completed
    let all_tasks = orchestrator.scheduler().task_count().unwrap();
    assert_eq!(all_tasks, 5);

    let completed_tasks = orchestrator
        .scheduler()
        .get_tasks_by_status(TaskStatus::Completed)
        .unwrap();
    assert_eq!(completed_tasks.len(), 5);
}

// ===== Scenario 6: Task Cancellation =====

#[test]
fn test_task_cancellation() {
    let orchestrator = Orchestrator::new("cancellation-orchestrator");

    let agent = create_agent("Worker", &["work"]);
    orchestrator.register_agent(agent).unwrap();

    let task1 = create_task("Cancellable task", Priority::Normal, vec![]);
    let task1_id = orchestrator.schedule_task(task1).unwrap();

    let task2 = create_task("Depends on cancelled", Priority::Normal, vec![task1_id.clone()]);
    let task2_id = orchestrator.schedule_task(task2).unwrap();

    // Cancel the first task
    orchestrator.scheduler().cancel_task(&task1_id).unwrap();
    assert_eq!(
        orchestrator.scheduler().get_task_status(&task1_id).unwrap(),
        TaskStatus::Cancelled
    );

    // The second task should never become ready because its dependency is cancelled
    let next_ready = orchestrator.get_next_ready_task().unwrap();
    assert!(next_ready.is_none());

    // Trying to cancel an already cancelled task should fail
    assert!(orchestrator.scheduler().cancel_task(&task1_id).is_err());
}

// ===== Scenario 7: Agent Capability Matching =====

#[test]
fn test_agent_capability_matching() {
    let orchestrator = Orchestrator::new("capability-orchestrator");

    let rust_dev = create_agent("RustDev", &["rust", "systems-programming"]);
    let python_dev = create_agent("PythonDev", &["python", "scripting"]);
    let full_stack = create_agent("FullStack", &["rust", "python", "web"]);

    orchestrator.register_agent(rust_dev).unwrap();
    orchestrator.register_agent(python_dev).unwrap();
    orchestrator.register_agent(full_stack).unwrap();

    // Find agent with specific capability
    let rust_agents = orchestrator.find_available_agent("rust").unwrap();
    assert!(rust_agents.as_uuid() != &uuid::Uuid::nil()); // Should find at least one

    let python_agents = orchestrator.find_available_agent("python").unwrap();
    assert!(python_agents.as_uuid() != &uuid::Uuid::nil());

    // Find agent with rare capability
    let web_agents = orchestrator.find_available_agent("web").unwrap();
    assert!(web_agents.as_uuid() != &uuid::Uuid::nil());

    // Non-existent capability should fail
    assert!(orchestrator.find_available_agent("nonexistent").is_err());
}

// ===== Scenario 8: Large-Scale Task Pipeline =====

#[test]
fn test_large_scale_pipeline() {
    let orchestrator = Orchestrator::new("large-pipeline-orchestrator");

    // Register 5 agents with different capabilities
    for i in 0..5 {
        let agent = create_agent(&format!("Agent{}", i), &[&format!("skill{}", i)]);
        orchestrator.register_agent(agent).unwrap();
    }

    // Create a chain of 10 dependent tasks
    let mut task_ids = Vec::new();
    let mut prev_id = None;

    for i in 0..10 {
        let deps = prev_id.map(|id| vec![id]).unwrap_or_default();
        let task = create_task(&format!("Stage {}", i), Priority::Normal, deps);
        let task_id = orchestrator.schedule_task(task).unwrap();
        task_ids.push(task_id.clone());
        prev_id = Some(task_id);
    }

    // Execute the pipeline sequentially
    for task_id in &task_ids {
        let next = orchestrator.get_next_ready_task().unwrap();
        assert!(next.is_some());
        let task = next.unwrap();
        assert_eq!(task.id, *task_id);

        // Find an available agent (any agent will do for this test)
        let agents = orchestrator.list_agents().unwrap();
        let available_agent = agents.iter().find(|a| a.state == AgentState::Idle).unwrap();

        orchestrator
            .scheduler()
            .assign_task(&task_id, &available_agent.id)
            .unwrap();
        orchestrator
            .scheduler()
            .complete_task(&task_id, Some(format!("Stage {} complete", task_id)))
            .unwrap();
    }

    // Verify all tasks completed
    for task_id in &task_ids {
        assert_eq!(
            orchestrator.scheduler().get_task_status(task_id).unwrap(),
            TaskStatus::Completed
        );
    }
}
