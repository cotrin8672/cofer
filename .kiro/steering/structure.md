# Project Structure

## Root Directory Organization
```
cofer/
├── .kiro/                   # Kiro spec-driven development
│   ├── steering/           # Project steering documents (this directory)
│   └── specs/              # Feature specifications
├── .serena/                # Serena AI assistant memories
│   └── memories/           # Project knowledge base
├── .claude/                # Claude Code configuration
│   └── commands/           # Custom slash commands
│       └── kiro/          # Kiro-specific commands
├── .cofer/                 # Cofer runtime data (gitignored)
│   └── worktrees/         # Git worktrees for environments
├── src/                    # Rust source code
│   └── main.rs            # Entry point (CLI with worktree management)
├── target/                 # Build artifacts (gitignored)
├── Cargo.toml             # Rust project manifest
├── Cargo.lock             # Dependency lock file
├── CLAUDE.md              # Claude Code instructions
├── AGENTS.md              # Coding agent project memory
├── about.md               # Project overview
└── .gitignore             # Git ignore patterns
```

## Subdirectory Structures

### Source Code (`src/`) - Current State
```
src/
└── main.rs                 # CLI entry point with worktree creation
```

**Current Implementation:**
- CLI parsing with clap subcommands
- Git worktree creation for isolated environments
- Branch management for cofer environments
- Async runtime setup with tokio

### Planned Module Structure (`src/`) - Future
```
src/
├── main.rs                 # Entry point (CLI + MCP server)
├── cli/                    # CLI implementation
│   ├── mod.rs             # Module exports
│   ├── commands.rs        # Command handlers
│   └── args.rs            # Argument parsing
├── mcp/                    # MCP protocol implementation
│   ├── mod.rs             # Module exports
│   ├── server.rs          # Server implementation
│   ├── handlers.rs        # Request handlers
│   └── types.rs           # Protocol types
├── container/              # Container management
│   ├── mod.rs             # Module exports
│   ├── podman.rs          # Podman API client
│   └── lifecycle.rs       # Container lifecycle
├── git/                    # Git operations
│   ├── mod.rs             # Module exports
│   ├── operations.rs      # Git commands
│   └── worktree.rs        # Worktree management (partially implemented)
├── fs/                     # File system operations
│   ├── mod.rs             # Module exports
│   ├── watcher.rs         # FS event monitoring
│   └── paths.rs           # Path utilities
└── utils/                  # Utilities
    ├── mod.rs             # Module exports
    ├── timeout.rs         # Timeout management
    └── logging.rs         # Log ring buffer
```

### Kiro Specifications (`.kiro/`)
```
.kiro/
├── steering/               # Always-included project context
│   ├── product.md         # Business and product context
│   ├── tech.md            # Technology decisions
│   └── structure.md       # This file
└── specs/                  # Feature specifications
    └── [feature-name]/    # Per-feature directory
        ├── requirements.md # Functional requirements
        ├── design.md      # Technical design
        └── tasks.md       # Implementation tasks
```

### Serena Memories (`.serena/`)
```
.serena/
└── memories/
    ├── project_overview.md      # High-level project description
    ├── tech_stack.md            # Technology choices
    ├── api_specifications.md    # API design
    ├── code_style_conventions.md # Coding standards
    ├── task_completion_checklist.md # Development checklist
    ├── suggested_commands.md    # Common commands
    ├── mcp_technology_stack.md  # MCP-specific tech decisions
    └── mcp_sdk_correction.md    # rmcp migration notes
```

## Code Organization Patterns

### Module Structure
- **Public API at module root**: Export public interface in `mod.rs`
- **Implementation in submodules**: Internal logic in separate files
- **Types near usage**: Define types close to where they're used
- **Error types per module**: Module-specific error handling

### Async Patterns
- **tokio runtime**: All async operations use tokio
- **Timeouts on everything**: Explicit timeouts for external operations
- **Graceful shutdown**: Clean resource cleanup on termination

### Error Handling
- **anyhow for applications**: Simple error propagation in main code
- **thiserror for libraries**: Custom error types for public APIs (future)
- **Result everywhere**: Explicit error handling, no panics in production

## File Naming Conventions

### Rust Files
- **Snake case**: `file_name.rs`
- **Module index**: `mod.rs` for module roots
- **Test modules**: `#[cfg(test)]` in same file
- **Integration tests**: `tests/` directory (future)

### Documentation
- **Markdown**: `.md` extension
- **Uppercase meta**: `README.md`, `CLAUDE.md`
- **Lowercase content**: `about.md`, `requirements.md`

### Configuration
- **TOML format**: `Cargo.toml`, config files
- **JSON for data**: MCP messages, API payloads
- **Environment files**: `.env` for local config (gitignored)

## Import Organization

### Standard Order
1. **External crates**: Third-party dependencies
2. **Standard library**: `std::` imports (if needed)
3. **Local modules**: Internal `crate::` imports
4. **Super/self**: Relative imports last

### Current Example (from main.rs)
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use git2::{Repository, WorktreeAddOptions};
```

### Future Module Example
```rust
// External crates
use anyhow::Result;
use tokio::time::timeout;
use bollard::Docker;

// Standard library
use std::time::Duration;
use std::path::PathBuf;

// Local modules
use crate::mcp::server::McpServer;
use crate::container::podman::PodmanClient;

// Relative imports
use super::types::Request;
```

## Key Architectural Principles

### Zero-Copy Operations
- **Bind mounts over copying**: Direct filesystem access
- **Streaming over buffering**: Process data as it arrives
- **References over clones**: Minimize data duplication

### Event-Driven Design
- **FS events over polling**: React to changes immediately
- **Async/await everywhere**: Non-blocking I/O
- **Message passing**: Loosely coupled components

### Resource Management
- **Bounded queues**: Prevent unbounded growth
- **Explicit timeouts**: No infinite waits
- **Clean shutdown**: Proper resource cleanup

### MCP Server Architecture
- **stdio transport**: JSON-RPC over standard I/O
- **Handler registry**: Dynamic method dispatch
- **Type-safe messages**: Serde for serialization

### Testing Strategy (Future)
- **Unit tests**: In-module `#[cfg(test)]`
- **Integration tests**: `tests/` directory
- **Performance benchmarks**: `benches/` with criterion
- **Mock containers**: Test without real Podman

## Build Artifacts
- `target/debug/`: Development builds
- `target/release/`: Optimized builds
- `target/doc/`: Generated documentation
- **All gitignored**: Not committed to repository