-- Builtin functions
-- functionality for making pages and html
-- This is reloaded for every script, so that warnings work mostly correctly

-- escape html
function site.escapeHtml(html)
  local subst = {
    ["&"] = "&amp;",
    ['"'] = "&quot;",
    ["'"] = "&apos;",
    ["<"] = "&lt;",
    [">"] = "&gt;",
  }
  local escaped = string.gsub(html, ".", subst)
  return escaped
end

-- file tree for a single directory
function Filetree()
  local tree = {
    files = {},
  }

  -- Add an index.html file
  function tree:withHtml(html)
    self.files['index.html'] = html

    return self
  end

  -- Add a file to the tree
  function tree:withFile(name, content)
    self.files[name] = content

    return self
  end

  -- Add an entire subtree to the file tree
  function tree:withSubtree(directory, subtree)
    for name, file in pairs(subtree.files) do
      -- add the concatted path
      self.files[site.concatPath(directory, name)] = file
    end

    return self
  end

  -- Add a filepath to use when a page is not found
  -- This only works if it's the root
  function tree:withNotFoundPath(path)
    self.notFound = path
  
    return self
  end

  return tree
end

-- make a node
function El(ty, void, ...)
  local element = {
    tag = ty,
    attributes = "",
    content = "",
  }

  -- add attributes
  function element:attrs(props)
    for key, value in pairs(props) do
      -- append
      self.attributes = self.attributes .. " " .. site.escapeHtml(key) .. '="' .. site.escapeHtml(value) .. '"'
    end
    return self
  end

  -- add content
  function element:sub(...)
    assert(not void, 'Elements of type "' .. self.tag .. '" cannot have children, as they are a void element')
    for _, value in pairs({ ... }) do
      if type(value) == "string" then
        -- If it's text, surround it by spaces
        self.content = self.content .. ' ' .. site.escapeHtml(value) .. ' '
      elseif value ~= nil then
        self.content = self.content .. value:render()
      end
    end
    return self
  end

  -- render to html
  function element:renderHtml()
    -- we are html, so include this
    return "<!DOCTYPE html>" .. self:render()
  end

  -- render ourselves
  function element:render()
    if void then
      return "<" .. self.tag .. self.attributes .. ">"
    end
    return "<" .. self.tag .. self.attributes .. ">"
        .. self.content
        .. "</" .. self.tag .. ">"
  end

  -- add initial attributes, if allowed
  if not void then element = element:sub(...) end

  return element
end

-- raw html
function RawHtml(text)
  return {
    renderHtml = function() return "<!DOCTYPE html>" .. text end,
    renderself = function() return text end,
    render = function() return text end
  }
end

-- fragment
function Fragment(...)
  local html = ""
  for _, value in pairs({ ... }) do
    if type(value) == "string" then
      html = html .. value
    else
      html = html .. value:render()
    end
  end

  return {
    renderHtml = function() return "<!DOCTYPE html>" .. html end,
    renderself = function() return html end,
    render = function() return html end
  }
end

-- raw text
function Txt(t)
  return {
    renderHtml = function() return "!<DOCTYPE html>" .. t end,
    renderself = function() return t end,
    render = function() return t end
  }
end

-- make a node function
-- void tags do not need a closing or /> at the end of them
local voidTags = {
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
  source = true,
  track = true,
  wbr = true
}

-- make an element
local function mkEl(ty)
  return function(...) return El(ty, voidTags[ty] ~= nil, ...) end
end

-- make an element accepting raw text
local function mkRw(ty)
  return function(c) return El(ty, voidTags[ty] ~= nil, RawHtml(c)) end
end

-- collection of all nodes
-- see https://developer.mozilla.org/en-US/docs/Web/HTML/Element
-- we put them in the h table, to not fill global scopeH

-- TODO no lsp complain
h = {}

-- root
h.html = mkEl('html')

-- metadata
h.base = mkEl('base')
h.head = mkEl('head')
h.link = mkEl('link')
h.meta = mkEl('meta')
h.style = mkRw('style')
h.title = mkEl('title')

