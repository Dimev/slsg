-- all subpages
local pages = {}

-- run the script for the page
for key, val in pairs(template.colocated.scripts) do
  pages[key] = val() 
end

-- the html index
local html = div():sub(
  h1():sub(
    txt("Hello world!")
  ),
  a("blog/"):sub(
    -- we only have this as page
    txt("Blog posts")
  ) 
):render()

return page()
  :withHtml(html)
  :withManyPages(pages)
  :withFile("style.css", template.styles.style)
