# xTranslator 参照差異記録（辞書自動翻訳 / XML一括翻訳）

## 目的
- ユーザーの主ワークフローである以下2点を実現するため、xTranslator仕様を参照基準として差異を記録する。
- ワークフローA: 辞書を使用した自動翻訳
- ワークフローB: 翻訳用XMLを使用した一括翻訳

## 参照元（2026-02-07確認）
- リポジトリ: `https://github.com/MGuffin/xTranslator`
- 直接リンク:
  - README: `https://github.com/MGuffin/xTranslator/blob/main/README.md`
  - XML direct import: `https://github.com/MGuffin/xTranslator/blob/main/TESVT_main.pas#L1095`
  - import/autotranslate option mode: `https://github.com/MGuffin/xTranslator/blob/main/TESVT_main.pas#L8600`
  - SST apply core: `https://github.com/MGuffin/xTranslator/blob/main/TESVT_MainLoader.pas#L2071`
  - batch importxml/finalize: `https://github.com/MGuffin/xTranslator/blob/main/TESVT_main.pas#L13263`
- README:
  - 辞書構築: `README.md:26`（`Build dictionaries...`）
  - XML import/export: `README.md:44`
  - Quick AutoTranslate: `README.md:52`（`Ctrl-R`）
  - XMLドラッグ&ドロップ適用: `README.md:186`
  - BatchProcessor（`language selection, sstImport, xmlImport, finalization`）: `README.md:200`
- 実装コード（参照のためローカル取得）:
  - XML直接取込: `TESVT_main.pas:1095`
  - XML取込処理呼び出し: `TESVT_main.pas:1114`
  - import/autotranslate共通オプション: `TESVT_main.pas:8600`
  - SST辞書適用処理: `TESVT_MainLoader.pas:2071`
  - Batch `importxml/finalize/generatedictionaries`: `TESVT_main.pas:13348`, `TESVT_main.pas:13374`, `TESVT_main.pas:13394`

## xTranslatorで確認できた仕様（対象ワークフロー）

### A. 辞書を使用した自動翻訳
- 文字列ペア辞書の構築機能を持つ（READMEベース）。
- `Ctrl-R` で選択行に対するQuick AutoTranslateが可能（READMEベース）。
- 辞書適用はSST/Vocabを読み込み、照合関数で各チャネルに反映する実装を持つ（`doApplySst`）。
- import/autotranslateは比較・適用モードを共通オプションで切り替える設計になっている。

### B. 翻訳用XMLを使用した一括翻訳
- XML import/exportを標準機能として持つ（READMEベース）。
- XMLを読み込み中データへ適用する直接導線（`importXMLDirect -> XMLImportbase`）を持つ。
- 読み込み済みESP/ESMにXMLをドラッグ&ドロップして既定オプションで適用可能（READMEの更新履歴）。
- BatchProcessorで `importxml` と `finalize` による一括処理を実行可能。

## 現設計との差異（xtrans-rs）

### 実装済み/部分一致
- XML import/exportの概念は存在（`spec/requirements.md` の `FR-XML-01`）。
- Strings/Pluginを読み込んで編集する基本経路は存在。
- xTranslator XML（`SSTXMLRessources`）の `Source/Dest` 取り込みを実装済み（`FR-XML-04`）。
- XML一括適用で `key一致優先 + source一致（一意）フォールバック` を実装済み（`FR-XML-05`）。
- `source一致` が競合する場合の安全側スキップ（missing）を実装済み（`FR-XML-06`）。
- 対応テストを `xt_core` に実装済み（`T-XML-IMPORT-002`, `T-XML-APPLY-002`, `T-XML-APPLY-003`）。
- 辞書設定（`source/target/root`）の永続化と再起動復元を実装済み（`FR-DICT-03`）。
- Quick AutoTranslate をメニューおよび `Ctrl-R` で実行可能（`FR-AUTO-03`）。
- XML一括適用のファイル導線（メニュー起点 + ドロップ適用）を実装済み（`FR-XML-07`）。

### 未充足（優先度高）
- 辞書運用は `Data/Strings/Translations` 前提だが、複数辞書セット切替（ゲーム別/プロジェクト別）は未実装。
- Quick AutoTranslate の適用範囲は「選択行のみ」で固定。xTranslatorの複数選択/一覧適用に相当する拡張は未実装。
- Batch相当の非対話処理は動作するが、xTranslator同等の最終化オプション群（運用フラグ網羅）は未完。

## 目標仕様（本プロジェクト）

### 目標A: 辞書自動翻訳ワークフロー
1. `言語ペア` と `辞書ソース`（`Data/Strings/Translations` を含む）を設定
2. `辞書を構築` を実行
3. エントリ選択またはフィルタ結果に対し `Quick AutoTranslate` を実行
4. 差分確認後に保存（Strings/Plugin）

### 目標B: XML一括翻訳ワークフロー
1. 対象Plugin/Stringsをロード
2. 翻訳XMLを指定して `Import XML (default profile)` を実行
3. 反映件数・未反映件数・エラー件数を表示
4. `Save` で上書きまたは別名出力

## 受け入れ基準（差異解消判定）
- `A-1` 辞書構築後、Quick AutoTranslateで選択範囲の訳文が更新される。
- `A-2` 辞書未構築時は実行不可かつ理由が表示される。
- `B-1` XML一括適用で更新件数がステータス表示される。
- `B-2` XML適用後の保存で上書き確認ダイアログとバックアップ方針が表示される。
- `B-3` 非対話（将来Batch）でも `load -> importxml -> finalize` と同等の結果を得られる。

## 補足
- ここでの「差異」は機能互換の差異を指し、UI外観の完全一致は必須条件にしない。
- ただし操作導線は、既存ユーザーの移行負荷を下げるため xTranslator に寄せる。
