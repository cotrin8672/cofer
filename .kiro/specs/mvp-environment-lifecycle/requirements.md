# Requirements Document

## Introduction
Cofer MCP サーバは、dagger/container-use の MCP 実装互換のインターフェースを踏襲しつつ、環境の生成・コマンド実行・破棄の 3 ツールを高速かつ安定に提供することで、AI エージェントからの運用を簡素化しゼロコピー構成の恩恵を最短で届ける。

## Requirements

### Requirement 1: environment_create Tool
**Objective:** AI エージェントとして、既存リポジトリから即座に環境を生成したいので、`environment_create` ツールを呼び出すだけでワークツリーの準備とコンテナ起動を完了したい。

#### Acceptance Criteria
1. WHEN MCP クライアントが `environment_create` を `environment_source`, `title`, `image`, `from_git_ref` (任意) と共に呼び出したとき THEN Cofer MCP Server SHALL `.cofer/worktrees/<env_id>` を作成し Podman コンテナを `image` で起動し、container-use 互換の JSON (id/title/config/remote_ref/checkout_command/log_command/diff_command) をレスポンスする。
2. IF `from_git_ref` が指定されたとき THEN Cofer MCP Server SHALL 指定リファレンスを `cofer/<env_id>` ブランチとしてプッシュし、初回チェックアウトをワークツリーに適用する。
3. WHEN `allow_replace` が false AND `environment_source` に同一 ID の環境が存在するとき THEN Cofer MCP Server SHALL `conflict` 種別の構造化エラーを返し既存環境を保持する。
4. WHERE `environment_source` に `cofer` リモートが存在しないと検出したとき THEN Cofer MCP Server SHALL `precondition_failed` 種別のエラーを返し `cofer init` 実行を促す説明を含める。

### Requirement 2: environment_run_cmd Tool
**Objective:** 開発者として、生成済み環境内で任意コマンドを安全に実行したいので、`environment_run_cmd` ツールの呼び出しで Podman コンテナに接続し結果を取得したい。

#### Acceptance Criteria
1. WHEN MCP クライアントが `environment_run_cmd` を `environment_source`, `environment_id`, `command`, `shell` (任意), `use_entrypoint` (任意) で呼び出したとき THEN Cofer MCP Server SHALL 対象環境のコンテナでコマンドを実行し stdout/stderr の末尾 512 行と終了コードを含む結果 JSON を返す。
2. IF `background` が true AND `ports` が配列で指定されたとき THEN Cofer MCP Server SHALL 背景コンテナを起動しポートごとの `environment_internal` と `host_external` エンドポイント一覧をレスポンスに含める。
3. WHEN 実行時間が `COHERRA_TIMEOUT_RUN` またはデフォルト 600000 ミリ秒を超過したとき THEN Cofer MCP Server SHALL コンテナ内プロセスツリーを終了し `timeout` 種別のレスポンスを返す。
4. WHILE コマンド実行が継続している間 THE Cofer MCP Server SHALL 標準出力・標準エラーを 64KB リングバッファに保持しメモリ上限を超えないよう制御する。

### Requirement 3: environment_destroy Tool
**Objective:** プラットフォーム管理者として、利用終了した環境をクリーンに破棄したいので、`environment_destroy` ツールでコンテナとワークツリーを確実に削除したい。

#### Acceptance Criteria
1. WHEN MCP クライアントが `environment_destroy` を `environment_source` と `environment_id` で呼び出したとき THEN Cofer MCP Server SHALL 対象コンテナを停止・削除し `.cofer/worktrees/<env_id>` を除去したうえで削除結果 JSON (環境 ID・削除したパス・停止したコンテナ ID) を返す。
2. IF 環境に紐づく Podman プロセスまたは子プロセスが残存していると検出したとき THEN Cofer MCP Server SHALL Job Object 等価機構でプロセスツリーを強制終了し結果に終了種別を記録する。
3. WHEN `force` が true AND Git ワークツリーの削除で I/O エラーが発生したとき THEN Cofer MCP Server SHALL フォールバックでディレクトリを再試行し、最終結果に警告メッセージを含める。
4. WHERE 指定した `environment_id` が存在しないとき THEN Cofer MCP Server SHALL `not_found` 種別のエラーを返し再送信不要であることを説明する。

### Requirement 4: MCP Registration & Operational Guardrails
**Objective:** 開発支援オペレーターとして、MCP サーバが 3 ツールを安定提供できるようにしたいので、起動時登録と各ツールの制約を統一したい。

#### Acceptance Criteria
1. WHEN MCP サーバが起動したとき THEN Cofer MCP Server SHALL MCP カタログに `environment_create`, `environment_run_cmd`, `environment_destroy` の 3 ツールを登録し、各ツールの引数スキーマを container-use 互換構造で公開する。
2. IF 同時稼働環境数が設定上限に達したとき THEN Cofer MCP Server SHALL `limit_exceeded` 種別のエラーを返し新規環境作成リクエストを拒否する。
3. WHEN いずれかのツールが実行され AND 処理が内部リトライを経ても失敗したとき THEN Cofer MCP Server SHALL 失敗ステップ・推奨復旧手順・トレース ID を含む構造化エラーを返す。
4. WHILE サーバが稼働している間 THE Cofer MCP Server SHALL すべてのツール呼び出しを ISO 8601 タイムスタンプと実行モード (foreground/background) を含む `tracing` ログとして出力する。
