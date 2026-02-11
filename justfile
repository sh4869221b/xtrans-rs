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
	@if command -v dx >/dev/null 2>&1; then echo "dev: hotpatch mode"; dx serve --hotpatch --package xt_app --bin xt_app --features hotpatch; else echo "dev: dx not found, fallback to auto-restart mode"; just dev-restart; fi

dev-hotpatch:
	@if ! command -v dx >/dev/null 2>&1; then echo "dx is required. Install with: cargo install dioxus-cli"; exit 1; fi; dx serve --hotpatch --package xt_app --bin xt_app --features hotpatch

dev-restart:
	@if ! command -v cargo-watch >/dev/null 2>&1; then echo "cargo-watch is required. Install with: cargo install cargo-watch"; exit 1; fi; cargo watch --clear -w crates/xt_app/src -w crates/xt_core/src -w crates/xt_esp/src -x "run -p xt_app --bin xt_app"

clean:
	cargo clean
