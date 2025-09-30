# Technology Stack

## Architecture
- **Language**: Rust (Edition 2024)
- **Runtime**: Asynchronous with tokio
- **Protocol**: Model Context Protocol (MCP) over stdio
- **Container**: Podman (Docker-compatible, rootless)
- **Version Control**: Native git libraries (gix + git2)

## Core Dependencies

### MCP Server Framework
- **rmcp** v0.7.0 (Official Rust MCP SDK)
  - Server implementation support
  - JSON-RPC message handling
  - Improved API design
  ```toml
  rmcp = { version = "0.7.0", features = ["server"] }
  ```

### Container Management
- **bollard** v0.19.2
  - Async Docker/Podman API client
  - Type-safe container operations
  - Enhanced Windows support
  ```toml
  bollard = "0.19.2"
  ```

### Git Operations
- **gix** v0.73.0 (gitoxide)
  - High-performance diff processing
  - Index and commit operations
  - Improved worktree support
  ```toml
  gix = { version = "0.73.0", features = ["worktree-mutation"] }
  ```
- **git2** v0.20.2 (libgit2)
  - Complete Git API (notes, submodules)
  - Worktree creation and management (actively used)
  ```toml
  git2 = "0.20.2"
  ```

### Async Runtime
- **tokio** v1.47.1
  - Full async/await support
  - Timer and I/O primitives
  - Enhanced performance
  ```toml
  tokio = { version = "1.47.1", features = ["full"] }
  ```

### File System Monitoring
- **notify** v8.2.0
  - Cross-platform FS events
  - Built-in debouncing (100-200ms)
  - Improved Windows support
  ```toml
  notify = "8.2.0"
  ```

### Error Handling
- **anyhow** v1.0.100
  - Simplified error propagation
  - Context-aware error messages
  ```toml
  anyhow = "1.0.100"
  ```

### CLI and User Interface
- **clap** v4.5.8
  - Command-line argument parsing
  - Derive-based API
  - Subcommand support
  ```toml
  clap = { version = "4.5.8", features = ["derive"] }
  ```

### Logging and Diagnostics
- **tracing** v0.1.41
  - Structured, async-aware logging
  - Performance diagnostics
  ```toml
  tracing = "0.1.41"
  ```
- **tracing-subscriber** v0.3.20
  - Log formatting and filtering
  - Environment-based configuration
  ```toml
  tracing-subscriber = { version = "0.3.20", features = ["env-filter"] }
  ```

### Additional Runtime Dependencies
- **async-trait** v0.1.89 - Async trait support
- **bytes** v1.10.1 - Efficient byte operations
- **futures** v0.3.31 - Stream and future utilities
- **serde** v1.0.228 - Serialization framework
- **serde_json** v1.0.145 - JSON processing
- **dirs** v5.0 - Platform-specific directories

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

## Testing Dependencies (Development)
Comprehensive testing infrastructure for quality assurance:
- **criterion** v0.7.0 - Benchmarking framework
- **insta** v1.43.2 - Snapshot testing with JSON support
- **mockall** v0.13.1 - Mock object generation
- **pretty_assertions** v1.4.1 - Enhanced assertion diffs
- **rstest** v0.26.1 - Fixture-based testing
- **serial_test** v3.2.0 - Serial test execution
- **tempfile** v3.23.0 - Temporary file handling
- **test-case** v3.3.1 - Parameterized testing
- **tokio-test** v0.4.4 - Async test utilities
- **wiremock** v0.6.5 - HTTP mocking
- **cargo-husky** v1.5.0 - Git hooks integration

## Future Considerations
- **ringbuf** v0.4: Ring buffer implementation for log management
- **Windows-specific crates**: Job Object and long path support
- **Container runtime alternatives**: Investigation of native container APIs