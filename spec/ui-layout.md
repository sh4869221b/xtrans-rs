# UI Layout（現実装準拠）

## 対象
- 実装ファイル: `crates/xt_app/src/main.rs`
- スタイル: `crates/xt_app/assets/main.css`
- 対応バージョン: 現行 `main`
- 仕様基準: xTranslator確認仕様（ゲーム選択、辞書構築、自動翻訳、MCM/Strings、保存）を正とする

## 1. 画面全体構成
- ルート: `#app`
- レイアウト: 2ペイン
- 左ペイン: Entry一覧（検索 + 仮想リスト）
- 右ペイン: 詳細編集 + ツール群

### レスポンシブ
- デスクトップ: 横並び（左 38% / 右 可変）
- モバイル（`max-width: 900px`）: 縦積み

## 2. 左ペイン（一覧）

### 領域
- 見出し: `Entries`
- 検索入力: `#search`
- 一覧領域: `#entry-list`

### 一覧表示仕様
- 要素種別: `EntryView`（`key`, `source_text`, `target_text`, `is_selected`）
- 描画: 仮想リスト（`virtual_window` + 上下 `Spacer`）
- 行高: `64px`（CSSと計算値を一致）
- 選択時: `selected` クラス付与

### 操作
- 検索入力:
  - `state.set_query(...)`
  - `scroll_offset = 0` へリセット
- エントリ選択:
  - `state.select(key)`
  - 右ペイン編集値 `edit_source` / `edit_target` に同期
- スクロール:
  - `scroll_top` から `scroll_offset` / `viewport_height` 更新

## 3. 右ペイン（詳細）

### 領域
- 見出し: `Detail`
- 選択中エントリ情報:
  - `Key`
  - `Source` テキストエリア
  - `Target` テキストエリア
- アクション:
  - `Apply Edit`
  - `Validate`
  - `Diff Check`
  - `Encoding`
- 結果表示:
  - Validation Issues
  - Diff status
  - Encoding status

### 状態遷移（詳細）
- 未選択時:
  - `"Select an entry from the list."` を表示
- `Apply Edit`:
  - `state.update_entry(...)` 成功時に `history.apply(...)`
- `Validate`:
  - placeholder / printf / alias を実行し `validation_issues` に反映
- `Diff Check`:
  - `DiffEntry` 生成後 `update_source(...)` を適用し `diff_status` 更新
- `Encoding`:
  - Latin1 encode/decode の結果を `encoding_status` に格納

## 4. 右ペイン（Tools）

### 構成
- 見出し: `Tools`
- ボタン群:
  - `Export XML`
  - `Import XML`
  - `Undo`
  - `Redo`
- ローダー群:
  - `Load XML`
  - `Load Strings`
  - `Load Plugin (ESP/ESM/ESL)`
  - `Build Hybrid`
- XML編集欄: `.xml-textarea`
- 結果表示:
  - `file_status`
  - `xml_error`
  - `hybrid_preview`
  - `hybrid_error`

### I/O 操作仕様
- Export XML:
  - `state.entries()` をXML化し `xml_text` に出力
- Import XML:
  - `xml_text` を取り込み、成功時は `history` と `state` を同期更新
- Undo/Redo:
  - `UndoStack` で巻き戻し/やり直し
- Load XML:
  - 読み込み成功で `xml_text` 更新
- Load Strings:
  - 拡張子別に `read_strings/read_dlstrings/read_ilstrings`
  - 成功時に Entry へ変換して `history/state` 更新
- Load Plugin:
  - `.xtplugin`: テキストパーサで読込
  - `.esp/.esm/.esl`: `extract_esp_strings(...)`
  - 失敗時は `extract_null_terminated_utf8(...)` にフォールバック
  - `workspace_root` はパス推定で決定
- Build Hybrid:
  - `loaded_plugin + loaded_strings` の両方がある場合にプレビュー生成

## 5. 主要UI状態（Signal）
- 履歴/一覧:
  - `history`, `state`
- スクロール:
  - `scroll_offset`, `viewport_height`
- 詳細編集:
  - `edit_source`, `edit_target`
- XML:
  - `xml_text`, `xml_error`
- 品質チェック:
  - `validation_issues`, `diff_status`, `encoding_status`
