local function page(content)
  -- this is the template we'll use
  local template = readfile 'templates/page.html'

  -- apply the template from the file we read
  return template:gsub("@@content", content)
end

return { page = page }
