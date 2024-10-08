local api = {}

-- Development mode
api.dev = internal.dev

-- read files and directories
api.dir = internal.dir
api.read = internal.read

-- file names
api.file_name = internal.file_name
api.file_stem = internal.file_stem
api.file_ext = internal.file_ext

-- where to output files to
-- provided from the rust side, but is removed before the site is run
local out = output

-- emit files to the site generator
function api.emit(path, data)
  out[path] = { type = 'data', data = data }
end

function api.emit_file(path, original)
  out[path] = { type = 'file', original = original }
end

function api.emit_command(path, command, ...)
  out[path] = {
    type = 'command',
    command = command,
    arguments = { ... }
  }
end

-- latex to mathml
api.latex_to_mathml = internal.latex_to_mathml

-- sass
api.sass = internal.sass

-- luamark
api.luamark_ast = internal.luamark_ast
api.luamark_run = internal.luamark_run

-- TODO
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

local html_meta = {}
function html_meta:__call(element)
  return '<!DOCTYPE html>' .. table.concat(element)
end

function html_meta:__index(element)
  return function(inside)
    -- if we get a string, put it inside an element with no styling
    if type(inside) == 'string' and void_elements[element] then
      error('Cannot have a void (' .. element .. ') element with content')
    elseif type(inside) == 'string' then
      return '<' .. element .. '>' .. api.escape_html(inside) .. '</' .. element .. '>'
    end

    local attributes = {}
    local elements = {}

    for key, value in pairs(inside) do
      if type(key) == 'string' then
        table.insert(attributes, api.escape_html(key) .. '="' .. api.escape_html(value) .. '"')
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

-- Same, but for generic xml (atom, svg etc)
-- aka without void elements
api.xml = {}

local xml_meta = {}
function xml_meta:__call(element)
  return table.concat(element)
end

function xml_meta:__index(element)
  return function(inside)
    if type(inside) == 'string' then
      return '<' .. element .. '>' .. api.escape_html(inside) .. '</' .. element .. '>'
    end

    local attributes = {}
    local elements = {}

    for key, value in pairs(inside) do
      if type(key) == 'string' then
        table.insert(attributes, api.escape_html(key) .. '="' .. api.escape_html(value) .. '"')
      end
    end

    for _, value in ipairs(inside) do
      table.insert(elements, value)
    end

    -- <open>inner</end>
    local open = '<' .. element .. ((#attributes > 0 and ' ') or '') .. table.concat(attributes, ' ') .. '>'
    local inner = table.concat(elements)
    local close = '</' .. element .. '>'

    return open .. inner .. close
  end
end

-- add nothing
function xml_meta:__newindex()
end

-- meta table for this to work
setmetatable(api.xml, xml_meta)

-- SLSG logo
local svg = api.xml
api.logo = svg {
  svg.svg {
    version = '1.1',
    width = '210',
    height = '100',
    xmlns = 'http://www.w3.org/2000/svg',
    svg.circle { cx = 50, cy = 50, r = 50, fill = '#1D2951' },
    svg.circle { cx = 65, cy = 35, r = 15, fill = 'white' },
    svg.g {
      fill = '#1D2951',
      ["font-family"] = 'monospace',
      ["font-size"] = 18,
      ["font-weight"] = 'bold',
      svg.text { x = 100, y = 20, 'Scriptable' },
      svg.text { x = 110, y = 45, 'Lua' },
      svg.text { x = 110, y = 70, 'Site' },
      svg.text { x = 100, y = 90, 'Generator' },
    }
  }
}

-- Icon version, without the text
api.icon = svg {
  svg.svg {
    version = '1.1',
    width = '100',
    height = '100',
    xmlns = 'http://www.w3.org/2000/svg',
    svg.circle { cx = 50, cy = 50, r = 50, fill = '#1D2951' },
    svg.circle { cx = 65, cy = 35, r = 15, fill = 'white' },
  }
}

return api
