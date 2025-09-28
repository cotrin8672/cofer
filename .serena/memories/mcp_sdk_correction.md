# MCP SDK 修正記録

## 日付: 2025-01-28

## 変更内容
- **削除**: rust-mcp-sdk v0.7.0（非公式実装）
- **追加**: rmcp v0.7.0（公式実装）

## rmcp (公式SDK)について
- **GitHub**: modelcontextprotocol/rust-sdk
- **バージョン**: 0.7.0
- **特徴**:
  - Model Context Protocolの公式Rust実装
  - サーバー/クライアント両対応
  - マクロによるボイラープレート削減
  - 高パフォーマンス（4,700+ QPS）

## 使用方法
```toml
rmcp = { version = "0.7.0", features = ["server"] }
```

## 注意事項
- Rust Edition 2024が必要な可能性（要確認）
- nightlyコンパイラが必要な場合がある

## 選定理由
1. 公式実装であり、プロトコル仕様への準拠が保証される
2. 活発な開発とコミュニティサポート
3. 実績のあるパフォーマンス