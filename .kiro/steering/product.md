# Product Overview

## Product Description
Coherra (通称 Cofer) は、既存の container-use 実装を Rust 製の高性能コンテナ環境管理ツールに置き換えるプロジェクトです。Podman と bind-mount を活用することで、ホストとコンテナ間のファイル同期をゼロコピー化し、Git 操作もライブラリ主体で最適化することを目指しています。現在はローカル Git リポジトリを Cofer ワークフロー向けに初期化する CLI を中心に据え、コンテナ実行レイヤーとの統合に向けた基盤を整備しています。

## Core Features
### 現在提供している機能
- **`cofer init` コマンド**: 既存リポジトリを検証し、`.cofer` 配下にベアリポジトリを生成して `cofer` リモートを追加。
- **Git ヘルパー群**: `fetch_from_cofer`、`create_branch`、`create_worktree_from_cofer` を備え、Cofer 専用ブランチやワークツリーの操作を内製 Git API で完結できる土台を用意。
- **`.gitignore` 保護**: `ensure_gitignore_has_cofer` により `.cofer/` が常に無視されるよう保証し、ホスト側に生成するメタデータを誤ってコミットしない仕組みを提供。

### 直近ロードマップ
- **Podman 連携**: `bollard` を介したコンテナ起動・ポート公開の一括制御。
- **ゼロコピー作業ツリー**: bind-mount と `notify` を用いたファイルイベント駆動の自動コミット機構。
- **MCP サーバ統合**: `rmcp` ベースでの Model Context Protocol 対応により、AI コーディングエージェントとのネイティブ連携を実現。

## Target Use Case
- コンテナ化された開発環境を高速に初期化・管理したいチーム。
- AI アシスタントや CLI からの Git ワークツリー制御を自動化したい開発者。
- 大規模リポジトリでの頻繁なファイル更新を効率良く取り込む必要がある CI/CD パイプライン。

## Key Value Proposition
### Performance (目標指標)
- 単一ファイル変更から commit 完了まで **≤120ms** (NVMe, 中規模リポ想定)。
- 1,000 ファイルバッチでも **≤2s** 以内でコミット処理を完了。
- bind-mount により Export/Import を排し、I/O コストを極小化。

### Reliability
- すべての外部操作に明示的なタイムアウトを適用しハングを防止 (run: 600s, git: 30s, startup: 30s)。
- トレース可能なログとエラー文脈を提供し、障害解析を容易に。

### Efficiency
- ファイル監視をポーリングからイベント駆動に切り替え、CPU の無駄なスピンを削減。
- ログはリングバッファで制限し、常に一定量で保持。
- Git 操作は gix/git2 を優先し、外部 CLI 呼び出しを最小限に抑制。

## Development Status
- **実装済み**: CLI スケルトン (`clap`)、`tokio` ランタイム、Git 初期化・ヘルパー群、`tracing` ベースのロギング。
- **着手中**: `notify` を活用した FS 監視と自動コミットフロー、コンテナ管理のための Podman API 層。
- **今後の注力ポイント**: MCP サーバの公開インターフェース整備、Windows 専用 Job Object を用いたプロセスツリー管理、性能測定用ベンチマーク (criterion) の導入。
