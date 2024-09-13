#!/usr/bin/env lua

-- hello world website
local new_script = [=[
local h = site.html

-- CSS for our site
-- site.css automatically minifies it
local style = site.css [[
html {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  font-family: sans-serif;
}
]]

-- Make the html page
-- building it like this minifies the html,
-- and h automatically adds the DOCTYPE
local page = h(
  h.style(style),
  h.title 'My website',
  h.div {
    h.h1 'Hello world!',
    h.img { class = 'logo', alt = 'logo', src = 'logo.svg' }
  }
)

-- emit our files to the final site
site.emit('index.html', page)
site.emit('logo.svg', site.logo)
]=]

local lfs = require 'lfs'
local generate = require 'generate'

-- TODO: path independent so require works

-- what to do
if arg[1] == 'dev' then
  print 'dev server'
elseif arg[1] == 'build' then
  print 'build the thing'
  lfs.chdir(arg[2])
  local out = generate.run()

  for k, v in pairs(out) do print(k, v) end
elseif arg[1] == 'api' then
  print 'api description'
elseif arg[1] == 'new' then
  local file
  if arg[2] then
    assert(lfs.mkdir(arg[2]), 'Failed to create directory ' .. arg[2])
    file = assert(io.open(arg[2] .. '/main.lua', 'w'), 'Failed to create file ' .. arg[2] .. '/main.lua')
    print('Made site in ' .. arg[2])
  else
    file = assert(io.open('./main.lua'), 'Failed to create file ./main.lua')
    print 'Made site in the current directory'
  end
  file:write(new_script)
  file:close()
else
  -- TODO: usage
  print 'slsg COMMAND [options]'
end
