# Implementation Plan

全タスクはテストファースト (TDD) を前提とし、各サブタスクは RED → GREEN → REFACTOR のサイクルを完了し、記述した受け入れテストが自動化され安定成功するまで完了とみなさない。

- [ ] 1. 環境管理の基盤を整備する
- [ ] 1.1 EnvironmentRegistry を実装し同時稼働数とロック制御を提供する
  - **Implementation Scope**: 環境 ID ごとに排他制御を行うレジストリを実装し、登録・取得・解放・削除・カウントの操作が永続的に整合するようにする。登録データには環境 ID、ワークツリーのパス、対応するコンテナ識別子、背景コンテナのリスト、直近の実行メタ情報を含める。
  - **Concurrency Rules**: 同一環境 ID への重複登録は禁止し、操作が失敗した場合は元の状態へロールバックする。操作中に panic やエラーが発生してもロックがリークしないようにし、Drop/RAII で確実に開放する。
  - **Limit Enforcement**: 設定された最大同時稼働数 (デフォルト 4) を超過した登録は `limit_exceeded` 相当を返し、必要に応じて待機させず即時失敗させる。強制削除や再登録時にも整合性を保つ。
  - **TDD Acceptance**:
    - 並列に 10 個のスレッドが同じ環境 ID を登録しようとするテストで、最初の 1 件のみ成功し、残りが競合として失敗すること。
    - 上限値を 2 に設定した場合、3 つ目の登録が `limit_exceeded` を返すこと。
    - 登録後に panic を発生させるテストで、ロックが解放され後続の登録が成功すること。
  - _Requirements: 1.3, 4.2_

- [ ] 1.2 共通レスポンス変換とトレース ID 付与を行う
  - **Implementation Scope**: container-use と互換のレスポンス構造体を定義し、環境情報、背景コマンドのエンドポイント、DestroySummary を JSON 化するヘルパーを実装する。蛇腹ケース (snake_case) デシリアライズ、任意フィールド省略、警告メッセージ配列の組み立てをサポートする。
  - **Traceability**: すべてのレスポンスとエラーにユニークなトレース ID を自動生成して埋め込み、`tracing` ログにも同一 ID を出力する。
  - **Error Mapping**: アプリケーション内部のエラー分類を `conflict`、`precondition_failed`、`not_found`、`limit_exceeded`、`timeout`、`internal` にマッピングし、追加情報として再試行推奨・問題箇所・サマリテキストを付与する。
  - **TDD Acceptance**:
    - 正常レスポンスで remote_ref / checkout_command / log_command / diff_command が container-use 仕様どおりに含まれるスナップショットテスト。
    - エラーオブジェクトをマッピングした際に、コード・メッセージ・トレース ID が JSON に含まれる検証。
    - 複数警告がある DestroySummary を JSON 化したとき、順序・内容が期待通りであること。
  - _Requirements: 1.1, 4.3_

- [ ] 2. environment_create ツールの機能を完成させる
- [ ] 2.1 入力検証とリポジトリ前提条件チェックを追加する
  - **Implementation Scope**: ツール引数の必須チェック (`environment_source`, `title`, `image`) と制約チェック (空文字禁止、パスの正規化、許可パス外アクセスの拒否) を実装する。
  - **Precondition Validation**: `environment_source` が Git 管理下で `cofer` リモートが存在し、`.cofer` ベアリポジトリが初期化されているか `ensure_remote` を用いて確認し、満たさなければ `precondition_failed` で失敗させる。
  - **Conflict Handling**: レジストリや `.cofer/worktrees` に同一 ID が存在する場合、`allow_replace` が false なら `conflict` を返し、true なら既存環境を削除して置換できる準備を整える。
  - **TDD Acceptance**:
    - ベアリポジトリ未作成の Git リポジトリに対して呼び出し、`precondition_failed` エラーと追加説明が返るエンドツーエンドテスト。
    - 無効なパス (`../outside`) を指定した場合にアクセス拒否となるセキュリティテスト。
    - `allow_replace` の挙動を検証するテストで、フラグに応じて成功／失敗が切り替わること。
  - _Requirements: 1.1, 1.3, 1.4_

- [ ] 2.2 Git ワークツリー生成フローを構築する
  - **Implementation Scope**: 指定された `from_git_ref` を解決し、`cofer/<env_id>` ブランチをプッシュしてから `.cofer/worktrees/<env_id>` を生成する。サブモジュールが存在する場合は警告メッセージをレスポンスに付与する。
  - **Idempotency**: 同じ env_id に対して複数回生成を試みた場合、既存ワークツリーを再利用するか、必要に応じて削除して再作成できるようにする。
  - **Error Reporting**: push 失敗やワークツリー生成失敗時に、Git コマンドの出力と参照 ID を含む構造化エラーを返す。
  - **TDD Acceptance**:
    - HEAD と main 以外のブランチ名を指定して正しいブランチが作られることを検証するテスト。
    - サブモジュールを含むリポジトリで警告メッセージが返る統合テスト。
    - 二重呼び出しで再利用と競合解消が働く並列テスト。
  - _Requirements: 1.1, 1.2_

