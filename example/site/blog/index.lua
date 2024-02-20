print("=== from blog ===")
print("-- files --")

local pages = {}

for key, val in pairs(template.colocated.files) do 
  print(key, val)
  pages[key] = page()
    :withHtml(
      div():sub(
        txt(val:parseTxt())
      ):render()
    )
end

print("-- directories --")
for key, val in pairs(template.colocated.directories) do 
  print(key, val)
end

print("-- scripts --")
for key, val in pairs(template.colocated.scripts) do 
  print(key, val)
  val()
end

return page()
  :withHtml("pronto sbinotto")
  :withManyPages(pages)
