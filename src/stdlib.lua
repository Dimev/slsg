local api = {}

-- Development mode
api.dev = internal.dev

-- read files and directories
api.dir = internal.dir
api.read = internal.read

-- file names
api.filename = internal.filename
api.filestem = internal.filestem
api.fileext = internal.fileext

-- emit files to the site generator
-- TODO: proper
-- we can probably try and get a table to put these in passed in
api.emit = internal.emit or print
api.emitfile = internal.emit_file or print
api.emitcommand = internal.emit_command or print

-- latex to mathml
api.latex_to_mathml = internal.latex_to_mathml

-- TODO
-- minification (css)
-- parser
-- highlighting

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
function api.unescape_html(html)
  local subst = {
    ["&amp;"] = "&",
    ['&quot;'] = '"',
    ["&apos;"] = "'",
    ["&lt;"] = "<",
    ["&gt;"] = ">",
  }
  return string.gsub(html, '.', subst)
end

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
api.html = {}

-- TODO: deal with string values

local html_meta = {}
function html_meta:__call(element)
  return '<!DOCTYPE html>' .. table.concat(element)
end

function html_meta:__index(element)
  return function(inside)
    -- if we get a string, put it inside an element with no styling
    if type(inside) == 'string' and void_elements[element] then
      error 'Cannot have a void element with content'
    elseif type(inside) == 'string' then
      return '<' .. element .. '>' .. api.escape_html(inside) .. '</' .. element .. '>'
    end

    local attributes = {}
    local elements = {}

    for key, value in pairs(inside) do
      if type(key) == 'string' then
        table.insert(attributes, key .. '="' .. value .. '"')
      end
    end

    for _, value in ipairs(inside) do
      table.insert(elements, value)
    end

    -- <open>inner</end>
    local open = '<' .. element .. ((#attributes > 0 and ' ') or '') .. table.concat(attributes, ' ') .. '>'
    local inner = table.concat(elements)
    local close = '</' .. element .. '>'

    -- no closing tag if we are a void element
    if void_elements[element] and #elements > 0 then
      error 'Cannot have a void element with content'
    elseif void_elements[element] then
      return open .. inner
    else
      return open .. inner .. close
    end
  end
end

-- add nothing
function html_meta:__newindex()
end

-- meta table for this to work
setmetatable(api.html, html_meta)

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
</svg>]]

return api
