-- parse our sass
local css = site.sass(site.read './style.scss')

-- shortcut
local h = site.html

-- make an example luamark parser
function parse(article)
  -- table with all values
  local macros = {
    title = '',
    text = h.p,
    paragraph = table.concat,
    document = table.concat,
  }

  -- add a title
  function macros:title(args)
    self.title = args
  end

  -- add an image
  function macros:img(args)
    return {

    }
  end

  -- parse a luamark article
  local res = site.luamark_run(article)
  return res
end

-- load the example article

-- make an example page
local html = h {
  h.html {
    h.head {
      h.style(css),
      h.title 'My Website',
      h.link { rel = 'icon', href = '/icon.svg' },
      h.meta { name = 'viewport', content = 'width=device-width, initial-scale=1.0' },
    },
    h.body {
      class = 'container',
      h.div {
        class = 'main',
        h.h1 'Hello, world!',
        h.p 'Edit the files to start making your site!',
        h.img { class = 'logo', alt = 'SLSG logo', src = 'logo.svg' },
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
