-- Builtin functions
-- functionality for making pages and html
-- This is reloaded for every script, so that warnings work mostly correctly

-- escape html
function escapeHtml(html)
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

-- make a page
function page() 
	local table = {
		-- no html to start off with
		html = nil,
		files = {},
		pages = {},
	}

 	-- set the html
	function table:withHtml(html) 
		-- check if it's a string
		assert(type(html) == "string", "The provided html is not a string, did you forget to call `:render()` ?")	
		self.html = html
		return self
	end

	-- add a file
	function table:withFile(path, file)
		assert(type(file) == "userdata", "The provided file is not userdata, are you sure you picked the right file?")
		self.files[path] = file
		return self
	end

	-- add a page
	function table:withPage(path, page)
		assert(type(page) == "table", "The provided page is not a table, did you forget to make a page?")
		self.pages[path] = page
		return self
	end

	-- add many files
	function table:withManyFiles(files) 
		for key, value in pairs(files) do
			assert(type(value) == "userdata", 'The provided file "' .. key .. '" is not a userdata, did you forget to make a file?')
			self:withFile(key, value)
		end
		
		return self
	end

	-- add many pages
	function table:withManyPages(pages) 
		for key, value in pairs(pages) do 
			assert(type(value) == "table", 'The provided page "' .. key .. '" is not a table, did you forget to make a page?')
			self:withPage(key, value)
		end

		return self
	end
	
	return table
end

-- make a node
function el(ty, void) 
	local element =  {
		tag = ty,
		attributes = "",
		content = "",
	}

	-- add attributes
	function element:attrs(props)
		for key, value in pairs(props) do 
			-- append
			self.attributes = self.attributes .. " " .. escapeHtml(key) .. '="' .. escapeHtml(value) .. '"'
		end
		return self 
	end

	-- add content
	function element:sub(...)
		assert(not void, 'Elements of type "' .. self.tag .. '" cannot have children, as they are a void element')
		for _, value in ipairs({ ... }) do
			if type(value) == "string" then  
				self.content = self.content .. escapeHtml(value)
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

	return element
end

-- raw html
function rawHtml(text) 
	return {
		renderHtml = function() return "<!DOCTYPE html>" .. text end,
		renderself = function() return text end,
		render = function() return text end
	}
end

-- fragment
function fragment(...)
	local html = ""
	for _, value in ipairs({...}) do
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

-- make a node function
-- void tags do not need a closing or /> at the end of them
local voidTags = { 
	area = true, base = true, br = true, col = true, embed = true, hr = true, img = true, 
	input = true, link = true, meta = true, source = true, track = true, wbr = true 
}

-- make an element
local function mkEl(ty) 
	return function() return el(ty, voidTags[ty] ~= nil) end
end 

-- make an element accepting text
local function mkTx(ty)
	return function(c) return el(ty, voidTags[ty] ~= nil):sub(c) end
end

-- make an element accepting raw text
local function mkRw(ty)
	return function(c) return el(ty, voidTags[ty] ~= nil):sub(rawHtml(c)) end
end

-- collection of all nodes
-- see https://developer.mozilla.org/en-US/docs/Web/HTML/Element
-- we put them in the h table, to not fill global scope
h = {}

-- TODO: special element functionality

-- root
h.html = mkEl('html')

-- metadata
h.base = mkEl('base')
h.head = mkEl('head')
h.link = mkEl('link')
h.meta = mkEl('meta')
h.style = mkRw('style')
h.title = mkTx('title')

-- sectioning root
h.body = mkEl('body')

-- content sectioning
h.address = mkEl('address')
h.article = mkEl('article')
h.aside = mkEl('aside')
h.footer = mkEl('footer')
h.h1 = mkTx('h1')
h.h2 = mkTx('h2')
h.h3 = mkTx('h3')
h.h4 = mkTx('h4')
h.h5 = mkTx('h5')
h.h6 = mkTx('h6')
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
h.p = mkTx('p')
h.pre = mkTx('pre')
h.ul = mkEl('ul')

-- inline text semantics
h.a = mkTx('a')
h.abbr = mkTx('abbr')
h.b = mkTx('b')
h.bdi = mkTx('bdi')
h.bdo = mkTx('bdo')
h.br = mkEl('br')
h.cite = mkTx('cite')
h.code = mkTx('code')
h.data = mkTx('data')
h.dfn = mkTx('dfn')
h.em = mkTx('em')
h.i = mkTx('i')
h.kbd = mkTx('kbd')
h.mark = mkTx('mark')
h.q = mkTx('q')
h.rp = mkTx('rp')
h.rt = mkTx('rt')
h.ruby = mkTx('ruby')
h.s = mkTx('s')
h.samp = mkTx('samp')
h.small = mkTx('small')
h.span = mkTx('span')
h.strong = mkTx('strong')
h.sub = mkTx('sub')
h.sup = mkTx('sup')
h.time = mkTx('time')
h.u = mkTx('u')
h.var = mkTx('var')
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
h.noscript = mkTx('noscript')
h.script = mkRw('script')

-- demarcating edits
h.del = mkTx('del')
h.ins = mkTx('ins')

-- table content
h.caption = mkTx('caption')
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
h.button = mkTx('button')
h.datalist = mkEl('datalist')
h.fieldset = mkEl('fieldset')
h.form = mkEl('form')
h.input = mkEl('input')
h.label = mkTx('label')
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
