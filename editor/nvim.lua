local root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:gsub("^@", ""), ":h:h")
vim.opt.rtp:prepend(root)
require("herdr-smart-nav").setup()
