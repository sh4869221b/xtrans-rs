# テスト台帳（T-*）

ここがTDDの実装順序。チェックが付いたもののみ実装してよい。

## Phase 1（MVP: Strings + TM/検索 + Validation + UI）
- [x] **T-WS-001**: workspace作成→保存→再読込が同値（FR-WS-01）
- [x] **T-STR-RT-001**: STRINGS round-trip（ASCII/JP/改行）（FR-MODE-02）
- [x] **T-STR-RT-002**: DLSTRINGS round-trip（FR-MODE-02）
- [x] **T-STR-RT-003**: ILSTRINGS round-trip（FR-MODE-02）
- [x] **T-TM-001**: StringsからTM構築→完全一致ヒット（FR-TM-01）
- [x] **T-SRCH-001**: source/target検索が期待通り（FR-SRCH-01）
- [x] **T-HEU-001**: 完全一致 > 部分一致 > 類似 の順位保証（FR-HEU-01）
- [x] **T-VAL-PH-001**: `{0}` 不一致はError（FR-VAL-01）
- [x] **T-VAL-PH-002**: `%s/%d` 不一致はError（FR-VAL-01）
- [x] **T-UI-001**: 2ペインが起動し、Entry選択→詳細表示できる（FR-UI-01）
- [x] **T-UI-002**: 10k seedで一覧スクロールが破綻しない（FR-UI-01）

## Phase 2（v1: ESP/Hybrid/XML/Undo/Diff）
- [x] **T-ESP-EX-001**: plugin fixture抽出→編集→書戻し→再読込一致（FR-MODE-01）
- [x] **T-HYB-CTX-001**: hybridでコンテキスト参照が引ける（FR-MODE-03）
- [x] **T-XML-RT-001**: xml export→importで同値（FR-XML-01）
- [x] **T-UNDO-001**: 単一編集→Undo/Redo一致（FR-UNDO-01）
- [x] **T-UNDO-002**: バッチ置換→Undo一致（FR-UNDO-01）
- [x] **T-DIFF-001**: source変更でNeedsReviewが立つ（FR-DIFF-01）
- [x] **T-VAL-ALIAS-001**: Aliasタグ整合チェック（FR-VAL-02）
- [x] **T-ENC-001**: 代表エンコーディングのround-trip（FR-ENC-01）
- [x] **T-XML-APPLY-001**: XML default profile適用で updated/unchanged/missing が正しい（FR-XML-02）
- [x] **T-XML-IMPORT-002**: xTranslator XML（`SSTXMLRessources`）をimportして `Source/Dest` を抽出できる（FR-XML-04）
- [x] **T-XML-APPLY-002**: key不一致でも source一致（一意）で訳文適用できる（FR-XML-05）
- [x] **T-XML-APPLY-003**: source一致が競合する場合は未適用（missing）になる（FR-XML-06）
- [x] **T-DICT-001**: 辞書Quick AutoTranslateは選択範囲のみ更新できる（FR-AUTO-03）
- [x] **T-DICT-002**: Stringsディレクトリから辞書構築できる（FR-DICT-02）
- [x] **T-APP-004**: 辞書設定の保存フォーマットは round-trip できる（FR-DICT-03）
- [x] **T-APP-005**: Quick AutoTranslate は選択行なしで実行不可になる（FR-AUTO-03）
- [x] **T-APP-006**: XML適用ヘルパーが更新件数を返す（FR-XML-07）
- [x] **T-BATCH-001**: `--load --importxml --finalize` 引数の解析が成立する（FR-BATCH-01）
- [x] **T-E2E-BOOT-001**: 起動直後の状態が空（0件/未選択）である（FR-UI-01）
- [x] **T-E2E-IO-001**: Strings読込→編集→保存で round-trip できる（FR-SAVE-01）
- [x] **T-E2E-XML-001**: XMLエディタ適用で対象行が更新される（FR-XML-02）
- [x] **T-E2E-DICT-001**: 辞書構築→Quick Auto（選択行）で訳文適用される（FR-AUTO-03）

## Phase 3（v2: Archive/PEX/音声）
- [ ] **T-BA2-EXT-001**: ba2 fixture抽出→ハッシュ一致（FR-ARC-01）
- [ ] **T-PEX-LOCK-001**: 編集不可領域が更新できない（FR-MODE-05）
- [ ] **T-FUZ-MAP-001**: dialog→音声が引ける（FR-FUZ-01）
