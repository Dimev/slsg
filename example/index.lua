local css = [[
html {
	font-size: 200%;
	font-family: Latin Modern Roman;
}

math {
	font-family: Latin Modern Math;
}

.code {
	font-family: Latin Modern Mono, monospace;
}

.code--comment {
	color: #888;
	font-style: italic;
}

.code--keyword {
	color: #D00;
}

.code--macro {
	color: #D0D;
}

.code--constant {
	color: blue;
}

.code--type {
	color: purple;
}

.code--string {
	color: green;
}

.code--function {
	color: blue;
}
]]

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

local html = Fragment(
	h.title('hello!'),
	h.meta():attrs({ charset = 'UTF-8' }),
	h.style(css),
	h.h1('Hello world'),
	h.p('This is SLSG new!'),
	RawHtml(math),
	h.pre():attrs({ class = 'code' }):sub(RawHtml(code))
):renderHtml()

return { files = { ['index.html'] = html } }
