local site = ...
--[[
local templates = {
  page = site.read_template "templates/page.html",
  index = site.read_template "templates/index.html",
}

local pages = {}

for path in site.glob "**/*.md" do
  local front, html = site.markdown(site.read(path))
  local res = templates[front.template](html)
  table.append(pages, res)
end

return { pages = pages }

OR

local templates = {
  page = site.read_template "templates/page.html"  
}

local pages = {}
for path, page in site.pages do
  local html = site.render(page, path, site.md)
  local res = templates[page.template](html)
  pages[page.path] = res
end

return { pages = pages }
]]

local pages = {}

for _, page in pairs(site.pages) do
  local html, err = site.render(page.content, 'sus', {
    highlight = function(lang, code) return code end,
    math_display = function() return "" end,
    math_inline = function() return "" end,
  })
end

-- TODO not sure if this is the best way to do it? 
-- return the generated site
return {
  pages = pages,
  not_found = "404.html"
}
