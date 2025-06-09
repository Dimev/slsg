local function page(args)
  -- this is the template we'll use
  local template = readfile 'templates/page.html'
      -- apply the templates we can here
      :gsub("@@title", args.title)
      :gsub("@@description", args.description)

  -- apply the template from the file we read
  return function(content)
    -- and apply the content
    return template:gsub("@@content", content)
  end
end

return { page = page }
