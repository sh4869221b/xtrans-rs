# xTranslator 詳細仕様書（再分析版）

## 0. 文書管理

- 文書名: xTranslator 詳細仕様書（ソース/ドキュメント分析）
- 分析対象リポジトリ: `https://github.com/MGuffin/xTranslator`
- 固定コミット: `9aa38d60860273401f8bb0dd1557c82a295c41b3`
- upstream HEAD 確認日: 2026-02-07（上記コミットと一致）
- 対象バージョン表記: `v1.6.0`（`TESVT_Const.pas`）
- 目的:
1. xTranslator の実装仕様を再現可能な粒度で言語化する
2. `xtrans-rs` 側の設計判断に使える参照仕様を提供する
3. 重点ワークフロー（SST自動翻訳 / XML一括翻訳）の仕様を明確化する

## 1. 分析方法と範囲

### 1.1 分析方法

- 静的解析（Delphi/Pascal ソース + DFM + 同梱ドキュメント）
- 主根拠:
  - `xTranslator.dpr`
  - `TESVT_main.pas`, `TESVT_main.dfm`
  - `TESVT_MainLoader.pas`
  - `TESVT_typedef.pas`
  - `TESVT_SSTFunc.pas`
  - `TESVT_XMLFunc.pas`
  - `TESVT_TranslatorApi.pas`
  - `TESVT_Const.pas`
  - `Res/English/Res.ini`, `Res/English/Tutorial.htm`, `Res/English/manual.htm`
  - `Misc/ApiTranslator.txt`, `Misc/customTxtDefinition.txt`
  - `Batch/*.txt`

### 1.2 範囲外

- 実行時のみ観測できる未公開仕様
- Nexus 記事等の外部ページ依存仕様（本書は repo 同梱物とソースを正とする）

## 2. プロダクト定義

### 2.1 対象ゲーム

`TESVT_Const.pas` `aPROG_NAME`:

1. Fallout4
2. Skyrim
3. FalloutNV
4. SkyrimSE
5. Fallout76
6. Starfield

### 2.2 動作モード

`sTESVTMode`:

1. `sTESVTNone`
2. `sTESVTsstCustom`
3. `sTESVTsstEdit`
4. `sTESVStrings`
5. `sTESVEsp`
6. `sTESVEspStrings`（Hybrid）
7. `sTESVMCM`
8. `sTESVPex`

表示ラベル (`sModeLabel`):

- None
- sstEditCustom
- sstViewEdit
- Strings
- Esp
- Strings+ EspLayout
- MCMTranslate
- ScriptPex

### 2.3 主要制約（明示）

`README.md` の明示事項:

1. xTranslator は esp/esm の localize/delocalize ツールではない（xEdit を使用する前提）
2. 純 STRINGS モード辞書運用は歴史的に縮退（Hybrid 推奨）

## 3. UI 仕様

### 3.1 画面全体構成

`TESVT_main.dfm` ベース:

1. ネイティブメニューバー
2. 上部ツールバー・検索バー
3. 中央メイングリッド（`STRINGS` / `DLSTRINGS` / `ILSTRINGS`）
4. 下部ワークタブ（Home, Vocabulary, Heuristic, EspTree, Pex, Quests, Npc/Fuz, Log ほか）
5. 最下部ステータスバー（4パネル）

### 3.2 メイン文字列グリッド

3タブ共通の主列:

1. EDID
2. ID
3. Original
4. Translated
5. LD（Levenshtein距離/類似度関連）

コンテキストメニュー主要操作:

1. Validate（F1）
2. Partial（F2）
3. Locked/No-Trans（F3）
4. Cancel/Untranslate（F4）
5. Collab ID（F9）/ Reset（F10）
6. FormID フィルタ（F12）
7. API Translation Array（Ctrl+F5）

### 3.3 下部タブ仕様

`pagecontrol2`:

1. `Home`（HtmlViewer）
2. `Vocabulary`（辞書ツリー、並べ替え、検索、適用）
3. `HeuristicSuggestions`（候補一覧）
4. `EspTree`（レコードツリー）
5. `PexData`（`PexAsm` / `PexScript`）
6. `Quests`
7. `Npc/Fuz Map`
8. `Records_def`（隠し）
9. `Log`

