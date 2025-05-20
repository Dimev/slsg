local function page(text)
  local template = readfile 'scripts/page.html'
  return template:gsub("@@content", text)
end
