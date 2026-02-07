# ESP/ESM/ESL 解析設計（FO4/Skyrim）詳細

実装は `xt_esp` クレートに分離する。

## 目的
- FO4/Skyrim のプラグインから文字列を抽出・編集・書き戻しできること。
- Localized（Strings参照）と inline 文字列の両方を扱うこと。
- 圧縮レコード（zlib）を必須対応とする。

## 前提 / 位置づけ
- 初期対象は **FO4 / Skyrim**。
- 解析は「翻訳対象の文字列抽出・更新」を主目的とし、
  xEdit級の任意レコード編集は対象外。
- 仕様の正式化は **fixture先行**（TDD契約に従う）。

## ブロック構造
- ファイルは `Record` と `Group (GRUP)` の列。
- `Group` は子ブロックを再帰的に持つ。
- `Record` は 24 byte ヘッダ + data payload。

### Record header（24 byte）
- `type` (4) / `data_size` (4) / `flags` (4) / `form_id` (4)
- `stamp` (2) / `version_control` (2) / `version` (2) / `unknown` (2)

### Group header（24 byte）
- `type` = `GRUP` (4) / `group_size` (4) / `label` (4)
- `group_type` (4) / `stamp` (4) / `unknown` (4)

## サブレコード
- Record data は Subrecord 連結。
- Subrecord header: `sub_type` (4) + `len` (2) + `payload`。
- `XXXX` は拡張長: 次の subrecord の長さを `u32` で指定。

## 圧縮
- Record flags の `0x00040000` が zlib 圧縮。
- 圧縮 payload 先頭 4byte は **非圧縮サイズ**（u32 LE）。
- 展開は zlib payload を inflate。
- 書き戻し時は同形式で再圧縮。

## 文字列抽出
- **対象 subrecord**: v1 は `FULL` / `DESC` から開始。
- ペイロードが 4byte で Strings 側に ID が存在する場合は **localized**。
- それ以外は UTF-8 inline として扱い、簡易文字列判定を実施。
- 生成キー: `"{record}:{form_id}:{subrecord}:{index}"`

> 対象 subrecord の拡張は fixture に合わせて段階的に行う。

## Localized Strings
- Strings ファイル解決（`workspace_root` を基準）:
  - `workspace_root/Data/Strings/{PluginBase}_{language}.strings`
  - `workspace_root/Data/Strings/{PluginBase}_{language}.dlstrings`
  - `workspace_root/Data/Strings/{PluginBase}_{language}.ilstrings`
- `workspace_root` は現状 UI 側で推定する（明示的ワークスペースは後続対応）:
  - プラグインが `.../Data/*.esm` にある場合は `Data` の親を `workspace_root` とする
  - それ以外はプラグインの親ディレクトリを `workspace_root` とする
- 取得時は ID で文字列を引く。
- 更新時は該当 ID の entry を更新（存在しない場合はエラー）。新規追加は行わない。

## 書き戻し
- inline: subrecord payload を更新（null 終端は保持）。
- localized: Strings ファイルの該当 ID を更新。
- それ以外の subrecord・record 構造は保持。

## エラーハンドリング
- 解析不能なレコードは失敗扱い（テストで検知）。
- Strings ファイル未検出は localized 更新時にエラー。
- 圧縮展開失敗は record 解析失敗扱い。

## テスト（TDD）
- **T-ESP-EX-001**: 抽出→編集→書戻し→再読込一致
  - inline / localized / compressed の最小ケースをテスト内で生成
- Fixture の不足はテスト内生成で補う。

## 未決定事項（要追加設計）
- 対象 subrecord タグの拡張リスト（FO4/Skyrim）
- LZ4 圧縮対応の必要性（v1は対象外）
- レコード固有の禁止文字／長さ制限
- ESL/ESM 固有のフラグやロード順の影響