-- sectioning root
h.body = mkEl('body')

-- content sectioning
h.address = mkEl('address')
h.article = mkEl('article')
h.aside = mkEl('aside')
h.footer = mkEl('footer')
h.h1 = mkEl('h1')
h.h2 = mkEl('h2')
h.h3 = mkEl('h3')
h.h4 = mkEl('h4')
h.h5 = mkEl('h5')
h.h6 = mkEl('h6')
h.hgroup = mkEl('hgroup')
h.main = mkEl('main')
h.nav = mkEl('nav')
h.section = mkEl('section')
h.search = mkEl('search')

-- text content
h.blockquote = mkEl('blockquote')
h.dd = mkEl('dd')
h.div = mkEl('div')
h.dl = mkEl('dl')
h.dt = mkEl('dt')
h.figcaption = mkEl('figcaption')
h.figure = mkEl('figure')
h.hr = mkEl('hr')
h.li = mkEl('li')
h.menu = mkEl('menu')
h.ol = mkEl('ol')
h.p = mkEl('p')
h.pre = mkEl('pre')
h.ul = mkEl('ul')

-- inline text semantics
h.a = mkEl('a')
h.abbr = mkEl('abbr')
h.b = mkEl('b')
h.bdi = mkEl('bdi')
h.bdo = mkEl('bdo')
h.br = mkEl('br')
h.cite = mkEl('cite')
h.code = mkEl('code')
h.data = mkEl('data')
h.dfn = mkEl('dfn')
h.em = mkEl('em')
h.i = mkEl('i')
h.kbd = mkEl('kbd')
h.mark = mkEl('mark')
h.q = mkEl('q')
h.rp = mkEl('rp')
h.rt = mkEl('rt')
h.ruby = mkEl('ruby')
h.s = mkEl('s')
h.samp = mkEl('samp')
h.small = mkEl('small')
h.span = mkEl('span')
h.strong = mkEl('strong')
h.sub = mkEl('sub')
h.sup = mkEl('sup')
h.time = mkEl('time')
h.u = mkEl('u')
h.var = mkEl('var')
h.wbr = mkEl('wbr')

-- image and multimedia
h.area = mkEl('area')
h.audio = mkEl('audio')
h.img = mkEl('img')
h.map = mkEl('map')
h.track = mkEl('track')
h.video = mkEl('video')

-- embedded content
h.embed = mkEl('embed')
h.iframe = mkEl('iframe')
h.object = mkEl('object')
h.picture = mkEl('picture')
h.portal = mkEl('portal')
h.source = mkEl('source')

-- svg and mathml
-- note that these aren't included
h.svg = mkEl('svg')
h.math = mkEl('math')

-- scripting
h.canvas = mkEl('canvas')
h.noscript = mkEl('noscript')
h.script = mkRw('script')

-- demarcating edits
h.del = mkEl('del')
h.ins = mkEl('ins')

-- table content
h.caption = mkEl('caption')
h.col = mkEl('col')
h.colgroup = mkEl('colgroup')
h.table = mkEl('table')
h.tbody = mkEl('tbody')
h.td = mkEl('td')
h.tfoot = mkEl('tfoot')
h.th = mkEl('th')
h.thead = mkEl('thead')
h.tr = mkEl('tr')

-- forms
h.button = mkEl('button')
h.datalist = mkEl('datalist')
h.fieldset = mkEl('fieldset')
h.form = mkEl('form')
h.input = mkEl('input')
h.label = mkEl('label')
h.legend = mkEl('legend')
h.meter = mkEl('meter')
h.optgroup = mkEl('optgroup')
h.options = mkEl('options')
h.output = mkEl('output')
h.progress = mkEl('progress')
h.select = mkEl('select')
h.textarea = mkEl('textarea')

-- interactive elements
h.details = mkEl('details')
h.dialog = mkEl('dialog')
h.summary = mkEl('summary')

-- web components
h.slot = mkEl('slot')
h.template = mkEl('template')

-- obsolete elements are not included
