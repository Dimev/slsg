-- parse our sass
local css = site.sass(site.read('./style.css'))

-- shortcut
local h = site.html

-- make an example page
local html = h {
  h.html {
    h.head {
      h.style(css),
      h.title 'My Website',
    },
    h.body {
      h.div {
        h.h1 'Hello, world!',
        h.img { class = 'logo', alt = 'SLSG logo', src = 'logo.svg' }
      }
    }
  }
}

-- emit it to the generator
site.emit('index.html', html)

-- emit the logo of SLSG to the generator
site.emit('logo.svg', site.logo)
