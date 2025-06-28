# SLSG
Scriptable Lua Site Generator

## TODO
- [ ] Fix fennel's function scopes, so they can be used acros scripts
- [ ] Project site
- [ ] Template for both fennel and lua (index, single post, rss feed)
- [ ] `new` command that uses one of the templates

## Instead of templating, there is lua (or fennel)
Write your files in normal markdown or html. When a file has a \*.lua.\* or \*.fnl.\*
in it's extension, it will be processed. In this casy, any `<? ... ?>` is interpreted
as lua or fennel, depending on the double extension In markdown files, this is done by
replacing any inline and block html that forms a processing instruction.

Any number or string that is returned from `<? ... ?>` expressions are added into the
resulting file.

If a function or table is returned, this function is called after templating is
done, with the entire text in the file. The resulting string from the function is
then used as the new file content, or is called again if it is a function or table.

## TeX Math
Any `$...$` and `$$...$$` in markdown files are interpreted as TeX math, and converted
to mathml.

## Syntax highlighting
Any code block in markdown is highlighted using syntect. Other languages can be loaded
via the config file.

## File renaming
Any `[name].htm` and `[name].html` files are automatically renamed to
`[name]/index.htm` and `[name]/index.html`

## Also, font subsetting!
Any `*.ttf` or `*.otf` font can be subset, by changing the extension to `*.subset.ttf`
or `*.subset.otf`

## Available functions
The following functions and variables are available from lua and fennel:
- ```lua
  dev = true
  ```
  Set to true if run with the development server, set to false otherwise
- ```lua
  function mathml(tex, inline) end
  ```
  Compile the given tex to mathml. If inline is true, the resulting mathml is
  inline instead of block.
- ```lua
  function highlight(language, code, prefix) end
  ```
  Highlight the given code

## Config file
This can all be controlled with the `site.conf` config file, which has the following
defaults:
```ini
[build]
output = dist/
# setup = script.lua # setup script, run before processing

[ignore]
# scripts/* # files to ignore when building the site

[dev]
# not-found = 404.html # page to use as 404 when developing

[font]
subset = true # whether to subset fonts
# extra = abc # add these characters as extra to subset
```
