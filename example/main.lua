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
  -- table with all values
  local macros = {
    title = '',
  }

  -- text is wrapped in <p>
  function macros:text(args)
    return h.p(args)
  end

  -- paragraphs are concatenated from the results
  function macros:paragraph(args)
    return site.html_merge(args)
  end

  -- same with the resulting document
  function macros:document(args)
    return site.html_fragment(args)
  end

  -- add a title
  function macros:title(args)
    self.title = args
  end

  -- add an image
  function macros:img(path, alt)
    return h.div { class = 'imgblock', h.img { src = path, alt = alt } }
  end

  -- code block
  function macros:code(language, content)
    return h.pre {
      class = 'codeblock',
      h.code { languages[language]:highlight_html(content, 'code-') }
    }
  end

  -- inline code
  function macros:inline(args)
    return h.p { h.code { class = 'codeline', args } }
  end

  -- add math
  function macros:math(args)
    return site.compile_tex(args)
  end

  -- parse a luamark article
  local res = site.compile_luamark(article, macros)
  return h.main {
    class = 'main',
    h.h1(macros.title),
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
