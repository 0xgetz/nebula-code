# Nebula Agents

**Multi-Agent Collaboration System for Nebula Code**

Nebula Agents provides the core infrastructure for multi-agent collaboration in Nebula Code. It implements a robust system for agent lifecycle management, inter-agent communication, task orchestration, and distributed task execution.

## Features

### Core Agent System
- **Agent Lifecycle Management**: Create, register, and manage agents with distinct capabilities
- **Type-Safe Communication**: Strongly-typed message protocol for reliable inter-agent communication
- **Agent Registry**: Dynamic service discovery and capability tracking
- **Pub/Sub Messaging**: Asynchronous message channels for decoupled agent communication

### Orchestration Engine
- **Task Scheduling**: Priority-based task queue with support for dependencies
- **Dependency Resolution**: Automatic task graph management and execution ordering
- **Load Balancing**: Intelligent agent assignment based on capabilities and availability
- **Fault Tolerance**: Agent failure detection, task recovery, and retry mechanisms

### Task Management
- **Priority Levels**: Critical, High, Normal, Low priority scheduling
- **Task Lifecycle**: Complete state tracking (Pending → Running → Completed/Failed/Cancelled)
- **Dependency Graphs**: Support for complex task dependencies and parallel execution
- **Real-time Monitoring**: Live task status updates and progress tracking

## Architecture

```
┌─────────────────┐
│   Orchestrator  │
│                 │
│ ┌─────────────┐ │
│ │TaskScheduler│ │
│ └─────────────┘ │
│ ┌─────────────┐ │
│ │AgentRegistry│◄──┐
│ └─────────────┘  │
└─────────────────┘ │
        │           │
        ▼           │
┌─────────────────┐ │
│ Communication   │ │
│ Protocol        │ │
│                 │ │
│ ┌─────────────┐ │ │
│ │MessageQueue │─┼──┘
│ └─────────────┘ │
│ ┌─────────────┐ │
│ │   Channels  │ │
│ └─────────────┘ │
└─────────────────┘
        ▲
        │
┌───────┴─────────┐
│                 │
▼                 ▼
Agent 1         Agent N
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
nebula-agents = { path = "../nebula-agents" }
```

### Basic Usage

```rust
use nebula_agents::{
    Agent, AgentId, AgentCapability, AgentMetadata,
    Orchestrator, Task, Priority, TaskStatus,
    CommunicationProtocol, Message, MessageType
};

// Create agents with specific capabilities
let architect = Agent::new(
    AgentId::new(),
    "architect",
    AgentMetadata::builder()
        .description("System architect agent")
        .version("1.0.0")
        .build(),
    vec![AgentCapability::ArchitectureDesign, AgentCapability::Planning],
);

let coder = Agent::new(
    AgentId::new(),
    "coder",
    AgentMetadata::builder()
        .description("Code generation agent")
        .version("1.0.0")
        .build(),
    vec![AgentCapability::CodeGeneration, AgentCapability::Refactoring],
);

// Set up orchestrator
let mut orchestrator = Orchestrator::new();
orchestrator.register_agent(architect);
orchestrator.register_agent(coder);

// Create and schedule tasks
let design_task = Task::builder()
    .description("Design system architecture")
    .priority(Priority::High)
    .assigned_agent(architect.id().clone())
    .build();

let implementation_task = Task::builder()
    .description("Implement based on design")
    .priority(Priority::Normal)
    .assigned_agent(coder.id().clone())
    .depends_on(design_task.id().clone())
    .build();

orchestrator.schedule_task(design_task);
orchestrator.schedule_task(implementation_task);

// Execute tasks
orchestrator.run().await?;
```

### Advanced Example: Multi-Agent Pipeline

```rust
// Create a pipeline of specialized agents
let mut orchestrator = Orchestrator::new();

// Register agents
orchestrator.register_agent(create_architect());
orchestrator.register_agent(create_coder());
orchestrator.register_agent(create_tester());
orchestrator.register_agent(create_reviewer());
orchestrator.register_agent(create_deployer());

// Define task pipeline with dependencies
let tasks = vec![
    Task::builder()
        .description("Analyze requirements")
        .priority(Priority::Critical)
        .agent("architect")
        .build(),
    Task::builder()
        .description("Design architecture")
        .priority(Priority::High)
        .agent("architect")
        .depends_on("analyze-requirements")
        .build(),
    Task::builder()
        .description("Implement features")
        .priority(Priority::Normal)
        .agent("coder")
        .depends_on("design-architecture")
        .build(),
    Task::builder()
        .description("Write tests")
        .priority(Priority::Normal)
        .agent("tester")
        .depends_on("implement-features")
        .build(),
    Task::builder()
        .description("Code review")
        .priority(Priority::High)
        .agent("reviewer")
        .depends_on("write-tests")
        .build(),
    Task::builder()
        .description("Deploy to production")
        .priority(Priority::Critical)
        .agent("deployer")
        .depends_on("code-review")
        .build(),
];

// Execute pipeline
for task in tasks {
    orchestrator.schedule_task(task);
}

orchestrator.run().await?;
```

