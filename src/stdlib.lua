site = {}

-- escape html
function site.escape_xml(xml)
  local subst = {
    ["&"] = "&amp;",
    ["<"] = "&lt;",
    [">"] = "&gt;",
  }
  local res = string.gsub(xml, '.', subst)
  return res
end

-- escape a html quote ("x" and 'x")
function site.escape_xml_quote(xml)
  local subst = {
    ['"'] = "&quot;",
    ["'"] = "&#39;",
  }
  local res = string.gsub(xml, '.', subst)
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

-- render an xml element
local function xml_render(elem, void)
  local attrs = {}
  local elems = ''

  for key, value in pairs(elem.attrs) do
    table.insert(attrs, site.escape_xml(key) .. '="' .. site.escape_xml_quote(value) .. '"')
  end

  for i = 1, #elem.elems do
    if type(elem.elems[i]) == 'table' then
      -- render the element
      elems = elems .. elem.elems[i]:render()
    elseif type(elem.elems[i]) == 'nil' then
      -- do nothing
    elseif type(elem.elems[i]) == 'function' then
      -- call the function to render the element
      elems = elems .. elem.elems[i]():render()
    else
      -- no escape, we accept html in text form here
      elems = elems .. elem.elems[i]
    end
  end

  if not elem.elem then
    -- fragment
    return elems
  elseif void[elem.elem] then
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

-- create an xml fragment
function site.xml_fragment(elems)
  return {
    attrs = {},
    elems = elems,
    render = function(self) return xml_render(self, {}) end,
  }
end

-- create an html fragment
function site.html_fragment(elems)
  return {
    attrs = {},
    elems = elems,
    render = function(self) return xml_render(self, void_elements) end,
  }
end

-- create an html element
local function xml_element(elem, content, void)
  -- if we get a string, put it inside an element with no attributes
  -- Also render it raw
  if type(content) == 'string' then
    content = { content }
  elseif type(content) == 'function' then
    content = content()
  end

  local attrs = {}
  local elems = {}

  for key, value in pairs(content) do
    -- skip if the key is not a string, as that means it's an index on the list
    if type(key) == 'string' then attrs[key] = value end
  end

  for i = 1, #content do
    if void[elem] then
      -- void elements cannot have children, so crash if it does
      error('Void element `' .. elem .. '` cannot have content')
    elseif type(content[i]) == 'string' then
      -- escape string content
      table.insert(elems, site.escape_xml(content[i]))
    else
      table.insert(elems, content[i])
    end
  end

  return {
    elem = elem,
    elems = elems,
    attrs = attrs,
    render = function(self) return xml_render(self, void) end,
  }
end


-- create an html element from a table
-- pairs are the attributes, ipairs are the children
site.xml = {}

local xml_meta = {}
function xml_meta:__call(elems)
  if type(elems) == 'table' then
    local res = ''
    for i = 1, #elems do
      if type(elems[i]) == 'table' then
        res = res .. elems[i]:render()
      else
        res = res .. site.escape_xml('' .. elems[i])
      end
    end
    return res
  else
    return {
      elems = elems,
      attrs = {},
      render = function() return elems end
    }
  end
end

function xml_meta:__index(element)
  return function(content)
    return xml_element(element, content, {})
  end
end

-- add nothing
function xml_meta:__newindex()
end

-- meta table for this to work
setmetatable(site.xml, xml_meta)

-- create an html element from a table
-- pairs are the attributes, ipairs are the children
site.html = {}

local html_meta = {}
function html_meta:__call(elems)
  if type(elems) == 'table' then
    local res = ''
    for i = 1, #elems do
      if type(elems[i]) == 'table' then
        res = res .. elems[i]:render()
      else
        res = res .. site.escape_xml('' .. elems[i])
      end
    end
    return '<!DOCTYPE html>' .. res
  else
    return {
      elems = elems,
      attrs = {},
      render = function() return elems end
    }
  end
end

function html_meta:__index(element)
  return function(content)
    return xml_element(element, content, void_elements)
  end
end

-- add nothing
function html_meta:__newindex()
end

-- meta table for this to work
setmetatable(site.html, html_meta)

-- TODO: xml

-- SLSG logo
local svg = site.xml
site.logo = svg {
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
site.icon = svg {
  svg.svg {
    version = '1.1',
    width = '100',
    height = '100',
    xmlns = 'http://www.w3.org/2000/svg',
    svg.circle { cx = 50, cy = 50, r = 50, fill = '#1D2951' },
    svg.circle { cx = 65, cy = 35, r = 15, fill = 'white' },
  }
}
