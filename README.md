# xtrans-rs

## Purpose

This project aims to re-implement the xTranslator workflow in Rust with an `eframe/egui` desktop UI.
The target is workflow compatibility, not a byte-for-byte clone of the original application.

## Current Scope

The current implementation intentionally focuses on the following workflows only:

1. Dictionary-based auto translation (build/apply dictionary)
2. XML-based bulk translation import/apply (including xTranslator XML compatibility)

Current UI operation for these workflows:

- Dictionary build: `翻訳 > 辞書を構築`
- Quick AutoTranslate: `翻訳 > Quick自動翻訳` or `Ctrl-R` (selected row only)
- XML bulk apply: `ファイル > 翻訳XMLを開く` / editor apply

Everything outside these workflows is partial, experimental, or not implemented yet.

## Legal and Attribution

- This project is an independent implementation and is not affiliated with xTranslator or its maintainers.
- Development follows a clean-room approach focused on workflow compatibility.
- The upstream xTranslator project is licensed under MPL-2.0: <https://github.com/MGuffin/xTranslator/blob/main/LICENSE>
- This repository is licensed under MIT (`LICENSE`).
- Third-party crate licenses are tracked in `THIRD_PARTY_NOTICES.md`.

## Development

This workspace is split into a core library and a desktop app.

```text
project/
├─ crates/
│  ├─ xt_core/ # Core library (format parsing, TM, validation, etc.)
│  │  ├─ src/
│  │  └─ tests/fixtures/
│  └─ xt_app/ # eframe/egui desktop app + batch CLI
│     ├─ src/main.rs
│     ├─ src/lib.rs
│     └─ src/bin/xt_batch.rs
├─ Cargo.toml # Workspace definition
```

### Quick Commands (`just`)

```bash
just test-core
just test-esp
just test-app
just build-app
just serve
just dev
```

### Live Reload (Development)

`xt_app` supports two development loops:

1. Hotpatch (best effort, no restart for some edits)
2. Auto restart fallback (rebuild + relaunch)

Install tools:

```bash
cargo install dioxus-cli cargo-watch
```

Run default development mode:

```bash
just dev
```

Use hotpatch explicitly:

```bash
just dev-hotpatch
```

Use auto-restart explicitly:

```bash
just dev-restart
```

Notes for hotpatch mode (`subsecond`):

- Detection is subsecond-class (<1s), but rebuild/apply latency depends on Rust compile time.
- Only the tip crate is reliably patched. In this workspace, edits under `crates/xt_app` are the primary target.
- Edits in `crates/xt_core` / `crates/xt_esp` should be treated as restart-required changes.
- Structural changes (for example struct layout, some thread-local heavy paths) may require restart.
- If hotpatch is unstable, switch to `just dev-restart`.

### Run Desktop App

```bash
cargo run -p xt_app
```

### Batch Workflow (`xt_batch`)

Run batch commands from repo root:

```bash
cargo run -p xt_app --bin xt_batch -- --load base.xml --importxml tr.xml --finalize out.xml
```

#### Strings pipeline

```bash
cargo run -p xt_app --bin xt_batch -- \
  --load-strings Data/Strings/mod_english.strings \
  --importxml tr.xml \
  --finalize Data/Strings/mod_japanese.strings
```

#### Plugin pipeline (ESP/ESM/ESL)

```bash
cargo run -p xt_app --bin xt_batch -- \
  --load-plugin Data/mod.esp \
  --workspace-root /path/to/game \
  --importxml tr.xml \
  --finalize out/mod.esp
```

#### Dictionary build/apply

```bash
# build
cargo run -p xt_app --bin xt_batch -- \
  --generate-dictionary Data/Strings/Translations \
  --source english --target japanese \
  --dict-out dict.tsv

# apply
cargo run -p xt_app --bin xt_batch -- \
  --load base.xml \
  --importxml tr.xml \
  --dict-in dict.tsv \
  --finalize out.xml
```
