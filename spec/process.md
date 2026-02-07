# 運用

## Codex CLI 運用ルール
### 毎回プロンプトの先頭に入れる（固定）
- 「テスト台帳（T-*）にチェックがない要件は実装禁止」
- 「まずテスト追加してRED確認、次に実装でGREEN」

### コミット粒度
- tests commit（RED確認）
- impl commit（GREEN）
- refactor/doc（任意）

## リリース/配布
- AppImage / deb / rpm は `dx bundle` を利用（v1以降）
- AppImage更新は基本「差し替え」運用
- LinuxはWebView依存のため、必要な依存関係をREADMEに明記

## リスクと対策（簡易）
- 大量行UI: 仮想化とプリフェッチキャッシュが必須
- 形式解析: fixture戦略（golden/生成）を早期に固める
- 依存関係: xt_coreは最小依存に抑える（UI依存禁止）
