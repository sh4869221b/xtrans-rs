# xtrans-rs

## Purpose

This project aims to re-implement the xTranslator workflow in Rust with a Dioxus desktop UI.
The target is workflow compatibility, not a byte-for-byte clone of the original application.

## Current Scope

The current implementation intentionally focuses on the following workflows only:

1. Dictionary-based auto translation (build/apply dictionary)
2. XML-based bulk translation import/apply (including xTranslator XML compatibility)

Current UI operation for these workflows:

- Dictionary build: `翻訳 > 辞書を構築`
- Quick AutoTranslate: `翻訳 > Quick自動翻訳` or `Ctrl-R` (selected row only)
- XML bulk apply: `ファイル > 翻訳XMLを一括適用` or XML drag-and-drop on the window

Everything outside these workflows is partial, experimental, or not implemented yet.

## Legal and Attribution

- This project is an independent implementation and is not affiliated with xTranslator or its maintainers.
- Development follows a clean-room approach focused on workflow compatibility.
- The upstream xTranslator project is licensed under MPL-2.0: <https://github.com/MGuffin/xTranslator/blob/main/LICENSE>
- This repository is licensed under MIT (`LICENSE`).
- Third-party crate licenses are tracked in `THIRD_PARTY_NOTICES.md`.

# Development

This workspace is split into a core library and a Dioxus desktop app.

## Quick Commands (just)

Common tasks are available via `just` at the repo root:

```bash
just test-core
just test-esp
just build-app
just serve
```

## Batch Workflow (`xt_batch`)

Run batch commands from repo root:

```bash
cargo run -p xt_app --bin xt_batch -- --load base.xml --importxml tr.xml --finalize out.xml
```

### Strings pipeline

```bash
cargo run -p xt_app --bin xt_batch -- \
  --load-strings Data/Strings/mod_english.strings \
  --importxml tr.xml \
  --finalize Data/Strings/mod_japanese.strings
```

### Plugin pipeline (ESP/ESM/ESL)

```bash
cargo run -p xt_app --bin xt_batch -- \
  --load-plugin Data/mod.esp \
  --workspace-root /path/to/game \
  --importxml tr.xml \
  --finalize out/mod.esp
```

### Dictionary build/apply

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

```
project/
├─ crates/
│  ├─ xt_core/ # Core library (format parsing, TM, validation, etc.)
│  │  ├─ src/
│  │  └─ tests/fixtures/
│  └─ xt_app/ # Dioxus desktop app
│     ├─ assets/
│     ├─ src/main.rs
│     └─ Dioxus.toml
├─ Cargo.toml # Workspace definition
```

### Automatic Tailwind (Dioxus 0.7+)

As of Dioxus 0.7, there no longer is a need to manually install tailwind. Simply `dx serve` and you're good to go!

Automatic tailwind is supported by checking for a file called `tailwind.css` in your app's manifest directory (next to `Dioxus.toml`). To customize the file, use `Dioxus.toml`:

```toml
[application]
tailwind_input = "my.css"
tailwind_output = "assets/out.css" # also customize the location of the out file!
```

### Tailwind Manual Install

To use tailwind plugins or manually customize tailwind, you can can install the Tailwind CLI and use it directly.

### Tailwind
1. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
2. Install the Tailwind CSS CLI: https://tailwindcss.com/docs/installation/tailwind-cli
3. Run the following command in `crates/xt_app` to start the Tailwind CSS compiler:

```bash
npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css --watch
```

### Serving Your App

Run the following command in `crates/xt_app` to start developing with the default platform:

```bash
dx serve --platform desktop
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```
