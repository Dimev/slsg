-- ignore our template files
ignorefiles 'templates/*'

-- funtions we can use in <? ... ?>
local mod = {}

-- page template
function mod.page(args)
  -- template we'll use
  local template = readfile 'templates/page.html'
      :gsub("@@title", args.title)             -- insert title
      :gsub("@@description", args.description) -- description

  -- insert the processed file into the template
  return function(content)
    return template:gsub("@@content", content)
  end
end

return mod
