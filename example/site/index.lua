local components = require('components.lua')
local markdown = require('markdown.lua')
local bib = require('bibliography.lua')

-- get all index pages
local pagelinks = {}
for key, value in pairs(script.colocated.files) do
  -- if it's markdown, and not the index page, include it
  if value.extention == "md" and value.stem ~= "index" then 
    local front = value:parseMd():front()
    
    -- link to the page
    pagelinks["/" .. value.stem] = front.title 
  end
end

-- sort so the order is consistent across reloads
table.sort(pagelinks)

-- code highlighing for markdown
function codeHighlight(ast)
  if ast.language and site.highlightExists(ast.language) then
    local html = site.highlightCodeHtml(ast.value, ast.language, "code--")
    return h.pre():attrs({ class = "code" }):sub(rawHtml(html))
  else
    warn("no language " .. ast.language .. " to highlight")
    return h.pre():attrs({ class = "code"}):sub(ast.value)
  end
end

-- load all colocated markdown files
local mdPages = {}
for key, value in pairs(script.colocated.files) do
  -- if it's markdown, and not the index page, include it
  if value.extention == "md" and value.stem ~= "index" then
    -- parse the markdown
    local md = value:parseMd()

    -- get the front matter for the title of the page
    local front = md:front()

    -- render out
    local mdHtml = markdown.compileMd(md:ast(), { code = codeHighlight }):renderHtml()
    local html = components.page(front.title, "", "/style.css", pagelinks, rawHtml(mdHtml))

    -- and the actual page file
    mdPages[value.stem] = page()
      :withHtml(html:renderHtml())
  end
end

-- render out
local indexHtml = markdown.compileMd(
  script.colocated.files['index.md']
    :parseMd()
    :ast(), 
  { code = codeHighlight }
)

-- index page
local html = components.page(
  config.name, "", "/style.css", pagelinks, h.main():sub(
    indexHtml,
    bib.generateBibHtml(script.static.files["references.bib"])
  )
):renderHtml()

local notFoundPage = site.file(components.page(
  "LSSG - Not found", "", "/style.css", pagelinks,
  h.main():sub(
    h.h1():sub("404!"),
    h.p():sub("Page not found!")
  )
):renderHtml())

return page()
  :withHtml(html)
  :withManyPages(mdPages)
  :withFile("style.css", script.styles.style)
  :withFile("404.html", notFoundPage)
  :withManyFiles(script.static.files)
