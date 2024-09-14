local api = {}

-- Development mode
api.dev = internal.dev

-- read files and directories
api.dir = internal.dir
api.read = internal.read

-- emit files to the site generator
api.emit = internal.emit
api.emit_file = internal.emit_file
api.emit_command = internal.emit_command

-- TODO: escaping
-- minification (css)
-- html
-- mathml
-- files
-- parser
-- highlighting

-- html
-- void elements don't need closing tags as they can't have children
local void_elements = {
  area = true,
  base = true,
  br = true,
  col = true,
  embed = true,
  hr = true,
  img = true,
  input = true,
  link = true,
  meta = true,
  param = true,
  source = true,
  track = true,
  wbr = true,
}

-- create an html element from a table
-- pairs are the attributes, ipairs are the children
function api.h(type)
  return function(elem)
    -- TODO

    return '<' .. type .. '>' .. '</' .. type .. '>'
  end
end

-- add the doctype around html
function api.html(html)
  return '<!DOCTYPE html>' .. html
end

-- escape html
function api.escape_html(html)
  local subst = {
    ["&"] = "&amp;",
    ['"'] = "&quot;",
    ["'"] = "&apos;",
    ["<"] = "&lt;",
    [">"] = "&gt;",
  }
  return string.gsub(html, '.', subst)
end

-- unescape html
-- TODO

-- SLSG logo
api.logo = [[
<svg version="1.1" width="210" height="100" xmlns="http://www.w3.org/2000/svg">
<circle cx="50" cy="50" r="50" fill="#1D2951" />
<circle cx="65" cy="35" r="15" fill="white" />
<g fill="#1D2951" font-family="monospace" font-size="18" font-weight="bold">
  <text x="100" y="20">Scriptable</text>
  <text x="110" y="45">Lua</text>
  <text x="110" y="70">Site</text>
  <text x="100" y="95">Generator</text>
</g>
</svg>
]]

return api
