# Technology Stack

## Architecture
- **言語**: Rust (Edition 2024) を採用し、将来的な async/zero-copy パイプラインを全面的にサポート。
- **実行モデル**: `tokio` による非同期ランタイムで CLI から将来のサーバ機能まで統一。
- **Git 連携**: `gix` と `git2` を併用し、高速差分処理と完全なリポジトリ操作 API を内製化。
- **コンテナ制御 (計画)**: Podman を想定し、Docker 互換 API (`bollard`) で管理。
- **プロトコル (計画)**: Model Context Protocol (MCP) を `rmcp` で提供し、AI エージェント経由の操作を標準化。

## ランタイム & アプリケーション層
- **clap v4.5.8**: CLI インターフェース。現在は `cofer init` サブコマンドのみを提供。
- **tokio v1.47.1**: `#[tokio::main]` による非同期エントリーポイント。
- **tracing v0.1.41 / tracing-subscriber v0.3.20**: 構造化ログと環境変数ベースのフィルタリング。
- **anyhow v1.0.100**: コンテキスト付きエラーハンドリングを簡潔に保持。

## Git Operations
- **gix v0.73.0**: 差分計算・インデックス操作・ワークツリー操作を高速化 (機能統合中)。
- **git2 v0.20.2**: ブランチ作成、ワークツリー生成、notes 等フル機能 API を提供。

## コンテナ & MCP (実装予定)
- **bollard v0.19.2**: Podman/Docker API クライアント。コンテナ起動とポート公開の一括設定に利用予定。
- **rmcp v0.7.0 (server feature)**: MCP サーバ実装。AI アシスタントとの統合地点として今後実装を進める。

## 非同期・ユーティリティ
- **async-trait v0.1.89**: 非同期トレイト実装用。
- **futures v0.3.31 / bytes v1.10.1**: Stream/Future ユーティリティと効率的なバッファ操作。
- **dirs v5.0**: プロジェクト固有の設定ディレクトリ解決 (初期化時に活用予定)。

## ファイルシステム監視 (計画)
- **notify v8.2.0**: クロスプラットフォーム FS イベント。150ms 前後のデバウンスで自動コミットフローを支える予定。

## シリアライゼーション
- **serde v1.0.228 (+derive)** / **serde_json v1.0.145**: JSON ベースの CLI/MCP ペイロード処理基盤。

## 開発環境
### システム要件
- Rust 1.75+ (Edition 2024 対応)。
- Podman 4.0+ または Docker 20.10+ (コンテナ統合フェーズで必須)。
- Git 2.30+ (外部 CLI を最小限補助で使用する場合のみ)。

### ツールチェーン設定
- `rust-toolchain.toml`: `stable` に固定し、`rustfmt`/`clippy` コンポーネントを同梱。
- `.rustfmt.toml`: インポートのグルーピングやコメント整形を定義 (`newline_style = "Unix"` で統一)。
- `clippy.toml`: `msrv = 1.75`、複雑度や関数長の上限、標準出力系マクロの禁止を明文化。

## Common Commands
```bash
cargo build              # デバッグビルド
cargo build --release    # パフォーマンス計測向けリリースビルド
cargo test               # テスト実行 (現時点でテストスイートは未整備)
cargo fmt                # Rustfmt
cargo clippy -- -D warnings  # Lint を Warning=Error で実行
cargo doc --open         # ドキュメント生成
```

## Environment Variables
- `PODMAN_SOCKET` / `CONTAINER_HOST`: Podman デーモンのソケット上書き (統合作業時に使用)。
- `RUST_LOG`: `tracing` レベル制御 (`info` デフォルト想定)。
- `RUST_BACKTRACE`: バックトレースの有効化。
- `COHERRA_TIMEOUT_RUN` / `COHERRA_TIMEOUT_GIT` / `COHERRA_TIMEOUT_STARTUP`: 既定タイムアウトの上書き (ms 単位)。

## Port Configuration
- 現状は stdio ベースの CLI のみでポート未使用。
- コンテナ公開時は要求ポートを一括で Podman に割り当てる設計を予定。

## Performance Targets
- 単一ファイル変更→commit ≤120ms。
- 1,000 ファイルバッチ ≤2s。
- ログはリングバッファ (64KB or 512 行) で頭打ち。
- FS イベントのデバウンス 100-200ms。

## Testing & QA Tooling
- **criterion v0.7.0**: ベンチマーク (HTML レポート対応)。
- **insta v1.43.2**: スナップショットテスト。
- **mockall v0.13.1 / wiremock v0.6.5**: モック生成・HTTP モック。
- **rstest / test-case / serial_test**: フィクスチャ・パラメトリックテスト補助。
- **tempfile / pretty_assertions / tokio-test**: テストユーティリティ。
- **cargo-husky**: Git フック連携。

## Pending Integration Notes
- `bollard`, `notify`, `rmcp` は依存として確保済みだが、現行コードベースでは未配線。ロードマップに沿って順次導入する。
- Git 周辺ロジックは `git` モジュールに集約済みで、CLI サブコマンド拡張時に再利用する想定。
