# Coherra MVP 実装計画

## 目的
- MCP サーバーとして最低限の環境操作を提供し、`create_environment` と `run_command` の 2 操作だけで Podman バックエンドを介した実行ができる状態を目指す。
- 将来的な `watch-commit` / `note-append` / `up` / `down` などの機能は後回しにし、呼ばれた場合は明示的に未実装エラーを返す。

## 実装範囲
- MCP エンドポイント定義
  - `create_environment` と `run_command` を MCP ハンドラに登録。
  - その他のリクエストは `Unimplemented` エラーで応答。
- 状態管理
  - 環境 ID、コンテナ名、ワークツリーなどを保持する `EnvironmentHandle` 構造体を導入。
  - `tokio::sync::RwLock<HashMap>` でアクティブ環境を管理。
- 環境作成 (`create_environment`)
  - 入力: プロジェクトルート／env_id／イメージ名／任意の環境変数。
  - 流れ: イメージ準備 → コンテナ作成 → `/workdir` へホストワークツリーをバインドマウント → コンテナ起動。
  - 出力: 環境ハンドル情報（コンテナ ID、マウント先など）。
- コマンド実行 (`run_command`)
  - 入力: 対象 env_id、コマンド配列、タイムアウト、追加入力環境変数。
  - 流れ: Podman exec を起動 → stdout/stderr をリングバッファで末尾 64 KiB まで保存 → `tokio::time::timeout` で強制終了 → 結果返却。
  - 出力: exit code、stdout_tail、stderr_tail、実行時間、タイムアウトの有無。
- ログとトレーシング
  - 環境生成／実行開始・終了を `tracing` で記録。
  - エラーは全て anyhow ベースでラップし、MCP エラーコードに変換。
- シャットダウン処理
  - Ctrl+C などのシグナル時にアクティブ環境を列挙し、コンテナ停止＋削除を順次実行。

## 未実装として扱う領域
- `watch-commit` / `note-append` / `up` / `down` / `init` などの追加コマンド。
  - それぞれ TODO コメントとともにハンドラ雛形だけ置き、呼ばれたら `Unimplemented` エラー。
- Git 差分処理・自動コミット機能。
- ノート（git-notes）連携。
- ポートトンネル／サービス公開機構。
- Windows 固有の Job Object や CRLF 設定最適化（MVP ではログ出力のみ）。

## 作業タスクの分割
1. MCP リクエスト型とレスポンス型の定義／`rmcp` サーバーへの組み込み。
2. 環境レジストリと `EnvironmentHandle` の導入。
3. Podman クライアントラッパー（イメージプル／コンテナ生成／起動）。
4. `create_environment` の実装と Registry 登録処理。
5. `run_command` の実装（exec・ストリーミング・タイムアウト・レスポンス整形）。
6. シグナルハンドラの追加とクリーンアップ処理。
7. 未実装ハンドラの雛形作成（共通エラー応答）。
8. 手動検証手順のスクリプト化（環境作成→`echo hello` 実行）。

## 暫定テスト戦略
- Podman 実機依存のため自動テストは最小限。ユニットテストではレジストリの CRUD とタイムアウトロジックのみ検証。
- 手動 E2E: `cargo run` → MCP クライアントから `create_environment` → `run_command` (`echo hello`) の成功を確認。
- 将来は Podman を利用した統合テストを feature flag 付きで整備予定。

## リスクと留意点
- Podman デーモン未起動時の扱い：MVP ではエラーをそのままユーザーに返し、起動手順をログへ案内。
- 長時間実行コマンドのログ肥大：リングバッファ上限を超過分切り捨てとして記録。
- 複数同時リクエスト：レジストリは RwLock で防御するが、完全な並列性保証は今後の課題。