- 形式読込:
  - `loaded_plugin`, `loaded_strings`, `loaded_esp_strings`, `file_status`
- Hybrid:
  - `hybrid_preview`, `hybrid_error`

## 6. 表示ポリシー
- エラー表示:
  - XML/Plugin/Strings の読込エラーは `file_status` または `xml_error`
- 成功表示:
  - 読込成功時は短いステータス文字列を表示
- 無効状態:
  - `Apply Edit` は選択行なしで `disabled`

## 7. 既知の制約（現実装）
- 画面コンポーネントが単一ファイル（`main.rs`）に集中している
- アクションハンドラがUI層に集約され、責務分離が浅い
- `workspace_root` はUI側推定で、明示設定UIは未実装

## 8. 今後の分割方針（設計メモ）
- `components/entry_list.rs`
- `components/detail_panel.rs`
- `components/tools_panel.rs`
- `state/app_state.rs`
- `actions/*`（I/O, validation, history）

## 9. 外部チュートリアルとの対応（Skyrim mod翻訳）
- 参照: `https://tktk1.net/skyrim/tutorial/translatemod/`
- 目的: 実装中レイアウトが、実ユーザーの作業フローをどこまでカバーしているかを明示する

### フロー対応表
- `SE版の翻訳下準備`:
  - 現状対応: 部分対応
  - 根拠: `Load Strings` はあるが、`Data/Strings/Translations` 生成支援UIは未実装
- `辞書の作成（english -> japanese）`:
  - 現状対応: 未対応
  - 根拠: 辞書ビルド画面、言語ペア設定、辞書状態表示がない
- `modの翻訳（ESP/ESMを開く）`:
  - 現状対応: 対応
  - 根拠: `Load Plugin (ESP/ESM/ESL)` と一覧/詳細編集を提供
- `自動翻訳`:
  - 現状対応: 未対応
  - 根拠: 辞書適用アクション、候補一括適用、未翻訳のみ対象などがない
- `MCM翻訳（strings系ファイル編集）`:
  - 現状対応: 対応
  - 根拠: `Load Strings` で `.strings/.dlstrings/.ilstrings` を読み込み編集可能
- `保存/書き戻し`:
  - 現状対応: 部分対応
  - 根拠: `Export XML` はあるが、`strings`/`esp` への直接保存UIは未実装

## 10. レイアウト要件ギャップ（優先度順）
- P0:
  - `Save Strings` / `Save Plugin` ボタンと保存先表示（読み込みフォーマットへ戻す）
  - 失敗時のエラー詳細（ファイル名、オフセット、レコード識別子）表示領域
- P1:
  - `Game Profile` セクション（Skyrim/FO4切替、Data/Strings ルート選択）
  - `Translations` フォルダ検出と警告表示（未検出時の導線）
- P2:
  - 辞書管理パネル（ソース言語/ターゲット言語、辞書構築、最終更新日時）
  - 自動翻訳パネル（適用範囲、プレビュー、未翻訳のみ適用）

## 11. 画面構成への反映方針（次段）
- 右ペイン `Tools` を3ブロックへ再編:
  - `I/O`（Load/Save/Export）
  - `Quality`（Validate/Diff/Encoding）
  - `Translate`（Dictionary/Auto Translate）
- 上部ヘッダに `Profile Bar` を追加:
  - ゲーム種別
  - `workspace_root/Data/Strings` の状態
  - 現在の読み込み対象（Plugin/Strings）
- この変更は `xt_app` のみで実施し、`xt_core` はフォーマット処理/API追加時のみ変更する

## 12. 画像確認ベースのターゲットレイアウト（xTranslator準拠）

### 12.1 全体ゾーン構成（上から順）
- Zone A: メニューバー（`ファイル`, `翻訳`, `オプション`, `ツール`）
- Zone B: ツールバー（小アイコン群、検索入力、補助トグル）
- Zone C: チャネルバー（`STRINGS`, `DLSTRINGS`, `ILSTRINGS` の3レーン）
- Zone D: メイングリッド（列: `EDID`, `ID`, `原文`, `訳文`, `LD`）
- Zone E: ワークタブ列（`ホーム`, `ヒューリスティック候補`, `言語`, `Espツリー`, `Pex解析`, `クエスト一覧`, `NPC/音声リンク`, `ログ`）
- Zone F: ログ/情報ペイン（1行ステータス or 複数行ログ）
- Zone G: ステータスバー（進捗色 + 言語ペア + ファイル名 + 件数）

