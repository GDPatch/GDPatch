local utils = {}

function utils.escape(str, percent_only)
  -- the second argument to gsub (repl) only needs to escape percentages
  if percent_only then
    return (str:gsub("%%", "%%%%"))
  end

  -- https://github.com/lua-nucleo/lua-nucleo/blob/4f30d4178c31417e9df9b976a0f36b3157c3e3b5/lua-nucleo/string.lua#L245-L267
  return (str:gsub(".", {
    ["^"] = "%^",
    ["$"] = "%$",
    ["("] = "%(",
    [")"] = "%)",
    ["%"] = "%%",
    ["."] = "%.",
    ["["] = "%[",
    ["]"] = "%]",
    ["*"] = "%*",
    ["+"] = "%+",
    ["-"] = "%-",
    ["?"] = "%?",
    ["\0"] = "%z"
  }))
end

return utils
