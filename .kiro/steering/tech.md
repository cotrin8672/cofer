# Technology Stack

## Architecture
- **Language**: Rust (Edition 2024)
- **Runtime**: Asynchronous with tokio
- **Protocol**: Model Context Protocol (MCP) over stdio
- **Container**: Podman (Docker-compatible, rootless)
- **Version Control**: Native git libraries (gix + git2)

## Core Dependencies

### MCP Server Framework
- **rust-mcp-sdk** v0.2.0
  - stdio transport for local execution
  - JSON-RPC message handling
  - Macro-based boilerplate reduction
  ```toml
  rust-mcp-sdk = { version = "0.2.0", default-features = false, features = ["server", "macros", "stdio"] }
  ```

### Container Management
- **bollard** v0.17
  - Async Docker/Podman API client
  - Type-safe container operations
  ```toml
  bollard = "0.17"
  ```

### Git Operations
- **gix** v0.64 (gitoxide)
  - High-performance diff processing
  - Index and commit operations
  ```toml
  gix = { version = "0.64", features = ["worktree-mutation"] }
  ```
- **git2** v0.19 (libgit2)
  - Complete Git API (notes, submodules)
  ```toml
  git2 = "0.19"
  ```

### Async Runtime
- **tokio** v1.41
  - Full async/await support
  - Timer and I/O primitives
  ```toml
  tokio = { version = "1.41", features = ["full"] }
  ```

### File System Monitoring
- **notify** v7.0
  - Cross-platform FS events
  - Built-in debouncing (100-200ms)
  ```toml
  notify = "7.0"
  ```

### Error Handling
- **anyhow** v1.0
  - Simplified error propagation
  ```toml
  anyhow = "1.0"
  ```

## Development Environment

### System Requirements
- **Rust**: 1.75+ (Edition 2024 support)
- **Podman**: 4.0+ or Docker 20.10+
- **Git**: 2.30+ (for external git operations if needed)

### Platform-Specific
#### Windows
- Podman service: `podman system service --time=0`
- Force CRLF handling: `core.autocrlf=false`
- Job Objects for process management
- Long path support (`\\?\` prefix)

#### Linux/macOS
- SELinux: Use `:Z` option for bind mounts
- Rootless containers recommended

## Common Commands

### Build
```bash
cargo build              # Debug build
cargo build --release    # Release build for performance testing
```

### Test
```bash
cargo test              # Run all tests
cargo test -- --nocapture  # With output visibility
cargo test --release    # Performance tests
```

### Quality Checks
```bash
cargo fmt               # Format code
cargo clippy -- -D warnings  # Lint with warnings as errors
cargo doc --open        # Generate and view documentation
```

### MCP Server Operations
```bash
cargo run               # Start MCP server (stdio mode)
```

## Environment Variables

### Podman Configuration
- `PODMAN_SOCKET`: Custom Podman socket path (optional)
- `CONTAINER_HOST`: Alternative to DOCKER_HOST for Podman

### Development
- `RUST_LOG`: Logging level (debug, info, warn, error)
- `RUST_BACKTRACE`: Enable backtrace (1 or full)

### Runtime
- `COHERRA_TIMEOUT_RUN`: Override default run timeout (ms)
- `COHERRA_TIMEOUT_GIT`: Override git operation timeout (ms)
- `COHERRA_TIMEOUT_STARTUP`: Override startup timeout (ms)

## Port Configuration
- **MCP Server**: stdio (no network port)
- **Container ports**: Dynamically mapped as requested
- **Development ports**: User-configurable (3000, 9229, etc.)

## Performance Targets
- **Single file change → commit**: ≤120ms (NVMe, medium repo)
- **1000-file batch operation**: ≤2s
- **Memory cap**: Ring buffer limited (64KB or 512 lines for logs)
- **Debounce timing**: 100-200ms for FS events

## Future Considerations
- **bytes** v1.8: Efficient byte operations
- **futures** v0.3: Stream processing
- **ringbuf** v0.4: Ring buffer implementation
- **serde/serde_json**: Additional JSON processing
- **Windows-specific crates**: Job Object and long path support