## Core Components

### Agent Types

- **`Agent`**: Represents an autonomous agent with capabilities and metadata
- **`AgentId`**: Unique identifier for each agent
- **`AgentState`**: Current state (Idle, Busy, Offline, Error)
- **`AgentCapability`**: Skills and abilities an agent possesses
- **`AgentMetadata`**: Descriptive information about the agent

### Communication Protocol

- **`Message`**: Typed message structure for inter-agent communication
- **`MessageType`**: Categories of messages (Task, Query, Response, Event)
- **`CommunicationProtocol`**: Trait for implementing custom protocols
- **`Channel`**: Bidirectional communication channel
- **`MessageQueue`**: Asynchronous message buffering

### Orchestration

- **`Orchestrator`**: Central coordinator for agents and tasks
- **`Task`**: Unit of work with dependencies and priority
- **`TaskScheduler`**: Manages task queue and execution order
- **`Priority`**: Task priority levels (Critical, High, Normal, Low)
- **`TaskStatus`**: Task lifecycle states

### Registry

- **`AgentRegistry`**: Service registry for agent discovery
- **`CapabilityRegistry`**: Index of capabilities to agents
- **`ServiceDiscovery`**: Dynamic agent lookup and routing

## API Reference

### Agent Management

```rust
// Create an agent
let agent = Agent::new(
    AgentId::new(),
    "my-agent",
    AgentMetadata::builder()
        .description("My custom agent")
        .version("1.0.0")
        .tags(vec!["custom", "specialized"])
        .build(),
    vec![AgentCapability::Custom],
);

// Register with orchestrator
orchestrator.register_agent(agent);

// Query agents by capability
let agents = orchestrator.get_agents_by_capability(AgentCapability::CodeGeneration);
```

### Task Scheduling

```rust
// Create a task with dependencies
let task = Task::builder()
    .id(TaskId::new())
    .description("Complex task")
    .priority(Priority::High)
    .assigned_agent(agent_id)
    .depends_on(vec![task_id_1, task_id_2])
    .metadata(TaskMetadata::builder()
        .estimated_duration(Duration::from_secs(300))
        .build())
    .build();

// Schedule task
orchestrator.schedule_task(task);

// Query task status
let status = orchestrator.get_task_status(&task_id);
```

### Communication

```rust
// Send message to agent
let message = Message::builder()
    .from(agent_id_1)
    .to(agent_id_2)
    .message_type(MessageType::Query)
    .content("What is the status?")
    .build();

protocol.send(message).await?;

// Subscribe to channel
let mut receiver = protocol.subscribe("task-updates");
while let Some(msg) = receiver.recv().await {
    println!("Received: {:?}", msg);
}
```

## Testing

The crate includes comprehensive test coverage:

```bash
# Run all tests
cargo test -p nebula-agents

# Run integration tests
cargo test --test integration_tests -p nebula-agents

# Run with coverage
cargo tarpaulin -p nebula-agents
```

### Example Tests

- **Multi-agent collaboration**: Agents working together on a task pipeline
- **Dependency resolution**: Complex task dependencies and ordering
- **Priority scheduling**: Priority-based task execution
- **Fault tolerance**: Agent failure and recovery scenarios
- **Message routing**: Inter-agent communication patterns

## Performance

- **Task Scheduling**: O(log n) for priority queue operations
- **Agent Lookup**: O(1) average case for capability-based routing
- **Message Delivery**: Asynchronous, non-blocking communication
- **Memory**: Minimal overhead, efficient data structures

## Error Handling

The crate uses `thiserror` for structured error types:

```rust
use nebula_agents::AgentError;

match orchestrator.schedule_task(task) {
    Ok(_) => println!("Task scheduled"),
    Err(AgentError::AgentNotFound(id)) => eprintln!("Agent {} not found", id),
    Err(AgentError::CircularDependency) => eprintln!("Circular dependency detected"),
    Err(AgentError::SchedulerFull) => eprintln!("Task queue is full"),
}
```

## Contributing

Contributions are welcome! Please see the main [Contributing Guide](../../CONTRIBUTING.md) for details.

### Development

```bash
# Build the crate
cargo build -p nebula-agents

# Run tests
cargo test -p nebula-agents

# Check code formatting
cargo fmt -- --check

# Run clippy
cargo clippy -p nebula-agents
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

Part of the Nebula Code project - AI Coding Agent with Federated Learning and Multi-Agent Orchestration