### 3.4 メニュー仕様（主要）

### File

1. Load Esp/Esm (`Ctrl+O`)
2. Load Strings
3. Load MCM/Custom text
4. Load PapyrusPex
5. Load BSA/BA2
6. Import Translation（SST / XML(xTranslator) / XML(EspTranslator)）
7. Export Translation（SST / XML）
8. Save SST (`Ctrl+S`)
9. Finalize（Esp/Strings/MCM/Pex）
10. Close Current (`Ctrl+W`)
11. Close All

### Translation

1. Exact Translation (`Ctrl+T`)
2. Heuristic Translation (`Ctrl+H`)
3. API Translation
4. Search and Replace (`Ctrl+F`)
5. Validate All
6. Reset All
7. Undo (`Ctrl+Z`)

### Options

1. Dictionaries and languages
2. Options / Advanced options / Startup
3. Translator APIs
4. SpellCheck options
5. Display/Theme
6. Workspace restart/switch
7. Language presets
8. Force codepage

### Tools

1. Esp Compare
2. Load Strings as Dest
3. MCM Compare
4. Batch Search & Replace
5. Regex Translation
6. SpellCheck
7. Load All Masters
8. Header Processor
9. Tag NoTrans
10. Add Id to Strings

## 4. 入出力とロードディスパッチ

### 4.1 入力形式

1. `*.esp`, `*.esm`, `*.esl`
2. `*.strings`, `*.dlstrings`, `*.ilstrings`
3. `*.sst`
4. `*.xml`
5. `*.txt` 等の Custom text（`customTxtDefinition.txt` による）
6. `*.pex`
7. `*.bsa`, `*.ba2`

### 4.2 `openAddonCommandLine` の拡張子ディスパッチ

`TESVT_main.pas`:

1. `.pex` -> `loadpex`
2. `.sst` -> `importSSTDirect`
3. `.xml` -> `importXMLDirect`
4. custom txt ext -> `loadMCM`
5. `.esp/.esm/.esl` -> `loadSingleEsp`
6. `.bsa/.ba2` -> `OpenBSA`

補足:

- Drag & Drop でも同経路を使用（`WMDropFiles`）

### 4.3 保存/Finalize 仕様

`SaveFile` は mode により分岐:

1. `sTESVPex` -> `FinalizePex`
2. `sTESVMCM` -> `FinalizeMCM`
3. ESP保存可能モード -> `FinalizeEsp`
4. それ以外 -> `FinalizeStrings`

共通挙動:

1. 事前警告ダイアログ（言語ペア含む）
2. 既存出力がある場合の上書き確認
3. バックアップ設定が有効なら事前バックアップ
4. 固定出力フォルダ指定オプションを尊重

### 4.4 Strings モードの位置づけ

- `Load Strings` は UI 上で deprecated 警告 (`Ask_WarningStringsDeprecated`)
- 互換のため残存するが、主運用は ESP/Hybrid 側に寄せる思想

## 5. ローダーと状態遷移

### 5.1 Loader

`tTranslatorLoader` がファイル単位の編集中核。

保持:

1. `listArray[0..2]`（STRINGS/DLSTRINGS/ILSTRINGS）
2. addon metadata（folder/name/lang/ext）
3. loaderType（esp/mcm/pex/strings 等）
4. `LoaderMode`

`setLoaderMode` は `CurrentTESVmode` を同期更新。

### 5.2 マルチロード

- 複数ファイル同時ロード対応（esp/pex）
- ローダー間切替時に表示位置やソート列を保持
- 言語ペア変更時は cache/unload/reload を実施し整合確保

### 5.3 workspace 不一致時の挙動

`doloadEsp`:

1. フォームバージョンから想定ゲームを推定
2. 現workspaceと不一致なら確認
3. 条件により再起動して正workspaceへ移行可能
4. 必要なら quick codepage 指定を挟む

## 6. データモデル仕様

### 6.1 中核型

`TESVT_typedef.pas`:

1. `tSkyStr`
2. `rEspPointer` / `rEspPointerLite`
3. `tEspStrRef`

`tSkyStr` 主フィールド:

