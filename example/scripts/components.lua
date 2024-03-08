local mod = {}

function titlebar(links)
  return h.div():attrs({ class = "titlebar" }):sub(
    h.a():attrs({ href = "/" }):sub("YASSG"),
    links
  )
end

function mod.page(title, description, css, links, body)
  return h.html():sub(
    h.head():sub(
      -- header
      h.meta():attrs({ charset = "UTF-8" }),
      h.title():sub(title),
      h.link():attrs({ rel = "stylesheet", href = css })
    ),
    -- links and title page

    -- body, in main section
    h.body():sub(titlebar(links), h.div():sub(body))
  )
end

return mod
