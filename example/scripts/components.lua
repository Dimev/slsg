local mod = {}

function titlebar(links)
  local pagelinks = {}
  for key, value in pairs(links) do
    table.insert(
      pagelinks, 
      h.a()
        :attrs({ class = "titlelink", href = key })
        :sub(value)
    )
  end
  
  return h.nav():attrs({ class = "titlebar" }):sub(
    h.a()
      :attrs({ class = "titlelink", href = "/" })
      :sub("YASSG"),
    fragment(table.unpack(pagelinks))
  )
end

function mod.page(title, description, css, links, body)
  return h.html():sub(
    h.head():sub(
      -- header
      h.meta():attrs({ charset = "UTF-8" }),
      h.meta():attrs({ content = "width=device-width,initial-scale=1", name="viewport"}),
      h.title():sub(title),
      h.link():attrs({ rel = "stylesheet", href = css })
    ),
    -- links and title page

    -- body, in main section
    h.body():sub(titlebar(links), h.div():attrs({ class = "content" }):sub(body))
  )
end

return mod