### 12.2 画面寸法ポリシー
- 既定はデスクトップ固定レイアウト優先（横幅 1200px 以上で崩さない）
- 高さは `Zone D` を最優先で伸縮し、`Zone E/F/G` は固定高を維持
- モバイル最適化は後段（v2）とし、MVPはデスクトップ用途を優先

## 13. 主要コンポーネント仕様（画像準拠）

### 13.1 メニューバー
- `ファイル`:
  - `Esp/Esmファイルを開く`
  - `Stringsファイルを開く`
  - `MCM/Translateテキストを開く`
  - `PapyrusPexを開く`
  - `翻訳ファイルのインポート`（`SST辞書`, `XMLファイル(xTranslator)`, `XMLファイル(EspTranslator)`）
  - `翻訳ファイルのエクスポート`
  - `ユーザー辞書を保存する`
  - `Esp/Esmファイルの上書き出力`
  - `Esp/Esmファイルを別名で出力`
- `オプション`:
  - `言語と辞書`
  - `高度なオプション`
  - `スペルチェック設定`
  - ゲーム切替（Skyrim/FO4など）

### 13.2 チャネルバー
- 3チャネル固定:
  - `STRINGS [current/total]`
  - `DLSTRINGS [current/total]`
  - `ILSTRINGS [current/total]`
- 各チャネルに進捗バー表示（色付き）
- 現在チャネルは枠線または背景色で強調

### 13.3 メイングリッド
- 列定義:
  - `EDID`（内部識別）
  - `ID`（レコード型 + サブタイプ）
  - `原文`
  - `訳文`
  - `LD`（状態/フラグ）
- 行色ポリシー:
  - 自動翻訳候補・既訳・未訳を背景色で区別（画像ではピンク系/青系が確認可能）
- 選択行は濃い枠線で明示
- ヘッダ行は固定（スクロール時も表示）

### 13.4 下部タブとログ
- タブは1行に並べる（`ホーム` から `ログ` まで）
- `ログ` タブでは処理結果と経過時間を表示
- `ホーム` タブはヘルプリンク表示領域として使用可

### 13.5 ステータスバー
- 左側: 進捗バー（緑）
- 中央: 言語ペア（例: `[english]->[japanese]`）
- 中央右: 対象ファイル名
- 右側: 件数（例: `1/1`, `0/0`）と選択数

## 14. モーダル/ダイアログ仕様（画像準拠）

### 14.1 保存確認ダイアログ
- 既存ファイル上書き時に確認:
  - 本文: 「既に存在しています。続行しますか？」
  - 補足: 「元ファイルのバックアップが自動作成されます」
- 操作:
  - `はい`
  - `いいえ`
  - `再表示しない` チェック

### 14.2 辞書構築ダイアログ（言語と辞書）
- 入力:
  - 翻訳元言語
  - 翻訳先言語
  - ゲームDataパス
  - Stringsパス（SE英語設定日本語化時は `Data/Strings/translations`）
- オプション:
  - `stringsフォルダのstringsファイルのみ使用する（bsa/ba2を無視）`
- 実行:
  - `辞書を構築`
- 結果:
  - 完了ポップアップ（処理完了メッセージ + `OK`）

## 15. 現実装との差分（要実装）
- 現実装は2ペイン編集UIであり、xTranslator型の`Zone A-G`構造になっていない
- 現実装にはメニューバー/ツールバー/下部タブ/ステータスバーが未実装
- 現実装の保存操作はXML中心で、`Esp/Esm上書き出力` と `別名出力` が未実装
- 現実装は辞書構築ダイアログが未実装

## 16. 実装段階（UIのみ）
- Phase 1（MVP）:
  - `Zone A, C, D, G` を先行実装
  - `ファイル` メニューに `開く` と `保存（上書き/別名）` を実装
  - `言語と辞書` ダイアログを最小実装
- Phase 2（v1）:
  - `Zone B, E, F` を追加
  - インポート/エクスポートとログ詳細化
  - 行色ルールとヒューリスティック候補連携
