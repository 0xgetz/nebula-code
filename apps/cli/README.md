# Nebula Code CLI

Command-line interface for Nebula Code, providing powerful development tools.

## Installation

```bash
# Build from source
cargo build --release

# The binary will be available at target/release/nebula
```

## Usage

```bash
# Initialize a new project
nebula init --name my-project --project-type web

# Generate a development plan
nebula plan --path ./my-project --format markdown

# Build the project
nebula build --config release

# Review code
nebula review src/ --depth deep

# Deploy to production
nebula deploy --target production
```

## Commands

- `init` - Initialize a new Nebula Code project
- `plan` - Generate a development plan
- `build` - Build the project
- `review` - Review code for issues
- `deploy` - Deploy the application

## Development

```bash
# Run in development mode with watch
cargo watch -x run

# Run tests
cargo test
```
