# 1．Container-use の実装詳細と問題点（事実ベース）

## 1.1 環境（Environment）とコンテナの関連

* `Environment` は Dagger の `Container` を保持し、`Workdir()` はコンテナ内の作業ディレクトリ（既定 `/workdir`）を `Directory` として返す。
* 新規作成時はベースイメージから `buildBase` → `apply`（`Container.Sync`→`ID` 取得）で状態を固定。

## 1.2 「Sync（Export）」の同期方法（コンテナ→ホスト）

* **core 手順**：

  1. `.git` を**gitfile**として作成し、`gitdir: <forkRepo>/worktrees/<envID>` を書く。
  2. サブモジュール配下にも `.git`（gitfile）を作成。
  3. `env.Workdir()` を **`Export(Wipe:true)`** で **ホスト側 worktree に全消し→全コピー**。
     実装：`exportEnvironment()`。
* サブモジュールの `.git` は `gitdir:` の中身を読み出して複製する。

## 1.3 「Propagate（ホスト→Git）」の反映

* `propagateToWorktree` → `exportEnvironment` 後、`propagateToGit` を実行。
* `propagateToGit` の処理：

  1. `commitWorktreeChanges`（`git status`→必要ファイル `add`→`commit`）。
  2. `saveState`（コンテナ状態を **git-notes** に保存）。
  3. `fetch container-use <envID>`（ユーザリポに取り込み）。
* ステージング対象の判定（非バイナリ優先など）はテストで検証されている。

## 1.4 Git 実行の仕組み（ホスト側）

* すべて外部 `git` CLI を **`exec.CommandContext(...).CombinedOutput()`** で実行し、標準出力・標準エラーを**全量バッファ**に取り込む設計。

## 1.5 ログ・ノート・コマンド実行の扱い

* コンテナコマンド後、`ExitCode` `Stdout` `Stderr` を **都度全取得**し、メモリ上のノートに追記。
* バックグラウンド起動（サービス）では **ポートごとに**トンネルを開始し、各ポートで `tunnel.Endpoint()` を取得する。

## 1.6 監視（watch）コマンド

* Unix 版：1 秒ごとに `git log --color=always --remotes=container-use --oneline --graph --decorate` を実行。
* Windows 版：同様に **1 秒間隔**で `git log` を呼び続け、都度全描画。  

## 1.7 排他制御（ロック）

* `TryRLockContext` など **100ms ポーリング**型で再試行する軽量ロック。高争奪時は CPU 無駄が増える。

---

## 1.8 問題点（性能・安定性・DX上の致命点）

### (A) I/O と同期

* **全量 Export（Wipe:true）**：差分が小さくても**毎回ツリー全コピー**。大きなリポ／生成物があるとレイテンシが線形増。
* サブモジュール `.gitdir` 読み取り→複製が**毎回**走る。

### (B) Git 実行の設計

* `CombinedOutput` により**巨大出力を全量メモリ**滞留。タイムアウトの明示もなく、対話プロンプト発生時に**ハング**。
* `watch` は**1 秒ポーリング**で外部 `git` を連打（Windows でも同様）。 

### (C) ログ・ノートの扱い

* コンテナ実行ごとに `Stdout/Stderr` を全文取込→ノートへ蓄積。**ログ上限・サマリ無し**でメモリ圧に。

### (D) サービス公開・ネットワーク

* **ポートごとに**トンネル Start→Endpoint 取得を直列で繰り返すため、N ポートで N 回の往復。障害やFW/WSL 問題時に**個別タイムアウトが弱い**。

### (E) 排他・並列化

* 100ms ティッカーのロック再試行は**混雑時のスピン**に近く、CPU とスループットの悪化因子。

---

# 2．リプレイス計画（Rust × Podman／bind-mount）と問題点の解消方法

> 目標：**軽量・高速・非対話・ハングゼロ**。
> 戦略：**Sync を捨てる**（Export/Import をやめる）→ **bind-mount でゼロコピー**／**Git はライブラリ主体（非対話・ストリーム）**。

## 2.1 全体アーキテクチャ（新）

```
[Host]
 ├─ Bare repo (~/.config/yourtool/repos/project)
 ├─ Worktree (~/.config/yourtool/worktrees/<env-id>)
 │    └─ .git (gitfile: gitdir -> repos/.../worktrees/<env-id>)
 └─ YourTool (Rust CLI / daemon)
      ├─ Podman API (Docker互換)  … コンテナ制御
      ├─ Git (gix + git2)        … 差分→index→commit、notes
      ├─ Watcher (notify)        … FSイベント→バッチコミット
      └─ Runner (tokio)          … 外部cmdは逐次読取+timeout+JobObject

[Container]
 └─ /workdir  ← bind-mount →  Host worktree（ゼロコピー）
```

### キー設計の差分

