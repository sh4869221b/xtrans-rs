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
