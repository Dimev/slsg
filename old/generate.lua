local api = require 'api'

local generate = {}

function generate.run(dev)
  -- site API table
  local site = {
    dev,
    logo = api.logo,
    escape_html = api.escape_html,
    unescape_html = nil,
  }

  -- files to emit
  local emitted = {}

  -- emit a file
  function site.emit(path, content)
    emitted[path] = content
  end

  -- css minify
  function site.css(css)
    return css
  end

  -- logo
  site.logo = 'sus amogus'

  -- run the script
  local env = setmetatable({ site = site }, { __index = _G })
  local chunk = assert(loadfile('./main.lua', 't', env))
  chunk()

  -- return the emitted files
  return emitted
end

return generate
