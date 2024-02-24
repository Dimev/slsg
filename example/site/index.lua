-- all subpages
local pages = {}

-- run the script for the page
for key, val in pairs(template.colocated.scripts) do
  pages[key] = val() 
end

-- add the bibliography
local bib = template.colocated.files["references.bib"]:parseBibtex()

-- render it out to text 
local function table2string(table)
  if type(table) ~= "table" then
    return tostring(table)
  end

  local str = ""
  for key, value in pairs(table) do
    str = str .. key .. " = " .. table2string(value) .. ",\n"
  end

  return "{" .. str .. "}"
end
local citations = table2string(bib)

-- the html index
local html = div():sub(
  h1():sub(
    txt("Hello world!")
  ),
  p():sub(txt("Welcome to " .. config.name)),
  a("blog/"):sub(
    -- we only have this as page
    txt("Blog posts")
  ),
  el("pre"):sub(txt("citations: " .. citations))
):render()

return page()
  :withHtml(html)
  :withManyPages(pages)
  :withFile("style.css", template.styles.style)
