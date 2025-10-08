# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project: Coherra - High-Performance Container Environment Manager

Cofer(コフェル)は、既存のcontainer-use実装をRustで置き換える高性能コンテナ環境管理ツールです。Podmanとbind-mountを活用し、ゼロコピー化による大幅な性能改善を実現します。

### Core Architecture
- **Bind-mount戦略**: Export/Importを撤廃し、worktreeを直接`/workdir`にマウント（ゼロコピー）
- **Git操作**: gix（高速差分処理）+ git2（完全API）の併用、外部git CLIは最小限
- **非同期処理**: tokioベース、すべての外部実行に明示的timeout
- **監視**: notifyによるFSイベント駆動（ポーリング廃止）

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build (性能測定用)

# Test
cargo test              # Run all tests
cargo test -- --nocapture  # With output
cargo test --release    # Performance tests

# Quality Checks (実行必須)
cargo fmt               # Format code
cargo clippy -- -D warnings  # Lint check

# Documentation
cargo doc --open        # Generate and view docs
```

## Performance Requirements
- 1ファイル変更→commit: **≤120ms** (NVMe, 中規模リポ)
- 1,000ファイルバッチ: **≤2s**
- メモリ上限: リングバッファで制限（ログ64KB or 512行）
- タイムアウト: run=600s, git=30s, startup=30s

## Architecture Overview

### 現行問題の解決方針
| 問題                             | 解決策                        |
| -------------------------------- | ----------------------------- |
| 全量Export（Wipe:true）のI/O爆発 | bind-mountでゼロコピー化      |
| Git CLIの全量バッファでハング    | gix/git2ライブラリ + 逐次読取 |
| 1秒ポーリングのCPU浪費           | notifyのFSイベント駆動        |
| ポート公開の直列処理             | Podman一括設定                |

### 主要コンポーネント（計画）
- **Podman制御**: bollard or reqwest経由
- **Git操作**: gix（差分/index/commit）、git2（notes等）
- **FS監視**: notify（debounce 100-200ms）
- **Windows対応**: Job Object、長パス（`\\?\`）、CRLF固定

## API仕様（JSON-RPC/CLI共通）

### init: 環境初期化
```json
{"project_root": "/path", "env_id": "name", "image": "ghcr.io/org/dev:latest"}
```

### run: コマンド実行
```json
{"cmd": ["bash", "-c", "npm test"], "timeout_ms": 600000}
```

### watch-commit: 自動コミット
```json
{"debounce_ms": 150, "nonbinary_only": true, "exclude": [".git/", "node_modules/"]}
```

## Windows固有の注意事項
- Podman service起動: `podman system service --time=0`
- CRLF設定: `core.autocrlf=false`を強制
- プロセス管理: Job Objectで確実なツリーkill

## Claude Code Spec-Driven Development

Kiro-style Spec Driven Development implementation using claude code slash commands, hooks and agents.

### Project Context

#### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`
- Commands: `.claude/commands/`

#### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalize development process for individual features

#### Active Specifications
- Check `.kiro/specs/` for active specifications
- `mvp-environment-lifecycle`: 環境作成/実行/削除のMVP計画
- Use `/kiro:spec-status [feature-name]` to check progress

### Development Guidelines
- Think in English, but generate responses in Japanese (思考は英語、回答の生成は日本語で行うように)

### Workflow

#### Phase 0: Steering (Optional)
`/kiro:steering` - Create/update steering documents
`/kiro:steering-custom` - Create custom steering for specialized contexts

Note: Optional for new features or small additions. You can proceed directly to spec-init.

#### Phase 1: Specification Creation
1. `/kiro:spec-init [detailed description]` - Initialize spec with detailed project description
2. `/kiro:spec-requirements [feature]` - Generate requirements document
3. `/kiro:spec-design [feature]` - Interactive: "Have you reviewed requirements.md? [y/N]"
4. `/kiro:spec-tasks [feature]` - Interactive: Confirms both requirements and design review

#### Phase 2: Progress Tracking
`/kiro:spec-status [feature]` - Check current progress and phases

### Development Rules
1. **Consider steering**: Run `/kiro:steering` before major development (optional for new features)
2. **Follow 3-phase approval workflow**: Requirements → Design → Tasks → Implementation
3. **Approval required**: Each phase requires human review (interactive prompt or manual)
4. **No skipping phases**: Design requires approved requirements; Tasks require approved design
5. **Update task status**: Mark tasks as completed when working on them
6. **Keep steering current**: Run `/kiro:steering` after significant changes
7. **Check spec compliance**: Use `/kiro:spec-status` to verify alignment

### Steering Configuration

#### Current Steering Files
Managed by `/kiro:steering` command. Updates here reflect command changes.

#### Active Steering Files
- `product.md`: Always included - Product context and business objectives
- `tech.md`: Always included - Technology stack and architectural decisions
- `structure.md`: Always included - File organization and code patterns

#### Custom Steering Files
<!-- Added by /kiro:steering-custom command -->
<!-- Format:
- `filename.md`: Mode - Pattern(s) - Description
  Mode: Always|Conditional|Manual
  Pattern: File patterns for Conditional mode
-->

#### Inclusion Modes
- **Always**: Loaded in every interaction (default)
- **Conditional**: Loaded for specific file patterns (e.g., "*.test.js")
- **Manual**: Reference with `@filename.md` syntax
