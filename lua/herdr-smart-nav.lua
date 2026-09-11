local M = {}

M.config = { keymaps = true, bin = nil }

local DIRS = { h = "left", j = "down", k = "up", l = "right" }

local function repo_root()
  local src = debug.getinfo(1, "S").source:gsub("^@", "")
  return vim.fn.fnamemodify(src, ":h:h")
end

local function newest(paths)
  local best, best_time = nil, -1
  for _, p in ipairs(paths) do
    if vim.fn.executable(p) == 1 then
      local t = vim.fn.getftime(p)
      if t > best_time then
        best, best_time = p, t
      end
    end
  end
  return best
end

local function nav_bin()
  if M.config.bin then
    return M.config.bin
  end
  local own = repo_root() .. "/target/release/herdr-smart-nav"
  if vim.fn.executable(own) == 1 then
    return own
  end
  local managed = vim.fn.glob(
    vim.fn.expand("~/.config/herdr/plugins/github/smart-nav-*/target/release/herdr-smart-nav"),
    false,
    true
  )
  return newest(managed)
    or (vim.fn.executable(vim.fn.expand("~/.cargo/bin/herdr-smart-nav")) == 1 and vim.fn.expand(
      "~/.cargo/bin/herdr-smart-nav"
    ) or nil)
    or (vim.fn.executable("herdr-smart-nav") == 1 and "herdr-smart-nav" or nil)
end

local function in_herdr()
  return vim.env.HERDR_ENV == "1"
end

local function go(wincmd, dir)
  local win = vim.api.nvim_get_current_win()
  vim.cmd.wincmd(wincmd)
  if vim.api.nvim_get_current_win() ~= win then
    return
  end
  if not in_herdr() then
    return
  end
  local bin = nav_bin()
  if bin then
    vim.system({ bin, "cross", dir }, { text = true }):wait()
  end
end

local function apply()
  for key, dir in pairs(DIRS) do
    local cmd = "SmartNav" .. dir:sub(1, 1):upper() .. dir:sub(2)
    pcall(vim.api.nvim_del_user_command, cmd)
    vim.api.nvim_create_user_command(cmd, function()
      go(key, dir)
    end, { desc = "Smart navigate " .. dir })
    if M.config.keymaps then
      vim.keymap.set("n", "<C-" .. key .. ">", function()
        go(key, dir)
      end, { silent = true, desc = "Smart navigate " .. dir })
      vim.keymap.set("t", "<C-" .. key .. ">", "<C-\\><C-n><cmd>" .. cmd .. "<cr>", {
        silent = true,
        desc = "Smart navigate " .. dir,
      })
    end
  end
end

function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  apply()
  vim.api.nvim_create_autocmd("User", {
    group = vim.api.nvim_create_augroup("HerdrSmartNav", { clear = true }),
    pattern = "VeryLazy",
    callback = apply,
  })
end

return M
