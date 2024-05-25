local mod = {}

function Titlebar(links)
  local pagelinks = {}
  for key, value in pairs(links) do
    table.insert(
      pagelinks,
      { key,
        h.a(value):attrs({ class = "titlelink", href = key })
      }
    )
  end

  -- sort them to be in alphabetical order
  local p2 = {}
  table.sort(pagelinks, function(l, r) return l[1] < r[1] end)

  for _, value in ipairs(pagelinks) do
    table.insert(p2, value[2])
  end

  return h.nav():attrs({ class = "titlebar" }):sub(
    h.a("LSSG"):attrs({ class = "titlelink", href = "/" }),
    Fragment(table.unpack(p2))
  )
end

function mod.page(title, css, links, body)
  return Fragment(
  -- header
    h.meta():attrs({ charset = "UTF-8" }),
    h.meta():attrs({ content = "width=device-width,initial-scale=1", name = "viewport" }),
    h.link():attrs({ rel = "icon", href = "icon.svg" }),
    h.style(css),
    h.title(title),
    -- body, in main section
    Titlebar(links),
    h.div()
    :attrs({ class = "content" })
    :sub(body)
  )
end

return mod
