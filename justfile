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

clean:
	cargo clean
