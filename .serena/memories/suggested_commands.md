# Suggested Commands

## Build Commands
```bash
# Debug build
cargo build

# Release build (for performance testing)
cargo build --release

# Check without building
cargo check
```

## Testing Commands
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test <test_name>

# Run tests in release mode (for performance tests)
cargo test --release
```

## Development Commands
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint with clippy
cargo clippy -- -D warnings

# Fix clippy warnings
cargo clippy --fix

# Watch and rebuild on changes (requires cargo-watch)
cargo watch -x check -x test
```

## Documentation
```bash
# Generate and open documentation
cargo doc --open

# Generate docs with private items
cargo doc --document-private-items
```

## Performance & Benchmarking
```bash
# Run benchmarks (when implemented)
cargo bench

# Profile with release build
cargo build --release && time ./target/release/coherra
```

## Windows-specific Commands
```powershell
# Run Podman service (required before running coherra)
podman system service --time=0

# Check Podman status
podman version
podman info
```

## Git Commands (for development)
```bash
# Check worktree status
git worktree list

# View git notes (when implemented)
git notes --ref=refs/notes/yourtool list
```