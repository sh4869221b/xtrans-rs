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

## バッチ運用（xt_batch）
- 実行バイナリ: `cargo run -p xt_app --bin xt_batch -- ...`

### XMLベース（既存）
- `cargo run -p xt_app --bin xt_batch -- --load base.xml --importxml tr.xml --finalize out.xml`

### Stringsベース
- `cargo run -p xt_app --bin xt_batch -- --load-strings Data/Strings/mod_english.strings --importxml tr.xml --finalize Data/Strings/mod_japanese.strings`

### Pluginベース（ESP/ESM/ESL）
- `cargo run -p xt_app --bin xt_batch -- --load-plugin Data/mod.esp --workspace-root /path/to/game --importxml tr.xml --finalize out/mod.esp`

### 辞書生成/保存
- 生成: `cargo run -p xt_app --bin xt_batch -- --generate-dictionary Data/Strings/Translations --source english --target japanese --dict-out dict.tsv`
- 適用: `cargo run -p xt_app --bin xt_batch -- --load base.xml --importxml tr.xml --dict-in dict.tsv --finalize out.xml`
