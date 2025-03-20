# SLSG
Scriptable Lua Site Generator

## How it works (Soon:tm:):
Generated site is output to `dist/`.
The project root is scanned for luamark (`*.lmk`) files.
`name.lmk` and `name/index.lmk` are treated the same.
`main.lua` is then run using the meta information from the files found.
`main.lua` is expected to output the table of templates to use. A template is
a function that has `self` as state, and must return html.

## Templates:
Templates are methods called on a page when used. The page has the following properties:
- `meta`: the meta information given in the page.
- `path`: path of the current page, without index.lmk
- `context`: An empty table that is reused within the page.
- `adjacent`: Table of (path, { meta, posts }) for adjacent pages.
- `root`: Table of (path, { meta, posts }) for the root page.

## Api:
Other API functions are provided
- `load_highlighters(path)`: load syntect highlighters in the given path
- `highlight(lang, theme, code)`: highlight the given code with the language and theme
- `compile_tex(tex, inline)`: compile tex to MathML, block (default) or inline if inline is true
- `compile_sass(path)`: compile the sass file at the given path
- `use(path)`: use the file at the given path.
- `usehtml(name, html)`: use the given html element under the given name
- `useraw(name, content)`: use the given content raw
- `read(path)`: read the given file to a string

## Example:
src/index.lmk:
```
-- Use a template function called `default`
-- This is the same as not specifying any template
@template = default
@title = Hello world!
@date = 15-03-2025

-- String, to support multiline
@desc = "A hello world!
Now with multiple lines!"

= Hello world
This is some text!
_italic!_ *Bold!* `Monospace`
Next up, an image!

@image(the SLSG logo; logo.svg)

Inline macros are possible too!
If not seperated by newlines, they are put inside the `<p>` element
like so! @link(Home; /)

Next up, a codeblock!

@begin code(lua)
-- Templates are defined as follows:
local templates
function templates:page(content)
  return h {
    h.head {
      h.title { self.meta.title }
    },
    h.body {
      h.h1 { self.meta.title },
      h.time { class = 'time-small`, page.meta.date }
      h.p { self.meta.desc },
      h.article { self.content }
    }
  }
end

-- Smaller templates, for things like images
function templates:image(alt, src)
  -- use the image
  -- Looks for the file next to the luamark file if it's relative,
  -- or the project root if absolute
  -- returns the eventual location of the file if the name is not changed
  local loc = site.use(src)
  return h.img { alt = alt, src = loc }
end

return templates
@end
```

## Current TODO:
- [ ] have example site also serve as short intro to slsg (show some features)
- [ ] Lua language server files
- [ ] Fix HTML escape
- [ ] API docs
- [ ] Luamark parser
- [ ] Syntax highlighting + html generation
- [ ] Functioning macros