- [ ] 2.3 Podman コンテナ起動とレスポンス整形を行う
  - **Implementation Scope**: `PodmanManager` を利用して bind-mount 済みのコンテナを起動し、環境用の標準コマンドが実行可能な状態にする。起動後はコンテナ ID、mount 情報、イメージタグを EnvironmentResponse に組み立てて返す。
  - **Lifecycle Handling**: 既にコンテナが存在する場合は再利用し、イメージ更新や置換時には古いコンテナを安全に停止してから新規作成する。
  - **Observability**: 起動時間と Podman API のレスポンスを `tracing` で計測し、閾値超過時に警告ログを残す。
  - **TDD Acceptance**:
    - Podman をモック化し、起動依頼・設定内容が期待どおりであることを検証する単体テスト。
    - レスポンス JSON に必要フィールドが揃うスナップショットテスト。
    - 既存コンテナ再利用パスの挙動を確認するシナリオテスト。
  - _Requirements: 1.1, 4.4_

- [ ] 3. environment_run_cmd ツールを実装する
- [ ] 3.1 フォアグラウンド実行とタイムアウト制御を実装する
  - **Implementation Scope**: フォアグラウンドコマンドを既存コンテナ内で実行し、標準出力・標準エラーを 64KB / 512 行のリングバッファに収集する。実行時間を計測し、完了後に exit code・stdout_tail・stderr_tail・elapsed_ms を組み立てる。
  - **Timeout Enforcement**: 実行時間が `COHERRA_TIMEOUT_RUN` あるいはハード上限を超過した場合、Podman exec を終了し、プロセスツリーを kill して `timeout` エラーを返す。
  - **Side Effects**: コマンド完了後にファイルシステム差分が発生した場合、GitFacade に反映させてワークツリーが最新状態になるようにする。
  - **TDD Acceptance**:
    - 正常終了時のレスポンス内容を検証する統合テスト (exit code、stdout、stderr、elapsed_ms)。
    - 人為的に長時間化したコマンドで timeout が発生し、プロセスが存在しないことを確認するテスト。
    - コマンド実行後の差分が Git index に反映されることを確認するテスト。
  - _Requirements: 2.1, 2.3, 2.4_

- [ ] 3.2 背景実行とポート公開を処理する
  - **Implementation Scope**: `background=true` の呼び出しでは一度だけ専用コンテナを生成し、要求ポートを Podman 経由で公開する。エンドポイント情報に `environment_internal` と `host_external` を含める。
  - **Lifecycle**: 背景コンテナを環境レジストリに格納し、停止依頼時に一括で破棄できるようにする。ポート競合時は即時エラーで通知する。
  - **Output Contract**: レスポンスに背景コンテナ ID、ポートマッピング、長期実行時のリセット注意を記載する。
  - **TDD Acceptance**:
    - ポートの割り当てとレスポンス JSON を確認する統合テスト。
    - 既に背景コンテナが存在する場合に再利用されることを確認するテスト。
    - ポート競合時に適切なエラーコードが返るテスト。
  - _Requirements: 2.1, 2.2_

- [ ] 3.3 失敗時レスポンスと再実行ガイドを整備する
  - **Implementation Scope**: コマンド失敗時に exit code、stderr tail、再実行時の推奨事項、失敗セクションのテキストをレスポンスへ含める。ユーザが同じコマンドを再度実行する際に参照できるガイドラインを記載する。
  - **Error Categorization**: Podman・コンテナ内エラー・Git 反映失敗を個別に識別し、それぞれ別のエラーコードと説明を付与する。
  - **Logging**: 失敗時のログがリングバッファから漏れないようにし、必要であれば先頭部分も取得できる仕組みを検討する。
  - **TDD Acceptance**:
    - 非ゼロ終了コードを返すコマンド実行で、レスポンスに必要なフィールドが揃うことを検証するテスト。
    - Podman 例外とコンテナ内例外が異なる分類で返ることを確認するテスト。
    - 再実行ガイドがレスポンス中に含まれ、テキスト内容が期待どおりであることを検証するテスト。
  - _Requirements: 2.3, 4.3_

- [ ] 4. environment_destroy ツールを実装する
- [ ] 4.1 コンテナ停止と残存プロセスの整理を行う
  - **Implementation Scope**: 環境コンテナおよび背景コンテナを停止・削除し、関連する Podman exec やサブプロセスを Job Object もしくは同等機構で強制終了する。停止結果を DestroySummary へ記録する。
  - **Observability**: 停止操作の成功・失敗を `tracing` に出力し、タイムアウトや強制終了の有無を記録する。
  - **Cleanup**: 残留するネットワークリソースやボリュームを検知し、必要に応じて追加清掃を行う。
  - **TDD Acceptance**:
    - コンテナ停止後に Podman へ問い合わせてコンテナが存在しないことを確認する統合テスト。
    - 背景コンテナとフォアグラウンドコンテナの双方が終了することを検証するテスト。
    - 強制終了が発生した場合に DestroySummary の警告へ反映されるテスト。
  - _Requirements: 3.1, 3.2_

