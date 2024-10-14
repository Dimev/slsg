local h = site.html

-- CSS for our site
-- site.css automatically minifies it
local style = [[
html {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  font-family: sans-serif;
}
]]

local article = site.read 'article.luamark'
local x = {}
function x:document(x)
  print(x)
  return h.main(x)
end

function x:paragraph(x)
  return 'mogu' -- table.concat(x)
end

function x:code(args, code)
  return args .. '\n' .. h.pre(code)
end

function x:title(args) return h.h1(args) end

function x:date(args) return h.pre(args) end

function x:section(args) return h.h2(args) end

function x:text(args) return h.p(args) end

local content = site.luamark_run(article, x)

-- Make the html page
-- building it like this minifies the html,
-- and h automatically adds the DOCTYPE
local page = h {
  h.style(style),
  h.title 'My website',
  h.div {
    h.h1 'Hello world!',
    h.img { class = 'logo', alt = 'logo', src = 'logo.svg' },
    content
  }
}

-- emit our files to the final site
site.emit('index.html', page)
site.emit('logo.svg', site.logo)
