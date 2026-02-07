# Tests

## TDD契約（必須）
### 実装禁止ルール
- **テストが存在しないFRの実装は禁止**
- 各サブ機能は必ず以下の順:
  1) テスト追加（RED確認）
  2) 実装（GREEN）
  3) リファクタ（任意、常にGREEN）

### テスト条件
- ネットワーク禁止
- 時刻依存を排除（必要なら `Clock` 注入）
- fixtureは小さく、決定的、同梱可能な形にする

## テスト戦略
### 種別
- Unit: validation/tm/diff/正規化
- Integration: formats read/write round-trip、xml import/export
- Golden: 小さな実ファイルで入出力固定化
- Perf-ish: 10万seedでの簡易測定（CIでは緩め）

### fixture方針
- 可能なら対象クレート配下の `tests/fixtures/` に同梱（例: `crates/xt_core/tests/fixtures/`）
- 難しい場合はテスト内生成（builder）で最小バイナリ生成
- いずれも **決定的** であること
