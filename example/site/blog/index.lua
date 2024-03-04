-- all pages we want to include
local pages = {}

-- all pages to add as links
local links = {}

-- load all markdown files 
for key, val in pairs(template.colocated.files) do 
  -- only work on markdown 
  if val.extention == "md" then
    local md = val:parseMd()
    local front = md.front
    local mdhtml = md.html
    -- make the page
    local html = div():sub(
      h1():sub(
        -- name of the post
        txt("Post: " .. front.title)
      ),
      p():sub(
        -- the actual post
        txt(mdhtml)
      )
    ):render()

    -- add it to the pages we want
    pages[val.stem] = page():withHtml(html)

    -- and the links
    links[val.stem] = val.stem
  end
end

-- run all script that also add pages
for key, val in pairs(template.colocated.scripts) do
  pages[key] = val()
  links[key] = key
end

-- links to all pages
local pageLinks = {}

for key, val in pairs(links) do
  table.insert(
    pageLinks, 
    a(key .. "/"):sub(
      txt(val)
    )
  )
end

local code = [[
-- I have no IO monad and I must scream
scream :: String -> String
scream s = seq a $ unsafePerformIO a 
]]

-- index page for this
local index = div():sub(
  table.unpack(pageLinks),
  el("pre"):sub(txt(yassg.highlight(code, "haskell", "susmogus")))
):render()

-- add our own page
return page()
  :withHtml(index)
  :withManyPages(pages)
