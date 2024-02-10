-- load the posts
local posts = {}

for key, value in pairs(colocatedFiles) do 
  -- if it's a markdown post, keep it
  if value.extention == "md" then 
    -- load it

    -- make the page
    local post = page():withHtml(p():sub(txt("Hello")))

    -- add it to the posts
    posts[value.stem] = post
  end
end

-- make the page
local html = p():attrs({ class = "main" }):sub(
  txt("Hello world!"),
    div():sub(
      txt("This is a div!"),
      txt("With some text in it!")
    )
  )

-- return the rendered page
return page()
  :withMeta({ x = 1 })
  :withMeta({ y = 1})
  :withHtml(html)
  :withSubs(posts)
