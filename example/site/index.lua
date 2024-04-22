local components = require 'components.lua'
local markdown = require 'markdown.lua'
local bib = require 'bibliography.lua'

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

    -- functions for the markdown renderer
    local markdownFns = { code = codeHighlight }

    -- render out
    local mdHtml = markdown.compileMd(md:ast(), markdownFns):renderHtml()
    local html = components.page(front.title, script.styles.style, pagelinks, rawHtml(mdHtml))

    -- and the actual page file
    mdPages[value.stem] = page()
      :withHtml(html:renderHtml())
  end
end

-- citation list
local citations = {}

-- bibliography
local bibtex = script.static.files['references.bib']:parseBibtex()

-- render out
local indexHtml = markdown.compileMd(
  script.colocated.files['index.md']
    :parseMd()
    :ast(), 
  { 
    code = codeHighlight,
    mdxTextExpressionSetup = function(ast, context)
      bib.addCitation(ast.value, citations, bibtex)
    end,
    mdxTextExpression = function(ast, context)
      return bib.renderCitation(ast.value, citations)
    end
  }
)

-- index page
local html = components.page(
  config.name, script.styles.style, pagelinks, h.main():sub(
    indexHtml,
    bib.generateBibHtml(bibtex)
  )
):renderHtml()

local notFoundPage = site.file(components.page(
  "LSSG - Not found", script.styles.style, pagelinks,
  h.main():sub(
    h.h1():sub("404!"),
    h.p():sub("Page not found!")
  )
):renderHtml())

return page()
  :withHtml(html)
  :withManyPages(mdPages)
  :withFile("404.html", notFoundPage)
  :withManyFiles(script.static.files)
