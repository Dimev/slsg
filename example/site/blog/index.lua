print("=== from blog ===")
print("-- files --")
for key, val in pairs(template.colocated.files) do 
  print(key, val)
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
