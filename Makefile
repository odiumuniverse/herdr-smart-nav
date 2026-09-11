.PHONY: test lint build

test:
	cargo build
	cargo test
	nvim --headless --noplugin --cmd "set rtp+=." +"lua require('herdr-smart-nav').setup()" +"lua assert(next(vim.fn.maparg('<C-h>', 'n', false, true)) ~= nil, 'C-h missing')" +qa
	nvim --headless --noplugin --cmd "set rtp+=." +"lua require('herdr-smart-nav').setup({ keymaps = false })" +"lua assert(next(vim.fn.maparg('<C-h>', 'n', false, true)) == nil, 'C-h leak')" +qa

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	command -v luac >/dev/null && luac -p editor/nvim.lua || echo "luac not installed, skipping lua check"

build:
	cargo build --release
