# Coherra Project Overview

## プロジェクトの目的
Coherra は既存の container-use 実装をRustで置き換えるプロジェクトです。Podmanとbind-mountを使用して、より効率的で高速なコンテナ環境管理ツールを実現します。

## 主要な技術的目標
- **ゼロコピー化**: Export/Import処理を撤廃し、bind-mountで直接ファイルシステムを共有
- **高速化**: 1ファイル変更→commit を120ms以下で実現
- **安定性向上**: すべての外部実行にtimeout設定、ハングゼロを達成
- **メモリ効率**: リングバッファによるログ管理で上限を設定

## 現行システムの問題点（解決対象）
1. 全量Export (Wipe:true) によるI/O爆発
2. Git実行時の全量バッファリングによるメモリ圧とハング
3. 1秒ポーリングによるCPU無駄
4. サブモジュール.gitdirの毎回再生成
5. ポート公開の非効率な直列処理

## 新アーキテクチャの特徴
- Podman APIを使用したコンテナ制御
- gix (gitoxide) + git2 (libgit2) によるGit操作
- notify crateによるファイルシステム監視
- tokioベースの非同期処理
- Windows対応（Job Object、長パス対応）