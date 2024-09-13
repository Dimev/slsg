local h = site.html

-- CSS for our site
-- site.css automatically minifies it
local style = site.css [[
html {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  font-family: sans-serif;
}
]]

-- Make the html page
-- building it like this minifies the html,
-- and h automatically adds the DOCTYPE
--local page = h(
--  h.style(style),
--  h.title 'My website',
--  h.div {
--    h.h1 'Hello world!',
--    h.img { class = 'logo', alt = 'logo', src = 'logo.svg' }
--  }
--)

-- emit our files to the final site
site.emit('index.html', 'hello!')
site.emit('logo.svg', site.logo)
