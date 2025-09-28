# Code Style and Conventions

## Rust Conventions
- Edition: 2024
- Follow standard Rust naming conventions:
  - snake_case for functions and variables
  - PascalCase for types and traits
  - SCREAMING_SNAKE_CASE for constants

## Error Handling
- Use `Result<T, E>` for fallible operations
- Implement custom error types with `thiserror` or similar
- Always provide timeout for external operations
- Explicit error messages with resolution hints

## Async/Concurrency
- Use `tokio` for async runtime
- Prefer `async`/`await` over manual futures
- Use channels for inter-task communication
- Implement proper cancellation with timeouts

## Performance Requirements
- 1ファイル更新→commit: ≤120ms (NVMe, 中規模リポ)
- 1,000ファイル変化→commit: ≤2s
- Use ring buffers for log management (64KB or 512 lines cap)

## Windows Compatibility
- Use `windows` crate for platform-specific features
- Handle long paths with `\\?\` prefix
- Implement Job Object for process tree management
- Set `core.autocrlf` and `core.filemode=false` on init

## Git Integration
- Prefer gix for performance-critical operations
- Use git2 for complete API needs (e.g., git-notes)
- Always non-interactive authentication
- No pager, no TTY prompts

## Testing
- Unit tests for all core logic
- Integration tests for Podman interactions
- Performance benchmarks for critical paths
- Mock external dependencies in tests

## Documentation
- Document public APIs with `///` doc comments
- Include examples in doc comments
- Document safety invariants for unsafe code
- Keep about.md updated with architecture decisions