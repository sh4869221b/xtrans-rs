# Data Model

## Entry
- `key`: 一意キー（例: `strings:{file}:{id}` / `plugin:{name}:{form}:{path}`）
- `origin`: 出自（plugin名/strings名 等）
- `source_text`: 原文
- `target_text`: 訳文
- `status`: `Untranslated | Draft | Reviewed | NeedsReview | Error`
- `source_hash`: 原文ハッシュ（差分検出）
- `updated_at`: unix epoch

## RowSummary（UI一覧）
- `key`, `status`, `origin`
- `source_preview`, `target_preview`
- `has_diff`: 原文更新ありフラグ

## ValidationIssue
- `entry_key`
- `severity`: `Info | Warn | Error`
- `rule_id`
- `message`
