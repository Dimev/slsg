-- HTML elements
local h = site.html

-- CSS for our site
local style = site.sass(site.read 'style.scss')

-- Syntax highlighter for our code
local highlighter = site.highlighter({
  start = {
    { token = 'comment', regex = [[--.*]] },
    { token = 'keyword', regex = [[print]] },
    { token = 'string',  regex = [[".*"]] }
  }
})

-- Article
local article = site.read 'article.lmk'

-- Functions to convert our article into html
local macros = {}
function macros:document(x)
  return h.main(x)
end

function macros:paragraph(x)
  return table.concat(x)
end

function macros:text(x)
  return h.p(x)
end

function macros:code(args, code)
  return h.code { h.pre { highlighter:highlight_html(code) } }
end

function macros:title(args) return h.h1(args) end

function macros:date(args) return h.pre(args) end

function macros:section(args) return h.h2(args) end

function macros:math(args) return site.latex_to_mathml(args) end

-- Compile our article with the functions we defined
local content = site.luamark_run(article, macros)

-- Make the html page
-- building it like this minifies the html,
-- and h automatically adds the DOCTYPE
local page = h {
  h.head {
    h.meta { charset = 'utf-8' },
    h.meta { name = 'viewport', content = 'width=device-width, initial-scale=1.0' },
    h.title 'My website',
    h.style(style),
  },
  h.body {
    h.div {
      class = 'main',
      h.img { class = 'logo', alt = 'logo', src = 'logo.svg' },
      content
    }
  }
}

-- emit our files to the final site
site.emit('index.html', page)
site.emit('logo.svg', site.logo)
