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
