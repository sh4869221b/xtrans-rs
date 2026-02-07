# Scope

## 仕様基準
- 本設計の機能基準は、xTranslatorの確認済み運用仕様（ゲーム選択、Strings下準備、辞書構築、ESP/Strings編集、自動翻訳、保存）を正とする。
- 未実装項目は `spec/requirements.md` の優先度に沿って段階導入する。

## 対象ゲーム（Workspace種別）
- Skyrim / Skyrim SE/AE
- Fallout 4
- Starfield

MVPでは1タイトル固定でもよいが、モデルと設定は拡張可能に設計する。

## 入出力形式
- Plugin: ESP/ESM/ESL（v1+）
- Strings: STRINGS/DLSTRINGS/ILSTRINGS（MVP）
- Archive: BSA/BA2（v2+）
- Papyrus: PEX（v2+）
- Import/Export: XML（v1+）

## 運用前提（xTranslator互換）
- `Data/Strings` を翻訳基盤ディレクトリとして扱う。
- Skyrim SE/AE（英語設定日本語化）では `Data/Strings/Translations` を辞書参照先として扱えることを前提にする。
