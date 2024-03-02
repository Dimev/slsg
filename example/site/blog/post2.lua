-- svg illustration
local html = div()
  :sub(
    txt("Hello world!")
  )
  :render()

return page()
  :withHtml(html)
