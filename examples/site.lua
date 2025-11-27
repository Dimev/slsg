local site = ...
local etlua = require 'etlua'
local pages = {}

for _, page in pairs(site.pages) do
  print(page.content)
  local html, err = etlua.render(page.content, {
    highlight = function(lang, code) return code end,
    math_display = function() return "" end,
    math_inline = function() return "" end,
  })
  print(html, err)
end

-- TODO not sure if this is the best way to do it? 
-- return the generated site
return {
  pages = pages,
  not_found = "404.html"
}
