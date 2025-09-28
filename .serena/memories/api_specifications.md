# API Specifications

## Core API Endpoints (JSON-RPC/CLI)

### init
Initialize environment with container and git worktree
```json
{
  "project_root": "/path/to/repo",
  "env_id": "adverb-animal",
  "image": "ghcr.io/org/dev:latest",
  "workdir": "/workdir",
  "mount": "/home/user/.yourtool/worktrees/adverb-animal",
  "submodules": true
}
```

### run
Execute command in container with timeout
```json
{
  "cmd": ["bash", "-lc", "npm ci && npm test"],
  "timeout_ms": 600000,
  "env": {"CI": "1"}
}
```

### watch-commit
Auto-commit on file changes with debouncing
```json
{
  "debounce_ms": 150,
  "nonbinary_only": true,
  "exclude": [".git/", "node_modules/", "target/"]
}
```

### note-append
Append to git-notes with size cap
```json
{
  "ref": "refs/notes/yourtool",
  "cap_lines": 120,
  "payload": "<base64>"
}
```

### up/down
Container lifecycle management
```json
{
  "ports": [3000, 9229],
  "entrypoint": null
}
```

## Timeout Defaults
- run: 600,000ms (10 minutes)
- git operations: 30,000ms
- startup: 30,000ms

## Error Handling
- All operations must have explicit timeouts
- Timeout exceeded → kill process tree
- Non-interactive auth only (fail fast)
- Return resolution hints on errors