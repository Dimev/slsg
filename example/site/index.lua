print("=== from site ===")
print("-- files --")
for key, val in pairs(template.colocated.files) do 
  print(key, val)
end

print("-- directories --")
for key, val in pairs(template.colocated.directories) do 
  print(key, val)
end

print("-- scripts --")
local pages = {}
for key, val in pairs(template.colocated.scripts) do 
  print(key, val)
  pages[key] = val()
end

local html = div():sub(
  h1():sub(txt("Hello world!"))
):render()

return page()
  :withHtml(html)
  :withManyFiles(template.colocated.files)
  :withManyPages(pages)
