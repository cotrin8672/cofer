# Project Structure

## Root Directory Organization
```
cofer/
├── .claude/                # Claude Code 用コマンド (Kiro 連携)
│   └── commands/
│       └── kiro/           # Kiro ワークフロー専用コマンド定義
├── .gitignore              # リポジトリ無視設定 (.cofer/, target/ 含む)
├── .kiro/
│   └── steering/           # 常時読み込むプロジェクト文脈 (本ドキュメント)
├── .rustfmt.toml           # フォーマッタ設定 (LF 強制, インポート整理)
├── .serena/
│   ├── memories/           # Serena エージェント向け記憶 (プロジェクト概要など)
│   └── project.yml         # Serena 設定
├── AGENTS.md               # コーディングエージェント向け指針
├── about.md                # 既存 container-use の問題点と再実装方針
├── Cargo.lock
├── Cargo.toml              # Rust プロジェクトマニフェスト
├── CLAUDE.md               # Claude Code 用ガイドライン
├── clippy.toml             # Clippy 設定 (MSRV, 制約値)
├── rust-toolchain.toml     # Rust toolchain 固定 (stable + rustfmt/clippy)
├── src/
│   ├── git/
│   │   └── mod.rs          # Git ヘルパー実装
│   └── main.rs             # CLI エントリーポイント
└── target/                 # ビルド生成物 (gitignore)
```

> 備考: `.cofer/` は `cofer init` 実行時に生成されるベアリポジトリ/ワークツリー領域で、`.gitignore` によりバージョン管理対象外。

## Source Layout (`src/`)
### 現在の実装
- **`main.rs`**: `clap` ベースの CLI (`cofer init`) と `tokio` ランタイム初期化、`git` モジュール呼び出しを司るエントリーポイント。
- **`git/mod.rs`**: Git 操作ヘルパーを集約。
  - `init_remote_repository`: 既存リポジトリ配下に `.cofer` ベアリポジトリを初期化し、`cofer` リモートを追加。
  - `fetch_from_cofer` / `create_branch`: Cofer 専用ブランチの取得・作成を Git ライブラリで完結。
  - `create_worktree_from_cofer`: `.cofer/worktrees/<branch>` にワークツリーを生成 (存在する場合は再利用)。
  - `ensure_gitignore_has_cofer`: `.gitignore` に `.cofer/` エントリを保証。

### 拡張予定 (設計方針)
- `cli/`, `container/`, `mcp/`, `fs/` などのモジュール分割を行い、CLI/サーバ/Podman/FS 監視を責務ごとに整理。
- `utils/timeout`, `utils/logging` でタイムアウト管理やリングバッファロギングを共通化。

## Steering & Workflow Assets
- `.kiro/steering/`: `product.md` / `tech.md` / `structure.md` を常時読み込み。
- `.kiro/specs/`: まだ作成されていない。新機能着手時に `/kiro:spec-init` で生成予定。
- `.claude/commands/kiro/`: Kiro ワークフローを操作するカスタムコマンド群。
- `.serena/memories/`: プロジェクト概要や API 設計、テストチェックリストなどをエージェント間で共有。

## 生成・一時ディレクトリ
- `.cofer/`: CLI 初期化で作成するベアリポジトリおよびワークツリー格納領域 (常に gitignore 対象)。
- `target/`: Cargo のビルド成果物。
- `target/criterion/`: ベンチマーク結果 (必要時に生成)。

## コード組織のパターン
- モジュールごとに `mod.rs` で公開、実装はサブモジュールへ分割する方針。
- エラーハンドリングは `anyhow::Result` を返しつつ `context` で詳細を付加。
- トレースログは `tracing` を利用し、標準出力系マクロは `clippy.toml` で禁止。

## ファイル命名規則
- Rust ファイルはスネークケース (`worktree.rs` 等) を採用。
- モジュールのエントリには `mod.rs` を使用。
- ドキュメントは Markdown (`*.md`) で統一し、メタ情報ファイルは大文字先頭 (`CLAUDE.md`, `AGENTS.md`)。

## Import & Dependency Order
1. 外部クレート (`anyhow`, `clap`, `git2` 等)。
2. 標準ライブラリ (`std`)。
3. ローカルクレート (`crate::git::*`)。
4. 相対モジュール (`super`, `self`)。

`src/main.rs` では以下の順序を採用:
```rust
use crate::git::init_remote_repository;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;
```

## Architectural Principles
- **ゼロコピー志向**: bind-mount とライブラリベースの Git 操作で I/O/メモリ負荷を最小化。
- **イベント駆動**: `notify` による FS イベントをトリガに、コミット処理をバッチ化 (実装中)。
- **リソース制御**: すべての外部処理にタイムアウトを付与し、リングバッファでログを制限。
- **AI 連携前提**: MCP サーバを通じた AI ワークフロー統合を前提にコマンド/エラーフローを設計。

## テスト & ベンチマーク戦略 (計画)
- モジュール内ユニットテスト (`#[cfg(test)]`) と `rstest` によるフィクスチャベーステスト。
- `tests/` ディレクトリでの統合テスト (CLI 動作検証) の導入を予定。
- `criterion` でコミットパスやコンテナ起動の性能測定を自動化。

## ビルド成果物
- `target/debug/`, `target/release/`: Cargo 標準のビルド出力。
- `target/doc/`: `cargo doc` によるドキュメント生成物。
- いずれも `.gitignore` 済みでリポジトリには含めない。
