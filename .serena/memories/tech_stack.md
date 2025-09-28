# Technology Stack

## Core Language
- **Rust** (Edition 2024)
- Project name: coherra
- Version: 0.1.0

## Planned Dependencies (設計仕様より)
- **bollard**: Docker/Podman API制御
- **gix (gitoxide)**: 高速Git操作（差分、index、commit）
- **git2 (libgit2)**: Git notes等の完全API
- **notify**: ファイルシステム監視（デバウンス100-200ms）
- **tokio**: 非同期ランタイム
- **bytes, futures**: 非同期I/O処理
- **arraydeque**: リングバッファ実装
- **windows**: Windows固有機能（Job Object等）
- **reqwest**: Podman REST API（代替案）

## Container Technology
- **Podman**: Dockerの代替、rootless実行推奨
- bind-mount使用（SELinux環境では:Zオプション）

## Development Environment
- Platform: Windows (win32)
- Git repository: Yes
- Worktree path: ~/.config/yourtool/worktrees/<env-id>
- Bare repo: ~/.config/yourtool/repos/project