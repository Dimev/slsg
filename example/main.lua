-- parse our sass
local css = site.compile_sass(site.read './style.scss')

-- shortcut
local h = site.html

-- page templates
local templates = {}

-- make an example page
function templates:page(article)
  return h {
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
end

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


-- mark it as 404
-- this needs to happen after it is emitted
site.set_404 '404.html'
