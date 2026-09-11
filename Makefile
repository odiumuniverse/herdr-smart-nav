.PHONY: test lint build

test:
	cargo build
	cargo test

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	command -v luac >/dev/null && luac -p editor/nvim.lua || echo "luac not installed, skipping lua check"

build:
	cargo build --release