* **旧**：`Export(Wipe:true)` で全量コピー → その後 Git。
* **新**：**bind-mount** により **コンテナの変更＝即ホストの worktree**（コピー不要）。
  → Export コストと `.gitdir` 再生成ループを撤廃（サブモジュールの gitfile は初期化時に一度だけ）。

## 2.2 技術スタック（Rust / Podman）

* **Podman 制御**：`bollard`（Docker API 互換） or `reqwest` で Podman REST

  * `podman system service --time=0` を前提に `run/stop/exec/logs` を操作。
  * コンテナ起動時：`-v <host_worktree>:/workdir[:Z]`（SELinux環境は `:Z`）。

* **Git**：**gix（gitoxide）＋ git2（libgit2）併用**

  * 差分抽出／index 操作／コミット → **gix**（高速・ノンブロッキング志向）
  * **git-notes** など完全 API が欲しい箇所 → **git2**
  * 認証はコールバックを**非対話**固定（失敗は即エラー）。
  * 初期化時に `core.autocrlf` と `core.filemode=false` を設定（Windows 揺れ抑止）。

* **Watcher**：`notify`（デバウンス 100–200ms）、`fs::exclude` リスト（`.git/`, `node_modules/`, `target/` …）。

* **非同期／I/O**：`tokio`／`bytes`／`futures`、ログはリングバッファ（`arraydeque` 等）で**上限**管理。

