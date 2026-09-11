local DIRS = { h = "left", j = "down", k = "up", l = "right" }

local function repo_root()
  local src = debug.getinfo(1, "S").source:gsub("^@", "")
  return vim.fn.fnamemodify(src, ":h:h")
end

local function nav_bin()
  local rel = repo_root() .. "/target/release/herdr-smart-nav"
  if vim.fn.executable(rel) == 1 then
    return rel
  end
  for _, p in ipairs({ vim.fn.expand("~/.cargo/bin/herdr-smart-nav"), "herdr-smart-nav" }) do
    if vim.fn.executable(p) == 1 then
      return p
    end
  end
  return nil
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

for key, dir in pairs(DIRS) do
  local cmd = "SmartNav" .. dir:sub(1, 1):upper() .. dir:sub(2)
  pcall(vim.api.nvim_del_user_command, cmd)
  vim.api.nvim_create_user_command(cmd, function()
    go(key, dir)
  end, { desc = "Smart navigate " .. dir })
  vim.keymap.set("n", "<C-" .. key .. ">", function()
    go(key, dir)
  end, { silent = true, desc = "Smart navigate " .. dir })
  vim.keymap.set("t", "<C-" .. key .. ">", "<C-\\><C-n><cmd>" .. cmd .. "<cr>", {
    silent = true,
    desc = "Smart navigate " .. dir,
  })
end
