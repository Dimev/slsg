local internal, output = ...
local api = {}

-- internal is provided from the rust side
-- Development mode
api.dev = internal.dev

-- read files and directories
api.dirs = internal.dirs
api.files = internal.files
api.read = internal.read
api.dir_exists = internal.dir_exists
api.file_exists = internal.file_exists

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

function api.set_404(path)
  if not api.dev then return end
  if not out[path] then error('404 page `' .. path .. '` not emitted yet!') end
  out[internal.long_404_path] = out[path]
end

-- latex to mathml
api.compile_tex = internal.compile_tex

-- sass
api.compile_sass = internal.compile_sass

-- luamark
api.compile_luamark = internal.compile_luamark

-- syntax highlighting
api.create_highlighter = internal.create_highlighter


-- escape html
function api.escape_html(html)
  local subst = {
    ["&"] = "&amp;",
    ['"'] = "&quot;",
    ["'"] = "&apos;",
    ["<"] = "&lt;",
    [">"] = "&gt;",
  }
  local res = string.gsub(html, '.', subst)
  return res
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

-- render an html element
function api.html_render(elem)
  local attrs = {}
  local elems = ''

  for key, value in pairs(elem.attrs) do
    table.insert(attrs, api.escape_html(key) .. '="' .. api.escape_html(value) .. '"')
  end

  for _, value in ipairs(elem.elems) do
    if type(value) == 'table' then
      elems = elems .. value:render()
    else
      -- no escape, we accept html in text form here
      elems = elems .. value
    end
  end

  if not elem.elem then
    -- fragment
    return elems
  elseif void_elements[elem.elem] then
    -- <open>
    return '<' .. elem.elem .. (#attrs > 0 and ' ' or '')
        .. table.concat(attrs, ' ') .. '>'
  else
    -- <open>inner<close>
    return '<' .. elem.elem .. (#attrs > 0 and ' ' or '')
        .. table.concat(attrs, ' ') .. '>'
        .. elems
        .. '</' .. elem.elem .. '>'
  end
end

-- create an html fragment
function api.html_fragment(elems)
  return {
    attrs = {},
    elems = elems,
    render = api.html_render,
  }
end

-- merge a list of html elements into a fragment
-- this means any consecutive elements with the same style will be merged into one longer element
function api.html_merge(elems)
  -- fast path if empty
  if #elems == 0 then
    return {
      attrs = {}, elems = {}, render = api.html_render
    }
  end

  local merged = {}
  for _, value in ipairs(elems) do
    if #merged == 0 then
      -- empty merged, add it
      table.insert(merged, value)
    elseif value.elem == merged[#merged].elem then
      -- same, merge attributes
      for k, v in pairs(value.attrs) do merged[#merged].attrs[k] = v end
      -- merge elements
      for _, v in ipairs(value.elems) do table.insert(merged[#merged].elems, v) end
    else
      -- different, just add
      table.insert(merged, value)
    end
  end

  return {
    attrs = {},
    elems = merged,
    render = api.html_render,
  }
end

-- create an html element
function api.html_element(elem, content)
  -- if we get a string, put it inside an element with no styling
  if type(content) == 'string' then
    content = { api.escape_html(content) }
  end

  local attrs = {}
  local elems = {}

  for key, value in pairs(content) do
    -- skip if the key is not a string, as that means it's an index on the list
    if type(key) == 'string' then attrs[key] = value end
  end

  for _, value in ipairs(content) do
    if void_elements[elem] then
      -- void elements cannot have children, so crash if it does
      error('Void element `' .. elem .. '` cannot have content')
    else
      table.insert(elems, value)
    end
  end

  return {
    elem = elem,
    elems = elems,
    attrs = attrs,
    render = api.html_render,
  }
end

-- create an html element from a table
-- pairs are the attributes, ipairs are the children
api.html = {}

local html_meta = {}
function html_meta:__call(elems)
  local res = ''
  for _, value in ipairs(elems) do
    if type(value) == 'table' then
      res = res .. value:render()
    else
      res = res .. api.escape_html('' .. value)
    end
  end
  return '<!DOCTYPE html>' .. res
end

function html_meta:__index(element)
  return function(content)
    return api.html_element(element, content)
  end
end

-- add nothing
function html_meta:__newindex()
end

-- meta table for this to work
setmetatable(api.html, html_meta)

-- Same, but for generic xml (atom, svg etc)
-- aka without void elements

-- render an xml element
function api.xml_render(elem)
  local attrs = {}
  local elems = ''

  for key, value in pairs(elem.attrs) do
    table.insert(attrs, api.escape_html(key) .. '="' .. api.escape_html(value) .. '"')
  end

  for _, value in ipairs(elem.elems) do
    if type(value) == 'table' then
      elems = elems .. value:render()
    else
      -- no escape, we accept xml in text form here
      elems = elems .. value
    end
  end

  if not elem.elem then
    -- fragment
    return elems
  else
    -- <open>inner<close>
    return '<' .. elem.elem .. (#attrs > 0 and ' ' or '')
        .. table.concat(attrs, ' ') .. '>'
        .. elems
        .. '</' .. elem.elem .. '>'
  end
end

-- create an xml fragment
function api.xml_fragment(elems)
  return {
    attrs = {},
    elems = elems,
    render = api.xml_render,
  }
end

-- create an xml element
function api.xml_element(elem, content)
  -- if we get a string, put it inside an element with no styling
  if type(content) == 'string' then
    content = { api.escape_html(content) }
  end

  local attrs = {}
  local elems = {}

  for key, value in pairs(content) do
    -- skip if the key is not a string, as that means it's an index on the list
    if type(key) == 'string' then attrs[key] = value end
  end

  for _, value in ipairs(content) do
    -- no need too deal with void elements
    table.insert(elems, value)
  end

  return {
    elem = elem,
    elems = elems,
    attrs = attrs,
    render = api.xml_render,
  }
end

api.xml = {}

local xml_meta = {}
function xml_meta:__call(elems)
  local res = ''
  for _, value in ipairs(elems) do
    if type(value) == 'table' then
      res = res .. value:render()
    else
      res = res .. api.escape_html('' .. value)
    end
  end
  return res
end

function xml_meta:__index(element)
  return function(content)
    return api.xml_element(element, content)
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
