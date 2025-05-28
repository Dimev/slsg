local function page(settings)
  local template = readfile 'templates/page.html'

  -- applies a template, and thus needs to run later
  return function(text)
    return template:gsub("@@content", text)
  end
end

return { page = page }
