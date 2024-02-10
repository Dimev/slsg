local pages = {}
for key, value in pairs(colocatedFiles) do 
  if value.type == "page" then 
    pages[key] = value
  end
end

return page()
  :withHtml(p():sub(txt("Hello")))
  :withSubs(pages)