1. `s`（source）
2. `sTrans`（dest）
3. `listIndex`（0/1/2）
4. `esp`（record/field/stringID 参照）
5. `VMAD` 参照
6. `sparams`（永続状態）
7. `sInternalparams`（内部状態）
8. `colabId`

### 6.2 永続状態（`sStrParam`）

1. `translated`（白）
2. `lockedTrans`（黄）
3. `incompleteTrans`（部分）
4. `validated`（青）
5. `oldData`
6. `pending`

`manual.htm` では運用意味を次のように定義:

1. translated: 自動適用済み
2. validated: ユーザー確定（辞書蓄積対象）
3. partial: 作業中/要確認（辞書には残るが扱いが異なる）
4. cancel/untranslated: 未翻訳へ戻す

### 6.3 内部状態（`sStrInternalParam`）

代表:

1. `aliasError`
2. `stringSizeError`
3. `stringCRError`
4. `isOrphean`
5. `isLookUpFailed`
6. `isVMADString`
7. `unusedInSST` / `AllUnusedInSST`
8. `OnTranslationApiArray*`
9. `StringIdChanged`

## 7. SST 辞書仕様

### 7.1 役割

SST は主辞書形式であり、以下を担う。

1. 自動翻訳の語彙基盤
2. 作業状態保持（partial/pending/collab）
3. 旧データ保持（mod更新差分追従）
4. 共有/分担翻訳（CollabID）

### 7.2 バージョン/ヘッダ

`VocabUserHeader..VocabUserHeader8`:

1. v5: real edidHash
2. v6: Collab ID
3. v7: Collab Label
4. v8: Master list

互換:

1. 新版は旧版読込を考慮した分岐を持つ
2. 逆方向（旧ツールで新SST）は非保証

### 7.3 v8 バイナリレイアウト（保存順）

`SaveSSTFile` 実装に基づく順序:

1. Header (`VocabUserHeader8`)
2. placeholder flag byte
3. masterList count + 各文字列長/文字列
4. colabLabel count + 各 (colabId, 文字列長, 文字列)
5. エントリ反復:
   - `listIndex`
   - `rEspPointerLite`（`strId`, `formID`, `rName`, `fName`, `index`, `indexMax`, `rHash`）
   - `colabId`
   - `sparams`（validated は保存時に除外）
   - source length + source
   - dest length + dest

### 7.4 読み込み時補正

`loadSstEdit` / `loadVocabUserCache`:

1. 旧versionの deprecated param を除去
2. `oldData` -> internal 未使用フラグへ変換
3. `pending` は resetTrans 等の補正
4. cache読込時は partial/locked/pending を除外する経路あり（条件依存）

### 7.5 優先順位

`README.md`:

1. Vocabulary リスト上位辞書が優先
2. 同一 source 競合時は上位が勝つ
3. 並び替え（drag/drop）可能

## 8. XML 仕様

### 8.1 xTranslator XML スキーマ

ルート:

- `SSTXMLRessources`

`Params`:

1. `Addon`
2. `Source`
3. `Dest`
4. `Version`（実装上 `2`）

`Content/String`:

属性:

1. `List`
2. `sID`（mode 条件で出力）
3. `Partial`（`1`: incomplete, `2`: locked）

子要素:

1. `EDID`
2. `REC`（text: `RRRR:FFFF`, 属性 `id`, `idMax`）
3. `Source`
4. `Dest`
5. `FuzInfo`（条件付き）

### 8.2 エクスポート規則

`saveXMLFile`:

1. 対象選別は `fProc`（全部/validated/選択/差分等）
2. mode により `EDID/REC/sID` を付与
3. Dialog/Fuz map があれば `FuzInfo` を埋める

### 8.3 インポート規則（xTranslator XML）

`XMLImportbase`:

1. `SSTXMLRessources` ルート存在チェック
2. `Params/Version=2` で `bNewXML` 扱い
3. `Content/String` を辞書リスト化
4. マッチ戦略は `idProc` / XML内容により切替
   - EDID/Form 参照が使える場合: EDID比較関数
   - 使えない場合: source文字列比較
5. VMAD は別分岐で strict/string fallback を適用

### 8.4 EspTranslator XML 互換

`XMLImportbase_EspTranslator` を別実装で保持。

