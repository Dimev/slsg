-- parse our sass
local css = site.compile_sass(site.read './style.scss')

-- shortcut
local h = site.html

-- make an example luamark parser
local function parse(article)
  -- title of the article
  local title = ''

  -- table with all values
  local macros = {
    text = h.p,                        -- wrap in <p>
    paragraph = site.html_merge,       -- concatenate tags from the results
    document = site.html_fragment,     -- same, but don't concatenate tags
    title = function(t) title = t end, -- set the title
  }

  -- add an image
  function macros.img(path, alt)
    return h.img { src = path, alt = alt }
  end

  -- inline code
  function macros.inline(args)
    return h.p { h.code { class = 'codeline', args } }
  end

  -- parse a luamark article
  local res = site.compile_luamark(article, macros)
  return h.main {
    class = 'main',
    h.h1(title),
    res
  }
end

-- load the example article
local article = parse(site.read 'article.lmk')

-- make an example page
local html = h {
  h.html {
    h.head {
      h.title 'My Website',
      h.link { rel = 'icon', href = '/icon.svg' },
      h.meta { charset = 'utf-8' },
      h.meta { name = 'viewport', content = 'width=device-width, initial-scale=1.0' },
      h.style(css),
    },
    h.body {
      class = 'container',
      -- emit the article we made
      article
    }
  }
}

-- emit it to the generator
site.emit('index.html', html)

-- emit the logo of SLSG to the generator
site.emit('logo.svg', site.logo)

-- and the icon
site.emit('icon.svg', site.icon)
