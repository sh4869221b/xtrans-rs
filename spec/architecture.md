# Architecture

## リポジトリ構造（目標）
- `crates/xt_core/`: 形式処理・DB/TM・検証・差分（UI依存なし）
- `crates/xt_app/`: Dioxus Desktop UI（薄い配線）
- `crates/xt_cli/`: 任意のヘッドレスツール

注: 現状は `crates/xt_core` と `crates/xt_app` に分割済み。

## 依存方向（厳守）
- `xt_app -> xt_core -> xt_esp`
- `xt_app` は `xt_esp` を直接参照しない。
- ESP/ESM/ESL 処理は `xt_esp` に実装し、`xt_core::formats::esp` を公開窓口とする。
- 依存は一方向を維持し、循環依存を禁止する。

## コアモジュール（xt_core）
- `workspace`: プロジェクト設定、ロード順、パス解決、キャッシュ
- `formats::strings`: STRINGS/DLSTRINGS/ILSTRINGS 読書き
- `formats::plugin`: Plugin 読書き（v1）
- `formats::archive`: BSA/BA2 抽出（v2）
- `formats::papyrus`: PEX 連携（v2）
- `tm`: 辞書/TM、候補スコアリング
- `index`: SQLite/FTS5（高速検索）
- `validation`: placeholder/タグ/長さ/alias 等
- `diff`: source更新検出（hash）
- `import_export`: XML 入出力（v1）

## UI原則（xt_app）
- **2ペイン**（左：一覧/検索、右：詳細/編集）
- 一覧は **仮想化（virtualized list）** 前提
- UIは `xt_core` のサービスAPIのみを呼ぶ

## ESP/ESM/ESL 解析設計（FO4/Skyrim）

### 目的
- 抽出・編集・書き戻しを可能にする。
- Localized（Strings参照）と inline 文字列の両方を扱う。
- 圧縮レコード（zlib）に対応する。

### パースモデル
- トップレベルは `Record` と `Group (GRUP)` の列。
- Recordヘッダは 24 bytes。
- Groupヘッダも 24 bytes。
- Record内は Subrecord 連結。
- `XXXX` による拡張長サブレコードをサポート。

### 圧縮
- `0x00040000` フラグで zlib 圧縮を示す。
- 先頭4byteの非圧縮サイズ + zlib payload を展開。
- 書き戻し時は同形式で再圧縮。
- LZ4 は v1 範囲外（必要になれば追加）。

### 文字列抽出
- 対象Subrecordは `FULL` / `DESC` から開始。
- ペイロードが4byteで、Strings側にIDがある場合は localized とみなす。
- それ以外は UTF-8 の inline として扱い、文字列妥当性を簡易判定。
- 生成キー: `"{record}:{form_id}:{subrecord}:{index}"`。

### Strings（localized）
- `workspace_root/Data/Strings/` 配下の `{PluginBase}_{language}.strings/.dlstrings/.ilstrings` を解決。
- `workspace_root` は **現状 UI が推定**して渡す（明示的な workspace 設定は後続で実装）。
  - プラグインが `.../Data/*.esm` にある場合: `Data` の親を `workspace_root` とみなす。
  - それ以外: プラグインの親ディレクトリを `workspace_root` とみなす。
- localized 更新は該当IDの文字列を書き戻す（ID未存在はエラー）。

### 書き戻し
- inline: subrecord payload を更新。
- localized: Stringsファイルを更新。
- 他サブレコード構造は保持。

### 拡張予定
- ゲーム別に対象Subrecordタグを拡張。
- LZ4 対応を追加（必要時）。
- レコード種別ごとの詳細バリデーション。

実装は `xt_esp` クレートに分離。詳細は `spec/esp-format.md` を参照。