## 9. 翻訳エンジン仕様

### 9.1 主要処理

`TESVT_TranslateFunc.pas`:

1. `findStrMatchEx`（source主導）
2. `findEdidMatchEx`（EDID/Form主導）
3. heuristic 候補生成と LD 管理

### 9.2 自動翻訳状態

README と実装から:

1. 辞書適用で変更された文字列は validated 扱いへ寄せる運用がある
2. Heuristic 結果は未確定扱い（manual 照合前提）
3. Lock/Partial/Filter 条件により適用対象を制御

### 9.3 VMAD

README + 実装分岐:

1. VMAD文字列は通常 string2string を抑制/制限
2. strict match や明示選択時のみ反映
3. 誤適用防止が設計優先

## 10. Translation API 仕様

### 10.1 API 一覧

`MaxApiCount=7`:

1. MsTranslate
2. Yandex（deprecated）
3. Baidu
4. Youdao
5. freeApi（deprecated）
6. Google
7. DeepL
8. OpenAI

`Misc/ApiTranslator.txt` で enabled 初期値を持つ。

### 10.2 Provider 設定項目

各APIに対して次を保持:

1. `CharLimit`
2. `ArrayLimit`
3. `ArrayTimePause`
4. `ArrayMaxCharPerMin`
5. `SingleTimePause`
6. 言語コードマップ
7. API URL

OpenAI 固有:

1. `OpenAI_ApiUrl`（既定: chat/completions）
2. `OpenAI_ModelN`
3. `OpenAI_DefaultQuery`
4. Query テンプレート内 `%lang_source%`, `%lang_dest%` 置換

### 10.3 有効化条件

`enabledAPIs`:

1. `*_enabled=true` かつ
2. 必須キーが設定済み（例: OpenAI_Key, DeepL_Key）かつ
3. 当該APIで source/dest 言語マップが解決可能

### 10.4 配列翻訳の実装要点

1. Google/OpenAI は仮想配列（改行連結）でまとめ送信
2. 受信後に改行パターンで再分解
3. 不整合時はエラー扱い（`Fbk_VirtualArrayNoMatch`）
4. CRLF 保全用タグ (`<L_F>`) を使用する経路あり

### 10.5 認証/通信

1. REST Client/Request を都度構築
2. OpenAI は `Authorization: Bearer ...` を付与
3. DeepL は free/pro エンドポイントをキー末尾で切替（`:fx`）
4. proxy 設定項目を保持

## 11. Batch Processor 仕様

### 11.1 ファイル構文

`batchCommands`:

1. コメント行除去
2. global パラメータ:
   - `global_vocabfolder`
   - `global_importfolder`
   - `global_exportfolder`
3. ルールブロック:
   - `startrule`
   - 各種 key/value
   - `command=...`
   - `endrule`

### 11.2 コマンドセット

`runCommands`:

1. `loadfile...`
2. `apitranslation...`
3. `loadmasters`
4. `finalize`
5. `closefile`
6. `closeall`
7. `savedictionary`
8. `generatedictionaries`
9. `applysst...`
10. `importsst...`
11. `importxml...`

### 11.3 パラメータ規約（実装由来）

`applysst/importsst/importxml` は固定位置で単一数字パラメータを抽出:

1. overwrite mode（0..4）
2. apply mode（0..3）
3. filename

overwrite mode（`OverWrite`）:

1. all
2. non-trans only
3. non-trans + partial除外条件
4. partial only
5. selection

apply mode（`apply_Mode`）:

1. FORMID only
2. FORMID + string strict
3. FORMID + string relax
4. string only

## 12. 言語・辞書設定仕様

### 12.1 言語ペア

`SetLanguagePair`:

1. codepage 定義に存在する言語のみ有効
2. 変更時は cache unload/rebuild を実施
3. 全 loader を再評価

### 12.2 Dictionaries and languages ダイアログ

`TESVT_LangPref.dfm` + `Res.ini`:

1. Source / Dest language
2. Data folder（esm, bsa/ba2）
3. Strings folder（loose strings）
4. Build dictionaries ボタン
5. only loose strings オプション
6. Preview / BSA定義タブ

### 12.3 Custom text 定義

