# Nebula Utils

Utility functions and helpers for Nebula Code.

## Modules

- `file` - File system operations (read, write, directory management)
- `validate` - Input validation for names, versions, and paths
- `format` - Human-readable formatting for sizes and durations

## Usage

```rust
use nebula_utils::{read_file, write_file, validate_name, format_size};

// File operations
write_file("output.txt", "Hello, World!")?;
let content = read_file("output.txt")?;

// Validation
validate_name("my-project")?; // OK
validate_name("invalid name!")?; // Error

// Formatting
let size = format_size(1048576); // "1.0 MB"
let duration = format_duration(65000); // "1m 5s"
```
