-- parse our sass
local css = site.compile_sass(site.read './style.scss')

-- shortcut
local h = site.html

-- syntax highlighting
local languages = {
  luamark = site.create_highlighter {
    start = {
      { token = 'comment', regex = '%.*' },
      { token = 'macro',   regex = [[@\w+]] },
    }
  },
  lua = site.create_highlighter {
    start = {
      { token = 'string',   regex = [=[\[\[.*\]\]]=] },
      { token = 'function', regex = [[\w+\s*(?=\[)]] }
    }
  }
}

-- make an example luamark parser
local function parse(article)
  -- title
  local title = ''

  -- table with all values
  local macros = {
    text = h.p,                        -- wrap in <p>
    paragraph = site.html_merge,       -- concatenate tags from the results
    document = site.html_fragment,     -- same, but don't concatenate tags
    title = function(t) title = t end, -- set the title
    math = site.compile_tex,           -- compile tex to mathml
  }

  -- add an image
  function macros.img(path, alt)
    return h.div { class = 'imgblock', h.img { src = path, alt = alt } }
  end

  -- code block
  function macros.code(language, content)
    return h.pre {
      class = 'codeblock',
      h.code { languages[language]:highlight_html(content, 'code-') }
    }
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

-- make an example 404 page
local not_found = h {
  h.html {
    h.head {
      h.title 'Not found!',
      h.link { rel = 'icon', href = '/icon.svg' },
      h.meta { charset = 'utf-8' },
      h.meta { name = 'viewport', content = 'width=device-width, initial-scale=1.0' },
      h.style(css),
    },
    h.body {
      class = 'container',
      -- page not found
      h.main {
        class = 'main',
        h.h1 'Page not found!',
        h.div { class = 'imgblock', h.img { src = '/logo.svg', alt = 'SLSG logo' } },
        h.p {
          'see ',
          h.a {
            href = '/index.html',
            h.code { class = 'inline', 'index.html' },
          },
          ' instead',
        }
      }
    }
  }
}

-- emit it to the generator
site.emit('index.html', html)

-- emit the logo of SLSG to the generator
site.emit('logo.svg', site.logo)

-- and the icon
site.emit('icon.svg', site.icon)

-- emit a 404
site.emit('404.html', not_found)

-- mark it as 404
-- this needs to happen after it is emitted
site.set_404 '404.html'