* **Windows**：`windows` crate で **Job Object** によるプロセスツリー Kill、長パス（`\\?\`）対応。

## 2.3 機能仕様（必須要件）

### 2.3.1 環境作成

* 入力：`project_root`、`env_id`、`image`、`workdir="/workdir"`、`mount=worktree_path`
* 処理：

  1. ホスト bare repo と **worktree** 構成（`.git` gitfile を生成し `gitdir` を設定）。
  2. サブモジュールの `.git` は **一度だけ** gitfile 化（旧実装の毎回処理を撤廃）。
  3. Podman でコンテナ起動：`-v worktree_path:/workdir`。
* 出力：`ContainerHandle`（id, name, mounts, ports）。

### 2.3.2 コマンド実行（コンテナ内）

* 入力：`cmd[]`, `env[]`, `timeout`
* 処理：`exec` を **tokio** で起動し、stdout/stderr を**逐次**読取（行上限 N／合計バイト上限 M）。
* 失敗時：**明示的 timeout**、Job Object／`SIGKILL`（Unix）で**確実に回収**。
* 出力：`exit_code`、`stdout_tail`、`stderr_tail`、`duration_ms`。

### 2.3.3 自動コミット（ファイル監視）

* 入力：`debounce_ms`、`exclude[]`、`nonbinary_only: true`
* 処理：

  1. `notify` で /workdir（＝worktree）を監視。
  2. バーストを `debounce_ms` でまとめ、**単発のバッチ**で `status`（gix）→`add`（非バイナリのみ）→`commit`。
  3. コミットメッセージには操作の要約（例：「Run: npm install」「Edit: src/app.ts」）。
* 出力：`commit_oid`（なければ `None`）。

### 2.3.4 ノート（git-notes）

* 入力：`ref="refs/notes/yourtool"`、`payload(bytes)`、`cap_lines`
* 処理：`git2::note_create` で追記。巨大ログは**先頭/末尾 N 行**のみ保持。
* 出力：`note_oid`。

### 2.3.5 ネットワーク／サービス

* 入力：`ports[]`、`entrypoint/use_entrypoint`
* 処理：コンテナRun時に**一括**でポート公開設定。
* 出力：`host_mappings`（失敗時は**内部ポートのみ**で継続するフォールバック）。

## 2.4 非機能要件（必達）

* **性能**：

  * 1ファイル更新→commit：**≤120ms（NVMe, 中規模リポ）**
  * 1,000ファイル変化→commit：**≤2s**（差分のみ／index 更新最適化前提）
* **安定性**：

  * すべての外部実行に `timeout`。timeout 超過時は**必ず**プロセスツリー kill。
  * 認証・pager・色・TTY は**常時非対話**設定（環境変数またはライブラリ設定）。
* **メモリ**：

  * 標準出力／標準エラーは**リングバッファ**で上限（例：各 64KB or 512 行）。
* **Windows**：

  * `core.autocrlf` と `.gitattributes` を初期化時に強制。
  * Job Object で子プロセスリーク 0 件。

## 2.5 現行問題へのマッピング（解消策の対応表）

| 現行の問題                                      | 根拠                                        | 新設計での解消                                                                   |
| ----------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------- |
| **全量 Export (Wipe:true)** による I/O 爆増     | `Export(..., Wipe:true)` で完全コピー。     | **bind-mount** でゼロコピー。Export 自体を撤廃                                   |
| Submodule `.gitdir` 毎回読込・生成              | サブモジュール gitfile を export 毎に生成。 | 初期化時に**一度だけ**生成し、以後は放置                                         |
| `CombinedOutput` 全量バッファでハング・メモリ圧 | 外部 `git` を全量取得。                     | `tokio::process` で**逐次読取 + timeout**／**可能な限り gix/git2**               |
| `watch` が 1 秒ポーリングで `git log`           | 連続外部実行。                              | **notify** の FS イベント駆動、`status`→差分のみ commit                          |
| ポートごとにトンネル Start/Endpoint             | 直列で N 回往復。                           | Podman 側で**一括公開**／失敗時の**フォールバック**実装                          |
| ロックが 100ms ポーリング                       | スピン風の再試行。                          | そもそも Git/FS 操作を短時間化＋**OS ネイティブ待機**に近づける（通知/キュー化） |
| コンテナ実行ログを全文ノート保存                | 全取得→ノート追記。                         | **リングバッファ**で tail のみ保存、ノートは要約                                 |

## 2.6 API 仕様（Vibe Coding でのプロンプト入力に最適化）

> すべて JSON-RPC/CLI いずれでも同じ入出力を返す。例は CLI 形。

### 2.6.1 `init`

* 入力：

  ```json
  {
    "project_root": "/path/to/repo",
    "env_id": "adverb-animal",
    "image": "ghcr.io/org/dev:latest",
    "workdir": "/workdir",
    "mount": "/home/user/.yourtool/worktrees/adverb-animal",
    "submodules": true
  }
  ```
* 出力：`{"worktree":"...","container_id":"...","gitdir":"..."}`
* 事後条件：`<worktree>/.git` は `gitdir: .../worktrees/<env-id>`（gitfile）。

### 2.6.2 `run`

* 入力：`{"cmd":["bash","-lc","npm ci && npm test"],"timeout_ms":600000,"env":{"CI":"1"}}`
* 出力：`{"exit":0,"stdout_tail":"...","stderr_tail":"...","ms":12345}`
* 例外：`{"error":"timeout","killed":true}`（Job Object / SIGKILL 実施済み）。

### 2.6.3 `watch-commit`

* 入力：`{"debounce_ms":150,"nonbinary_only":true,"exclude":[".git/","node_modules/"]}`
* 出力：`{"commit":"<oid>","changes":123}` or `{"commit":null}`
* 動作：notify でイベント→バッチ `status -z`（gix）→非バイナリ `add`→`commit`。

### 2.6.4 `note-append`

* 入力：`{"ref":"refs/notes/yourtool","cap_lines":120,"payload":"<base64>"}`
* 出力：`{"note":"<oid>"}`
* ルール：テキストログは先頭/末尾 `cap_lines` のみ保存。

### 2.6.5 `up` / `down`

* `up`：`{"ports":[3000,9229],"entrypoint":null}` → 一括公開、失敗時は内部のみで継続。
* `down`：コンテナ停止／後始末。

## 2.7 エラー／タイムアウト方針

* すべての外部呼び出しに `timeout_ms`。既定：`run=600_000`, `git=30_000`, `startup=30_000`。
* timeout 超過時：**コンテナ exec kill**、**子プロセスは Job Object / SIGKILL**。
* 認証や pager が必要な状況：**禁止**（非対話設定で即エラー化）し、**解決ヒント**を返す。

## 2.8 セキュリティ／ポリシー

* コンテナは **rootless** 実行推奨。
* 秘密情報は**環境変数の値をログに出さない**（ノート要約前にマスク）。
* `git` 認証は agent 経由／トークン固定。**コンテナ内には秘密鍵を置かない**。

## 2.9 マイグレーション計画（最短 3 ステップ）

1. **bind-mount 移行**：現行の Export 経路を止め、worktree を直接 `/workdir` にマウント。
2. **差分コミット置換**：`watch` を notify ベースに変更、gix で `status/add/commit`。
3. **後方互換**：git-notes の ref をそのまま使用（`container-use` 相当 → `yourtool` に可変設定）。

---

## 2.10 成果物の受け入れ基準（Acceptance Criteria）

* 小変更（1 ファイル編集）→ `watch-commit` のレスポンス **≤120ms**（平均、p95 ≤180ms）。
* 1,000 ファイル変更 → `commit` 完了 **≤2s**（NVMe, リリースビルド）。
* どの操作も **ハングしない**（全 API に timeout 実装／kill 完了をテレメトリで確認）。
* Windows：CRLF 設定を固定し、**偽差分（改行／実行ビット）ゼロ**。
* ノート：巨大ログを投入してもプロセスメモリ消費が**頭打ち**（リングバッファで上限）。

---

### 付記：現行設計との差異の要点（エージェントへのコンテキストに重要）

* **「コンテナ→ホストの Export」フェーズを完全撤廃**（ゼロコピー化）。
* **外部 `git` は極力使わず**、使う場合も**逐次読取＋timeout**で**全量バッファ禁止**。
* **watch はポーリング廃止**、FS イベントで差分コミット。 
* **ポート公開は一括**（直列回数を N→1 に）。

---