local lfs = require 'lfs'

local api = {}

-- TODO: escaping
-- minification (css)
-- html
-- mathml
-- files
-- parser

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

-- file system functions
api.dir = lfs.dir -- TODO: iter over dir
api.read = nil

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
