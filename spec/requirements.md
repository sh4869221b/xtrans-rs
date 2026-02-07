# Requirements

## 機能要件（FR）
優先度: MVP / v1 / v2

### A) ワークスペース
- **FR-WS-01（MVP）** ワークスペース作成・保存・読込
- **FR-WS-02（v1）** 複数ワークスペース運用（切替/混線なし）

### B) 編集モード
- **FR-MODE-01（v1）** ESP mode（抽出→編集→書戻し）
- **FR-MODE-02（MVP）** Strings mode（STRINGS/DLSTRINGS/ILSTRINGS 読書き）
- **FR-MODE-03（v1）** Hybrid mode（Plugin参照 + Strings編集）
- **FR-MODE-04（v1）** MCM/Translate（UI文字列ファイル）読書き
- **FR-MODE-05（v2）** Papyrus PEX（編集不可領域ロック含む）
- **FR-MODE-06（MVP）** Game profile（Skyrim/FO4）選択とプロファイル状態表示

### C) 翻訳支援
- **FR-TM-01（MVP）** Stringsから対訳抽出 → TM登録
- **FR-TM-02（v1）** EspCompare（2esp比較で対訳生成）
- **FR-DICT-01（MVP）** 辞書言語ペア設定（source/target）
- **FR-DICT-02（MVP）** 辞書構築（指定したStringsディレクトリを入力）
- **FR-AUTO-01（MVP）** 辞書ベース自動翻訳（未翻訳のみ対象）
- **FR-AUTO-03（MVP）** Quick AutoTranslate（選択範囲に対する即時適用）
- **FR-AUTO-02（v1）** 自動翻訳の適用範囲指定（全件/選択/フィルタ結果）
- **FR-SRCH-01（MVP）** 原文/訳文/ID/参照で検索
- **FR-HEU-01（v1）** 類似候補提示（順位保証）
- **FR-REGEX-01（v1）** 正規表現検索/置換（範囲指定）
- **FR-SPELL-01（v2）** スペルチェック（辞書差し替え）
- **FR-ONTR-01（v2,任意）** オンライン翻訳（プロバイダ差替/キャッシュ）

### D) 品質・安全
- **FR-DIFF-01（v1）** 原文更新検出→NeedsReview
- **FR-VAL-01（MVP）** placeholder整合（{0}, %s/%d 等）
- **FR-VAL-02（v1）** aliasタグ整合（<Alias=...> 等）
- **FR-ENC-01（v1）** エンコーディング破損防止

### E) 入出力・共有
- **FR-XML-01（v1）** XML import/export
- **FR-XML-02（MVP）** 翻訳XMLの一括適用（default profile）
- **FR-XML-03（v1）** XMLドラッグ&ドロップ適用
- **FR-UNDO-01（v1）** Undo/Redo（単一/バッチ/インポート）
- **FR-SAVE-01（MVP）** Strings形式への保存（元拡張子を保持）
- **FR-SAVE-02（v1）** Plugin形式への保存（ESP/ESM/ESL）
- **FR-IO-ERR-01（MVP）** I/O失敗時に詳細表示（ファイル名/原因/位置）

### I) バッチ運用
- **FR-BATCH-01（v1）** `load -> importxml -> finalize` を非対話で実行可能
- **FR-BATCH-02（v1）** 辞書生成/保存をバッチコマンドから実行可能

### H) 互換運用
- **FR-XT-01（MVP）** `Data/Strings` 配下運用を前提にしたパス解決
- **FR-XT-02（MVP）** `Data/Strings/Translations` 不在時の警告表示
- **FR-XT-03（v1）** `Translations` 初期化補助（作成ガイド/自動生成）

### F) アーカイブ・音声
- **FR-ARC-01（v2）** BSA/BA2 抽出
- **FR-FUZ-01（v2）** 音声突合（dialog→音声）

### G) UI
- **FR-UI-01（MVP）** xTranslator準拠の画面ゾーン `A-G` を提供（メニュー/チャネルバー/グリッド/タブ/ログ/ステータス）
- **FR-UI-02（MVP）** `STRINGS/DLSTRINGS/ILSTRINGS` 3チャネル表示（`current/total` と進捗バー）
- **FR-UI-03（MVP）** メイングリッド列 `EDID/ID/原文/訳文/LD` を固定ヘッダで表示
- **FR-UI-04（MVP）** `ファイル` メニューに `開く` と `保存（上書き/別名）` を提供
- **FR-UI-05（MVP）** 上書き保存時に確認ダイアログ（`はい/いいえ/再表示しない`）を表示
- **FR-UI-06（MVP）** `オプション > 言語と辞書` ダイアログで言語ペア/パス/辞書構築を実行可能
- **FR-UI-07（MVP）** ステータスバーに進捗、言語ペア、対象ファイル、件数を表示
- **FR-UI-08（v1）** 下部ワークタブ（`ホーム/ヒューリスティック候補/言語/Espツリー/Pex解析/クエスト一覧/NPC音声リンク/ログ`）を提供
- **FR-UI-09（v1）** 行状態（既訳/未訳/候補）を背景色で視覚区別
- **FR-UI-10（v1）** UIローカライズ（日本語/英語切替）
- **FR-UI-11（v2）** モバイル最適化レイアウト（現状はデスクトップ優先）

## 非機能要件（NFR）
- **NFR-01 性能**: 10万Entry, 検索(FTS)<300ms, UIが固まらない
- **NFR-02 再現性**: ワークスペース設定は保存可能、環境差に耐える
- **NFR-03 安全性**: 書戻し前バックアップ、失敗時ロールバック
