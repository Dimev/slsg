<? require('scripts/templates').page {
  title = "Scriptable Lua Site Generator",
  description = "Generate static sites with lua",
} ?>

# SLSG --- Scriptable Lua Site Generator
Instead of a templating language like tera, handlebars, or other, there are `lua`
and `fennel`. This allows more freedom in deciding how you template your site!

## Template with lua or fennel
Any files ending with `*.lua.*` or `*.fnl.*` will scan for `<?lua ... ?>` or
`<?fnl ... ?>`, and evaluate any code inside.

The code is then evaluated, and any bool, string or number returned is put directly
in the file. A function or table is instead called after templating is done, with the
entire file as arguments.

## Get the project
The code can be found [here on github](https://github.com/Dimev/slsg).
The easy way to install is to install [the rust compiler](https://www.rust-lang.org/),
and then run the following:
```sh
cargo install --git https://github.com/Dimev/slsg
```

This will install SLSG for you. Alternatively, you can
***TODO*** download the binary

# Example:
This page!

We can create a template like so:
```html
<!-- template.html -->
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta content="width=device-width,initial-scale=1" name="viewport">
  <title>@@title</title>
</head>

<body>@@content</body>

</html>
```

And a function to load our template
```lua
-- template.lua
function page(args)
  local template = readfile 'template.html'
    :gsub('@@title', args.title)

  return function(content)
    return template:gsub('@@content', content)
  end
end
```

Then, when we write markdown, we can use this to template our page!
```markdown
<? require 'template'.page {
  -- Give our title
  title = "Hello word!"
} ?>

# Hello world
We have a page!
```

# Features
Here's the full feature list!

## Markdown
Any file ending in `*.md` is interpreted as markdown. Combined with the
templating features, this allows you to write markdown for the content!

Inside markdown, the following is possible:
- Templating: Any processing instruction ([see markdown spec](https://spec.commonmark.org/0.31.2/#html-blocks))
  that starts with a `<?lua` or `<?fnl`, and closes with a `?>` is interpreted
  as templating. The script inside is then run as described earlier.
- Syntax highlighting: Any code block will be highlighted, as so:
  ```lua
  -- Comment!
  function replace(a, b) return a:gsub('@@content', b) end
  ```
- Math: Any markdown math blocks, delimited with \$, are converted to mathml:
  $$ \sqrt{t^2 + h^2 + 2th \cos(\theta)} $$
  Inline works as well: $a^2 + b^2 = c^2$

## Font subsetting
Any OpenType or FreeType (`*.otf` and `*.ttf`) font is subsetted if it ends with
either `*.subset.ttf` or `*.subset.otf`.

The charset used for subsetting are
all characters found after parsing the generated `html` files.
More characters can be added from lua with the function `extendcharset`, or
via the config file.

## Automatic `index.html` generation
Any `*.md` and `*.html` files that are not named `index.md` or `index.html` are
automatically placed into a directory with their file name.

