# Task Completion Checklist

## Before Committing Code

### 1. Code Quality
- [ ] Run `cargo fmt` to format code
- [ ] Run `cargo clippy -- -D warnings` to check for lints
- [ ] Fix all clippy warnings

### 2. Testing
- [ ] Run `cargo test` to ensure all tests pass
- [ ] Add tests for new functionality
- [ ] Verify performance requirements are met (if applicable)

### 3. Documentation
- [ ] Update doc comments for public APIs
- [ ] Update about.md if architecture changes
- [ ] Ensure examples in comments are accurate

### 4. Build Verification
- [ ] Run `cargo build` for debug build
- [ ] Run `cargo build --release` for release build
- [ ] Check for any build warnings

### 5. Performance Validation (for performance-critical changes)
- [ ] Test 1-file change → commit latency (target: ≤120ms)
- [ ] Test 1000-file batch commit (target: ≤2s)
- [ ] Verify memory usage is bounded (ring buffers)

### 6. Platform-specific Testing
- [ ] Test on Windows with proper path handling
- [ ] Verify Podman integration works
- [ ] Check timeout mechanisms function correctly

## Git Workflow
- Create meaningful commit messages following project conventions
- Reference issue numbers if applicable
- Keep commits focused and atomic

## Before Opening PR
- [ ] Rebase on latest main
- [ ] Squash WIP commits if needed
- [ ] Ensure CI passes
- [ ] Update CHANGELOG if applicable