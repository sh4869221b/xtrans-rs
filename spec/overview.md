# Overview

## ステータス
- 状態: Draft
- 最終更新: 2026-01-31
- 方針: **TDD（テスト先行）**で実装（詳細は `spec/tests.md`）

## 概要
Bethesda系ゲームのMOD翻訳作業を支援するデスクトップアプリを、
Linux（優先）向けに新規実装する。互換対象は「xTranslator相当の翻訳編集ワークフロー」。

UI: Dioxus Desktop（WebView）
コア: Rust（UI依存を排除し `xt_core` を窓口に、形式解析は `xt_esp` 等へ分離）

## 用語
- **Entry**: 翻訳単位（原文 `source_text` と訳文 `target_text`）
- **TM（Translation Memory）**: 既存翻訳から得た対訳DB
- **Strings**: `STRINGS / DLSTRINGS / ILSTRINGS`
- **Plugin**: `ESP/ESM/ESL`
- **Hybrid**: Plugin（コンテキスト）+ Strings（実データ）統合
- **Golden fixture**: 入出力の正しさを保証する小さな実ファイル

## 目標 / 非目標
### 目標
- 抽出 → 編集 → 検証 → 書き戻し を一気通貫で提供
- 辞書/TM・検索・差分検出により品質と速度を向上
- ワークスペースとして保存し再現可能にする
- 10万件規模でもUI操作が破綻しない

### 非目標
- xEdit級の汎用レコード編集・競合解消
- localize/delocalize を主目的化
- ネットワーク必須機能（オンライン翻訳等）
