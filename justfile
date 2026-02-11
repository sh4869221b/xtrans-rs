set dotenv-load := false

default: help

help:
	@just --list

status:
	@git status --short

build:
	cargo build

build-app:
	cargo build -p xt_app

build-core:
	cargo build -p xt_core

build-esp:
	cargo build -p xt_esp

check:
	cargo check

test:
	cargo test

test-app:
	cargo test -p xt_app

test-core:
	cargo test -p xt_core

test-esp:
	cargo test -p xt_esp

license-report:
	cargo license --json

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features

serve:
	cargo run -p xt_app --bin xt_app

dev:
	@DX_BIN=$(command -v dx 2>/dev/null || true); \
	if [ -n "$DX_BIN" ] && [ -x "$DX_BIN" ]; then \
		if ! command -v ld >/dev/null 2>&1; then echo "ld is required for hotpatch build. Install binutils."; exit 1; fi; \
		echo "dev: hotpatch mode"; \
		PATH="/usr/bin:/bin:$PATH" "$DX_BIN" serve --hot-patch --package xt_app --bin xt_app --features hotpatch; \
	else echo "dev: dx not found, fallback to auto-restart mode"; just dev-restart; fi

dev-hotpatch:
	@DX_BIN=$(command -v dx 2>/dev/null || true); \
	if [ -z "$DX_BIN" ] || [ ! -x "$DX_BIN" ]; then echo "dx is required. Install with: cargo install dioxus-cli"; exit 1; fi; \
	if ! command -v ld >/dev/null 2>&1; then echo "ld is required for hotpatch build. Install binutils."; exit 1; fi; \
	PATH="/usr/bin:/bin:$PATH" "$DX_BIN" serve --hot-patch --package xt_app --bin xt_app --features hotpatch

dev-restart:
	@if ! command -v cargo-watch >/dev/null 2>&1; then echo "cargo-watch is required. Install with: cargo install cargo-watch"; exit 1; fi; cargo watch --clear -w crates/xt_app/src -w crates/xt_core/src -w crates/xt_esp/src -x "run -p xt_app --bin xt_app"

clean:
	cargo clean
