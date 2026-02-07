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

## 運用ルール（Workflow Policy）

### 1. 実装順序（必須）
- `spec/test-ledger.md` に対象T項目がない要件は実装しない
- 変更は `test -> impl -> doc` の順で行う
- 仕様変更時は同一変更セットで `spec/` も更新する

### 2. 変更単位
- 1コミット1目的を原則とする
- 推奨粒度:
  - `docs(spec)` 設計更新
  - `feat/fix(core|app|batch)` 実装
  - `test(...)` テスト追加
- 無関係な既存差分を混ぜない

### 3. xTranslator互換方針
- 優先ワークフロー:
  1. 辞書ベース自動翻訳
  2. XML一括翻訳
- 差異は `spec/xtranslator-gap.md` に記録し、未解消理由を残す
- UI外観の完全一致より、操作導線と結果互換を優先する

### 4. 保存と安全性
- 上書き保存時はバックアップを作成する
- 保存失敗時は対象パスと原因を表示する
- `finalize`（バッチ含む）前に入力ファイル存在確認を行う

### 5. バッチ運用
- 標準入口は `xt_batch` とする
- サポート対象:
  - `--load/--load-strings/--load-plugin`
  - `--importxml`
  - `--finalize`
  - `--generate-dictionary`, `--dict-in`, `--dict-out`
- 追加オプションは後方互換を壊さない

### 6. テスト運用
- マージ前に最低実行:
  - `cargo test`
  - 変更がバッチのみなら `cargo test -p xt_app --bin xt_batch`
- 不具合修正は再発防止テストを必須とする

### 7. リリース判定
- 受け入れ基準は `spec/xtranslator-gap.md` の項目で判断する
- `updated/unchanged/missing` など結果統計を確認可能であること
