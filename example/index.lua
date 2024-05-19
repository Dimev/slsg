local css = site.parseSass(
	site.openFile('styles/style.scss'):readString()
)

local math = site.latex2Mathml("\\int_0^1 x dx")

site.addHighlighters([[
[funlang]
keyword = '\<(fun|when|is|then)\>'
comment = '--.*'
]])

local code = site.highlightCodeHtml('funlang', [[
-- fibbonachi sequence
fun fibbonachi n = when n
  is 0 then 0
  is 1 then 1
  is n then fibbonachi (n - 1) (n - 2)
]], 'code--')

local content = h.div():attrs({ class = 'content' }):sub(
	h.h1('Hello world'),
	h.p('This is SLSG new!'),
	RawHtml(math),
	h.pre():attrs({ class = 'code' }):sub(RawHtml(code))
)

local html = Fragment(
	h.title('hello!'),
	h.meta():attrs({ charset = 'UTF-8' }),
	h.style(css),
	content
):renderHtml()

local static = Filetree()

-- load all static files
for name, path in pairs(site.listFiles('static')) do
	static:withFile(name, site.openFile(path))
end

return Filetree()
		:withHtml(html)
		:withFile('404.html', html)
		:withNotFoundPath('404.html')
		:withSubtree('.', static)