`Misc/customTxtDefinition.txt`:

1. 形式ごとに regex, encoding, id backref, ext 制約を定義
2. fallback ルールあり
3. 対応拡張子リストを外部設定化

## 13. ESP / STRINGS / Archive 仕様

### 13.1 ESP

`doloadEsp`:

1. ヘッダ検証
2. localized/unlocalized 判定
3. workspace 整合確認
4. localized の場合は strings 連携ロード

### 13.2 STRINGS

`saveStringFile` / load 系:

1. 3種（S/DL/IL）を独立管理
2. codepage と source/dest language 依存
3. localized lookup failure 補正経路あり

### 13.3 BSA/BA2

1. archive ブラウズ
2. strings/script 取り出し
3. Finalize 時の再統合（条件付き）
4. Fallout76 向け専用注入処理分岐あり

## 14. MCM / PEX 仕様

### 14.1 MCM / Custom text

1. custom definition に応じた parse
2. 言語 suffixe 付与可否を format ごとに制御
3. SaveAs 時に suffixe 規則を適用

### 14.2 PEX

1. 免責確認ダイアログ後にロード
2. PexAsm / PexScript の二面表示
3. FinalizePex で既存ファイル確認 + backup 対応

## 15. 主要ユーザーワークフロー（要求重点）

### 15.1 SST 辞書を使った自動翻訳

1. `File -> Load Esp/Esm`
2. `Options -> Dictionaries and languages` で言語ペア/辞書準備
3. Vocabulary 順序を調整（優先度制御）
4. SST を Apply（必要なら compare/apply mode 指定）
5. 未訳に Heuristic / API を適用
6. F1/F2/F3/F4 で状態を確定
7. `Save SST` で辞書へ蓄積
8. `Finalize` で esp/strings 出力

### 15.2 XML 一括翻訳

1. 対象ファイルをロード
2. `File -> Import -> XML (xTranslator or EspTranslator)`
3. overwrite/apply mode を選択
4. EDID strict / relax / string-only で照合
5. 反映結果を色/LD/ログで検証
6. 必要箇所を手修正・Validate
7. Finalize

## 16. 仕様上の注意点

1. 新旧 SST 互換は一方向中心（新->旧は非保証）
2. pure strings 運用は legacy 扱い
3. VMAD は厳密運用前提（誤適用リスク高）
4. API 配列翻訳は分割不整合時のフォールバックを要する
5. workspace/codepage 不一致は文字化け/破損の主要原因として扱う

## 17. xtrans-rs 参照適用境界

この文書は外部参照であり、`xtrans-rs` の正式要件は `spec/` を正とする。

実装優先度（推奨）:

1. `File -> Esp/Esm Open` 起点の統一フロー
2. SST apply/save（最重要）
3. XML import（xTranslator形式優先）
4. Finalize（上書き/別名/確認）
5. API translation / batch は後段拡張

## 18. 差異記録テンプレート

```md
## 差異ID
- Area:
- xTranslator仕様:
- xtrans-rs現状:
- 影響:
- 対応方針:
- 優先度:
- 根拠ソース:
```

## 付録A. 根拠ファイル一覧（再分析で参照）

1. `xTranslator.dpr`
2. `TESVT_main.dfm`
3. `TESVT_main.pas`
4. `TESVT_MainLoader.pas`
5. `TESVT_Const.pas`
6. `TESVT_typedef.pas`
7. `TESVT_SSTFunc.pas`
8. `TESVT_XMLFunc.pas`
9. `TESVT_TranslatorApi.pas`
10. `TESVT_LangPref.dfm`
11. `TESVT_ApplySSTOpts.dfm`
12. `TESVT_EspCompareOpts.dfm`
13. `Misc/ApiTranslator.txt`
14. `Misc/customTxtDefinition.txt`
15. `Res/English/Res.ini`
16. `Res/English/manual.htm`
17. `Res/English/Tutorial.htm`
18. `README.md`
19. `Batch/BatchExample-BetterItemSorting_en_fr.txt`
20. `Batch/JP-BatchExample-BetterItemSorting_en_ja.txt`

---

注記:

- 本書は「上記コミット時点の実装」を仕様化したものであり、将来コミットで変更される可能性がある。