- [ ] 4.2 ワークツリー削除と force オプションを実装する
  - **Implementation Scope**: `.cofer/worktrees/<env_id>` と対応するブランチを削除し、`force=true` 指定時には I/O エラー発生後に再試行して結果に警告メッセージを含める。
  - **Rolloff**: 削除後に remote pruning を行い、孤立したリファレンスが残らないようにする。
  - **Failure Handling**: ファイルアクセス権限の不足などで削除できない場合、警告とともにユーザへ追加手順を提示する。
  - **TDD Acceptance**:
    - 正常削除時にブランチとワークツリーが存在しないことを確認するテスト。
    - 故意にロックされたファイルがある状態で force=true により警告付きで成功するテスト。
    - 削除途中でエラーになった場合に `internal` エラーと解決策案内が返るテスト。
  - _Requirements: 3.1, 3.3_

- [ ] 4.3 存在しない環境 ID の扱いを確立する
  - **Implementation Scope**: レジストリ・ファイルシステム・Podman のいずれでも対象が見つからない場合、`not_found` を返し、再送や調査のための説明を含める。
  - **Safety**: ネガティブケースでも副作用を発生させないようにし、誤った削除操作が発生しないことを保証する。
  - **Logging**: 発生した環境 ID を監査ログに記録し、不正リクエスト検知の材料とする。
  - **TDD Acceptance**:
    - 未登録 ID の呼び出しで `not_found` が返り、副作用が無いことを確認するテスト。
    - 過去に削除済み環境 ID を再度指定した場合にも同様のレスポンスが返るテスト。
    - ログに痕跡が残ることを確認するテスト (ログキャプチャ使用)。
  - _Requirements: 3.4_

- [ ] 5. MCP サーバとツールルーターを統合する
- [ ] 5.1 ToolRouter を実装しサービスを接続する
  - **Implementation Scope**: ツール名ごとに EnvironmentService・CommandExecutionService・EnvironmentDestroyService へディスパッチし、環境レジストリとレスポンス整形ヘルパーを統合する。共通の前処理 (environment_source の正規化、トレース ID 注入) を行う。
  - **Error Propagation**: サービス層のエラーを前述のエラーマッピングで統一し、MCP 経由でも同様の構造化レスポンスを返す。
  - **Instrumentation**: 各呼び出しの開始・終了・エラーを `tracing` に出力し、latency を測定する。
  - **TDD Acceptance**:
    - 各ツール名に対するディスパッチが正しいサービスモックへ届くことを検証する単体テスト。
    - サービスから返却されたエラーが適切にマッピングされることを確認するテスト。
    - トレース ID がレスポンスとログ双方に出力されることを確認するテスト。
  - _Requirements: 4.1, 4.3_

- [ ] 5.2 rmcp サーバの起動処理を仕上げる
  - **Implementation Scope**: Tokio 上で `rmcp` ストリームを起動し、定義したツールを catalog へ登録、標準入出力または IPC を通じてクライアントと通信できるようにする。サーバ停止時はコンテナをクリーンアップする。
  - **Signal Handling**: SIGINT/SIGTERM 受領時に安全にシャットダウンし、レジストリ内の環境を順次破棄するフックを登録する。
  - **Startup Validation**: 起動時に必要な依存 (Podman、Git) をチェックし、問題があればログへ出力して起動を止める。
  - **TDD Acceptance**:
    - MCP クライアント相当の統合テストで、catalog に 3 ツールが登録され、callTool が正常に応答することを確認する。
    - 起動前チェックで Podman ソケットが無い場合にエラー終了するテスト。
    - シャットダウン時にレジストリ内の環境が空になることを検証するテスト。
  - _Requirements: 4.1, 4.4_

- [ ] 6. 横断的なエラーハンドリング・ログ・セキュリティを仕上げる
- [ ] 6.1 ツール共通のエラーコードとトレース出力を統合する
  - **Implementation Scope**: サービス層のエラー型を ToolRouter で受け、マッピング表に従いコードとメッセージを整形する。トレース ID、再試行推奨、追加ログ参照 URL を出力する。
  - **Observability**: 重要エラー時には `error` レベルログを出力し、成功時は `info` で概況を残す。メトリクス収集のためにエラー種別ごとのカウンタを設ける。
  - **TDD Acceptance**:
    - 各エラー種別に対して期待する JSON が生成されるスナップショットテスト。
    - トレース ID が常に含まれること、重複しないことを検証するテスト。
    - 追加ログ URL が生成されるケースと生成されないケースの両方を検証するテスト。
  - _Requirement
