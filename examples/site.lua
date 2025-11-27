local site = ...
local pages = {}

for _, page in pairs(site.pages) do
  print(page.content)
end

-- TODO not sure if this is the best way to do it? 
-- return the generated site
return {
  pages = pages,
  not_found = "404.html"
}
