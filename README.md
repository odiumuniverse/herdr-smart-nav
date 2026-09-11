# herdr-smart-nav

One key family across four levels. `Ctrl+h/j/k/l` moves through, in order:

1. **nvim windows** — inside Neovim, `:wincmd h/j/k/l` first (`editor/nvim.lua`);
2. **herdr panes** — at the nvim edge, or directly in non-vim panes;
3. **herdr tabs** — at the pane strip end (no wrap);
4. **herdr workspaces** ("spaces") — at the tab strip end (wraps around).

`vim-tmux-navigator` ported to herdr, then extended past panes. Written in Rust:
no `jq`, no Python, no daemons — a single binary talking to the herdr CLI.

## How it works

Two cooperating sides, one shared path. The binary exposes two entrypoints:

- `herdr-smart-nav <dir>` — full chain, bound to herdr keys. It checks the
  focused pane's *foreground* process via `herdr pane process-info`. If it's
  Vim/Neovim it forwards the chord with `herdr pane send-keys` (nvim then owns
  level 1); otherwise it runs `cross`.
- `herdr-smart-nav cross <dir>` — levels 2–4 only: `pane focus`, and on
  `changed=false` the adjacent tab (scoped to the pane's workspace), then the
  adjacent workspace.

`editor/nvim.lua` maps the same keys to try the nvim window first and, at the
edge, shell out to `cross`. Outside herdr the keys stay inside nvim.

Vim detection matches `name`, `argv0` and `argv[0]` (herdr's `name` field alone
is unreliable) against `^g?(view|l?n?vim?x?)(diff)?$`.

## Requirements

- herdr `>= 0.7.0`
- Rust toolchain **at install time** (`cargo build --release` runs during
  `herdr plugin install`; runtime needs nothing but the herdr CLI)
- Linux or macOS

## Install

```sh
herdr plugin install odiumuniverse/herdr-smart-nav
```

### 1. Bind the keys in herdr

Add to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "ctrl+h"
type = "plugin_action"
command = "smart-nav.left"
description = "smart navigate left"

[[keys.command]]
key = "ctrl+j"
type = "plugin_action"
command = "smart-nav.down"
description = "smart navigate down"

[[keys.command]]
key = "ctrl+k"
type = "plugin_action"
command = "smart-nav.up"
description = "smart navigate up"

[[keys.command]]
key = "ctrl+l"
type = "plugin_action"
command = "smart-nav.right"
description = "smart navigate right"
```

Then `herdr server reload-config`.

### 2. Wire up Neovim

Without this, level 1 is skipped: in a vim pane the key still reaches nvim
(forwarded), but the nvim edge won't continue into tabs/workspaces.

With lazy.nvim (the module self-registers; `opts` may stay empty):

```lua
{
  "odiumuniverse/herdr-smart-nav",
  build = "cargo build --release",
  opts = {},
}
```

With vim-plug:

```vim
Plug 'odiumuniverse/herdr-smart-nav', { 'do': 'cargo build --release' }
```

```lua
require("herdr-smart-nav").setup()
```

No plugin manager: copy the repo's `lua/herdr-smart-nav.lua` somewhere on
your runtimepath and call `setup()`, or source `editor/nvim.lua` (does both).

`setup({ keymaps = false })` skips the `<C-h/j/k/l>` maps and only defines
the `:SmartNavLeft/Down/Up/Right` commands. `setup({ bin = "..." })` pins
the binary; otherwise the repo build, the herdr-managed build,
`~/.cargo/bin` and `PATH` are tried in order.

Remove or disable any other `<C-h/j/k/l>` owners (`vim-tmux-navigator`
mappings, `devxplay/herdr.nvim`, hand-rolled `<C-w>h` maps) — only one owner
per chord, otherwise the last-loaded wins silently. Verify with
`:verbose nmap <C-h>`.

The lua side needs the release binary (`target/release/herdr-smart-nav`,
built by `plugin install` or `make build`), falling back to
`~/.cargo/bin/herdr-smart-nav` and `PATH`.

## Configuration

Non-Vim TUIs that own `Ctrl+h/j/k/l` themselves (lazygit, k9s, …) can opt
into key forwarding like Vim. Set where you launch herdr:

```sh
export HERDR_NAV_PASSTHROUGH_RE='^(lazygit|k9s)$'
```

Unlike Vim, these apps don't cross *out* at an edge — use the prefixed
`prefix+h/j/k/l` to leave the pane. Regex matched against the lower-cased
process basename; invalid regex disables passthrough safely.

## Notes & tradeoffs

- **`Ctrl+l` / `Ctrl+k` in shells.** Binding these globally shadows readline's
  `Ctrl+L` (clear) and `Ctrl+K` (kill line) in non-Vim panes. Same tradeoff as
  `vim-tmux-navigator`. Prefer `alt+h/j/k/l` if you want those back.
- **`Ctrl+h` vs Backspace.** Same byte (`0x08`) unless the kitty keyboard
  protocol is active. Neovim ≥ 0.10 enables it automatically in herdr panes.
- **Old herdr without `focus.changed`.** Treated as "moved" — degrades to
  classic 2-level navigation instead of misfiring tab switches.

## Development

```sh
make test   # 21 tests: 12 unit + 9 integration (stub herdr CLI, no server needed)
make lint   # fmt + clippy + lua syntax
make build  # release binary
```
