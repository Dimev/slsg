-- ignore our template files
ignorefiles 'templates/*'

-- functions we can use
local mod = {}

-- page template
function mod.page(args)
  -- this is the template we'll use
  local template = readfile 'templates/page.html'
      -- apply the templates we can here
      :gsub("@@title", args.title or "")
      :gsub("@@description", args.description or "")

  -- apply the template from the file we read
  return function(content)
    -- and apply the content
    return template:gsub("@@content", content)
  end
end

return mod
