print("Hello from content/blog/index.lua!")
for k, v in pairs(directories) do
  print(k, v)
end

return { type = "page", meta = {}, html = "<!doctype html>hello", subs = directories }
