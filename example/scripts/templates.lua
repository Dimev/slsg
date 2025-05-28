local function page(settings)
  local template = '<!DOCTYPE html><html><head><link rel="stylesheet" type="text/css" href="/style.css"><meta charset="utf-8"><meta content="width=device-width,initial-scale=1" name="viewport"><link rel="icon" type="image/svg+xml" href="/icon.svg"></head><body>@@content</body></html>'--readfile 'scripts/page.html'

  -- applies a template, and thus needs to run later
  return function(text)
    return template:gsub("@@content", text)
  end
end

return { page = page }
