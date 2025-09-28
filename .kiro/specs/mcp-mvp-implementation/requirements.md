# Requirements Document

## 概要
Coherra MVPは、MCPプロトコルを通じてコンテナ環境の管理を提供する最小限の実装です。このMVPでは、`create_environment`と`run_command`の2つの基本操作のみを実装し、Podmanバックエンドを介した安全で効率的なコンテナ実行環境を提供します。将来的な機能拡張の基盤となる設計を行いながら、段階的な開発を可能にします。

## 要件

### 要件1: MCPサーバー基盤
**目的:** 開発者として、MCPプロトコルに準拠したサーバーを利用して、標準的なJSONRPCインターフェースでコンテナ環境を操作できるようにする。

#### 受け入れ条件

1. WHEN MCPクライアントがstdio経由で接続を試みる THEN Coherraサーバー SHALL 標準入出力を介してJSONRPCメッセージを受信し処理する
2. WHEN 無効なJSONRPCメッセージが受信される THEN Coherraサーバー SHALL 適切なエラーコードとメッセージを含むJSONRPCエラーレスポンスを返す
3. WHILE MCPセッションがアクティブである間 THE Coherraサーバー SHALL 同時に複数の環境を管理し、それぞれに一意の環境IDを割り当てる
4. WHEN サポートされていないMCPメソッド（watch-commit、note-append、up、down等）が呼び出される THEN Coherraサーバー SHALL "Unimplemented"エラーコードを返す

### 要件2: 環境作成機能
**目的:** 開発者として、Podmanコンテナベースの隔離された実行環境を作成して、プロジェクトのコードを安全に実行できるようにする。

#### 受け入れ条件

1. WHEN `create_environment`メソッドが有効なパラメータ（project_root、env_id、image）で呼び出される THEN Coherraサーバー SHALL 新しいPodmanコンテナを作成し起動する
2. IF 指定されたコンテナイメージがローカルに存在しない THEN Coherraサーバー SHALL レジストリからイメージをプルしてからコンテナを作成する
3. WHEN コンテナが作成される THEN Coherraサーバー SHALL プロジェクトルートを`/workdir`にバインドマウントする
4. IF 同じenv_idで環境がすでに存在する THEN Coherraサーバー SHALL 適切なエラーメッセージを返す
5. WHEN 環境変数が提供される THEN Coherraサーバー SHALL それらをコンテナ環境に設定する
6. WHEN 環境作成が成功する THEN Coherraサーバー SHALL 環境ハンドル情報（コンテナID、マウント先パス）を返す

### 要件3: コマンド実行機能
**目的:** 開発者として、作成した環境内で任意のコマンドを実行して、その結果を取得できるようにする。

#### 受け入れ条件

1. WHEN `run_command`メソッドが有効なenv_idとコマンド配列で呼び出される THEN Coherraサーバー SHALL 指定された環境内でコマンドを実行する
2. WHILE コマンドが実行中 THE Coherraサーバー SHALL stdoutとstderrの出力を最大64KBまでリングバッファに保存する
3. IF タイムアウト値が指定される THEN Coherraサーバー SHALL 指定された時間後にコマンドを強制終了する
4. WHEN コマンドが完了またはタイムアウトする THEN Coherraサーバー SHALL exit code、stdout_tail、stderr_tail、実行時間、タイムアウトの有無を返す
5. IF 指定されたenv_idが存在しない THEN Coherraサーバー SHALL 適切なエラーメッセージを返す
6. WHEN 追加の環境変数が提供される THEN Coherraサーバー SHALL それらをコマンド実行時の環境に適用する
7. IF タイムアウトが指定されない THEN Coherraサーバー SHALL デフォルトの120秒（2分）タイムアウトを適用する

### 要件4: 状態管理とリソース管理
**目的:** システム管理者として、アプリケーションが適切にリソースを管理し、クリーンアップされることを保証する。

#### 受け入れ条件

1. WHILE Coherraサーバーが稼働中 THE サーバー SHALL アクティブな環境のレジストリを`tokio::sync::RwLock<HashMap>`で管理する
2. WHEN Ctrl+Cまたはシステムシャットダウンシグナルが受信される THEN Coherraサーバー SHALL すべてのアクティブなコンテナを停止し削除する
3. WHEN シャットダウン処理が開始される THEN Coherraサーバー SHALL 実行中のコマンドを適切にタイムアウトまたは終了させる
4. WHERE ログ出力が64KBを超える場合 THE Coherraサーバー SHALL 最新の出力のみを保持し古い出力を破棄する
5. WHEN メモリリソースが制限に達する THEN Coherraサーバー SHALL リングバッファ制限を適用して無制限の成長を防ぐ

### 要件5: エラー処理とロギング
**目的:** 開発者として、問題が発生した際に適切なエラーメッセージとログを受け取って、デバッグと問題解決ができるようにする。

#### 受け入れ条件

1. WHEN Podmanデーモンが起動していない THEN Coherraサーバー SHALL 明確なエラーメッセージと起動手順をログに記録する
2. WHEN 内部エラーが発生する THEN Coherraサーバー SHALL anyhowベースのエラーをMCPエラーコードに変換して返す
3. WHILE 環境操作が実行される間 THE Coherraサーバー SHALL tracingフレームワークを使用して環境生成、実行開始、終了をログに記録する
4. WHERE 長時間実行コマンドがログを生成する場合 THE Coherraサーバー SHALL リングバッファ上限（64KB）を適用する
5. WHEN 複数の同時リクエストが処理される THEN Coherraサーバー SHALL RwLockを使用してレジストリへの安全なアクセスを保証する

### 要件6: プラットフォーム互換性
**目的:** 開発者として、WindowsとLinux/macOSの両方でCoherraサーバーを利用できるようにする。

#### 受け入れ条件

1. IF 実行環境がWindowsの場合 THEN Coherraサーバー SHALL Podman serviceモードで動作する
2. WHERE Windows環境で実行される場合 THE Coherraサーバー SHALL CRLF設定を適切に処理する
3. WHEN プロセス管理が必要な場合 AND 環境がWindowsの場合 THEN Coherraサーバー SHALL 適切なプロセス管理メカニズムを使用する（将来的にはJob Object）
4. IF 実行環境がLinux/macOSの場合 THEN Coherraサーバー SHALL Podmanのネイティブソケット通信を使用する