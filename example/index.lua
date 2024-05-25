local comps = require 'scripts/components.lua'

-- parse the CSS
local css = site.parseSass(site.readFile('styles/style.scss'))

-- Make all pages
local pages = {}
for name, path in pairs(site.listFiles('site')) do
	-- parse it and make it a page
	local page = comps.page(name, css, { mogus = '/index.html' }, RawHtml(site.readFile(path)))

	pages[name] = page
end

local html = Fragment(
	h.title('hello!'),
	h.meta():attrs({ charset = 'UTF-8' }),
	h.style(css)
):renderHtml()

local static = Filetree()

-- load all static files
for name, path in pairs(site.listFiles('static')) do
	static:withFile(name, site.openFile(path))
end

return Filetree()
		:withHtml(pages['index.lua']:renderHtml())
		:withFile('404.html', html)
		:withNotFoundPath('404.html')
		:withSubtree('.', static)
