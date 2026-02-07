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

### C) 翻訳支援
- **FR-TM-01（MVP）** Stringsから対訳抽出 → TM登録
- **FR-TM-02（v1）** EspCompare（2esp比較で対訳生成）
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
- **FR-UNDO-01（v1）** Undo/Redo（単一/バッチ/インポート）

### F) アーカイブ・音声
- **FR-ARC-01（v2）** BSA/BA2 抽出
- **FR-FUZ-01（v2）** 音声突合（dialog→音声）

### G) UI
- **FR-UI-01（MVP）** 2ペイン + 仮想化リスト + 検索/編集/候補/検証
- **FR-UI-02（v1）** ダイアログ特化ビュー（会話系）
- **FR-UI-03（v1）** UIローカライズ

## 非機能要件（NFR）
- **NFR-01 性能**: 10万Entry, 検索(FTS)<300ms, UIが固まらない
- **NFR-02 再現性**: ワークスペース設定は保存可能、環境差に耐える
- **NFR-03 安全性**: 書戻し前バックアップ、失敗時ロールバック
