print("Hello from content/index.lua!")
for k, v in pairs(directories) do
  print(k, v)
end


return { type = "page", meta = {}, html = "<!doctype html>index", subs = { posts = directories.blog } }
