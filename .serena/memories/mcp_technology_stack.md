# MCP Server Technology Stack

## 選定日: 2025-01-28

## MCPサーバー実装のための技術スタック

### MCP SDK (必須)
- **rust-mcp-sdk** v0.2.0
  - Model Context Protocolの公式Rust実装
  - stdio transport対応（ローカル実行）
  - 軽量Axumベース
  - マクロによるボイラープレート削減
  - 2025-06-18プロトコル対応（最新）
  ```toml
  rust-mcp-sdk = { version = "0.2.0", default-features = false, features = ["server", "macros", "stdio"] }
  ```

### コンテナ管理
- **bollard** v0.17
  - Docker/Podman API クライアント
  - 非同期対応、型安全
  - Podmanとの互換性確保
  ```toml
  bollard = "0.17"
  ```

### Git操作
- **gix** v0.64 (gitoxide)
  - 高速な差分処理、index操作
  - Pure Rust実装
  - メモリ効率的
  ```toml
  gix = { version = "0.64", features = ["worktree-mutation"] }
  ```

- **git2** v0.19 (libgit2)
  - git-notes等の完全なGit機能
  - 安定したAPI
  ```toml
  git2 = "0.19"
  ```

### 非同期ランタイム
- **tokio** v1.41
  - MCPサーバーに必須
  - フル機能セット
  ```toml
  tokio = { version = "1.41", features = ["full"] }
  ```

### ファイルシステム監視
- **notify** v7.0
  - クロスプラットフォーム対応
  - FSイベント駆動
  - デバウンス機能
  ```toml
  notify = "7.0"
  ```

### エラーハンドリング
- **anyhow** v1.0
  - シンプルなエラー処理
  ```toml
  anyhow = "1.0"
  ```

### その他の依存関係（将来追加予定）
- **bytes** v1.8 - 効率的なバイト操作
- **futures** v0.3 - 非同期ストリーム処理
- **ringbuf** v0.4 - リングバッファ（ログ管理）
- **serde/serde_json** - JSON-RPC通信（rust-mcp-sdkに含まれる可能性）

### Windows固有（必要に応じて）
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_JobObjects",
    "Win32_System_Threading",
    "Win32_Security"
]}
```

## 選定理由

### MCPサーバーとしての要件
1. **stdio通信**: rust-mcp-sdkのstdio transportでカバー
2. **JSON-RPC**: rust-mcp-sdkに組み込み済み
3. **非同期処理**: tokioで実現
4. **型安全性**: Rustの型システムを活用

### パフォーマンス要件への対応
1. **120ms以下のコミット**: gix + bind-mount
2. **メモリ効率**: ストリーミング処理
3. **CPU効率**: notifyのFSイベント駆動

### 除外した技術
- **CLIフレームワーク（clap）**: MCPサーバーには不要
- **ロギング（tracing-subscriber）**: MCPクライアント側で処理
- **テストユーティリティ（tempfile）**: 開発後期に追加

## 実装順序
1. MCPサーバー基盤（rust-mcp-sdk + tokio）
2. コンテナ制御（bollard）
3. Git操作（gix + git2）
4. FS監視（notify）
5. 統合とテスト