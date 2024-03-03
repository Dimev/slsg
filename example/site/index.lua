-- all subpages
local pages = {}

-- run the script for the page
for key, val in pairs(template.colocated.scripts) do
  pages[key] = val() 
end

-- add the bibliography
local bib = template.colocated.files["references.bib"]:parseBibtex()

-- convert a table to a string
local function table2string(table, ident)
  if type(table) ~= "table" then
    return tostring(table)
  end

  local str = ""
  for key, value in pairs(table) do
    str = str .. string.rep("  ", ident or 0) .. key .. " = " .. table2string(value, (ident or 0) + 1) .. ",\n"
  end

  return "{\n" .. str .. " \n" .. string.rep("  ", ident or 0) .. "}"
end

-- render out like this for now
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
  :withFile("404.html", yassg.file(p():sub(txt("Not found!")):render()))
  :withFile("style.css", template.styles.style)
