-- Manually convert markdown to HTML, according to commonmark
-- TODO
local function table2string(table, ident)
  if type(table) ~= "table" then
    return tostring(table)
  end

  local str = ""
  for key, value in pairs(table) do
    str = str .. string.rep("  ", ident or 0) .. key .. " = " .. table2string(value, (ident or 0) + 1) .. ",\n"
  end

  return "{\n" .. str .. " \n" .. string.rep("  ", ident or 0) .. "}"
end

