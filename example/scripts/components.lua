local mod = {}

function titlebar(links)
  local pagelinks = {}
  for key, value in pairs(links) do
    table.insert(
      pagelinks, 
      { key, 
        h.a()
          :attrs({ class = "titlelink", href = key })
          :sub(value)
      }
    )
  end

  -- sort them to be in alphabetical order
  local p2 = {}
  table.sort(pagelinks, function (l, r) return l[1] < r[1] end)
  
  for key, value in ipairs(pagelinks) do
    table.insert(p2, value[2])
  end
  
  return h.nav():attrs({ class = "titlebar" }):sub(
    h.a()
      :attrs({ class = "titlelink", href = "/" })
      :sub("LSSG"),
    fragment(table.unpack(p2))
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
