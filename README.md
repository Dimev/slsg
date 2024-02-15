# YASSG
Yet another static site generator

# How does this differ from other site generators?
- YASSG uses lua as it's scripting and templating language, allowing for more advanced logic for generating sites
- YASSG allows pages to be made from lua, thus allowing to generate a different file structure than the defined one

# How to use it
`yassg new [name]` to initialize a new project. This will create a directory with that name, 
a `site.toml` file, the content directory, style, static and lib directory (TODO)

content is for the site, lib is for lua scripts, style is for sass stylesheets, 
static is for statically accessible files 

The completed website is output to `public/`

`yassg build` builds the site by looking at the first found `site.toml` file in the current or any ancestor directories

# How to use:
yassg runs the `site/index.lua` file, which is expected to return a page
this page is then converted into a website

The final returned item of this script is expected to be a page

`index.lua`, as well as all other scripts found in `site/` or subdirectories
get access to a global named `template`, which is a `template`
They also get access to the `config` table, which is loaded from `site.toml`

`template`:
- `colocated`: `directory` for the colocated files, if this was an `index.lua` file, otherwise an empty directory
- `name`: stem of the `*.lua` file, or directory name if `index.lua`
- `static`: `directory` for the static files
- `styles`: return table of `file`s, for the style
- `find(path)`: finds a `file` from the given path
- `findStatic(path)`: find a static `file` from the given path

`directory`:
- `files`: table for all colocated files
- `directories`: table for all colocated directories
- `scripts`: table for all colocated scripts (`*.lua`, or `./index.lua`)

`file`:
- can be created with the `file(text)` function
- `parseMd()`: parses the file as markdown
- `parseJson()`: parses the file as json, into a table
- `parseYaml()`:  parses the file as yaml, into a table
- `parseToml()`: parses the file as toml, into a table
- `parseTxt()`: loads the file as a string
- `name`: full name
- `stem`: file name without the extention
- `extention`: extention only, if any, excluding the leading `.`

`markdown`:
- `front`: the front matter, parsed as toml
- `raw`: the raw markdown
- `html`: the markdown as html
- TODO

`page`:
- can be created with the `page(name)` function
- `withFile(path, file)`: adds a file at the given relative path
- `withHtml(html)`: adds html to the page. If no html is used, no index.html file is generated for the directory
- `withPage(page)`: adds a subpage to the page

# Rendering HTML
Besides including these page and file searching functions, 
there's also a small library for rendering html
TODO

# Code highlighting
TODO

# Static content
Static content under the `static/` folder is not included by default,
and has to be added manually via `withFile` on page 

# Styling
all content under the `styles` folder is interpreted as css, scss or sass, depending on the extention
all top-level at the root of the directory is available under the styles

# Minification
all html passed in via withHtml is minified, as well as all stylesheets under `style/`
the `minifyHtml`, `minifyCss` and `minifyJs` functions can be used to minify html, css and javascript respectively
TODO

# Config
the `site.toml` file can be used for configuring.
it is loaded into a table, and can be accessed via the `config` global 

# Organization TODO:
- create separate rust modules for these types

# Current TODO:
- rewrite the entire thing
- don't load package lua stdlib, use our own require instead
- code highlighting
- subcommands for new, build, dev
- dev server
- docs
