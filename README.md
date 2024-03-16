# LSSG
Lua Static Site Generator

# How does this differ from other site generators?
- lssg uses lua as it's scripting and templating language, allowing for more complex logic with less boilerplate for generating sites
- lssg allows pages to be made from lua, thus allowing to generate a different file structure than the defined one in the file system

# How to use it
`lssg new [name]` to initialize a new project. This will create a directory with that name, 
a `site.toml` file, the content directory, style, static and lib directory

`lssg init` initializes the current directory as a new site, if it's empty

`site/` is for the site, `lib/` is for lua scripts, `styles/` is for sass stylesheets, 
`static/` is for statically accessible files

The completed website is output to `public/`, or the folder specified by `-o` or `--output`

`lssg build` builds the site by looking at the first found `site.toml` file in the current or any ancestor directories.

`lssg cookbook [name]` shows a script that may be useful when making a site. Run without name to see the full list.

# How to use:
lssg runs the `site/index.lua` file, which is expected to return a page
this page is then converted into a website

The final returned item of this script is expected to be a page

`index.lua`, as well as all other scripts found in `site/` or subdirectories get access to a global named `script`.
This serves as the main way to interact with the file system
They also get access to the `config` table, which is loaded from `site.toml`

`script`:
- `colocated`: `directory` for the colocated files, if this was an `index.lua` file, otherwise an empty directory
- `name`: stem of the `*.lua` file, or directory name if `index.lua`
- `static`: `directory` for the static files
- `styles`: return table of `file`s, for the style

`directory`:
- `files`: table for all colocated files
- `directories`: table for all colocated directories
- `scripts`: table for all colocated scripts (`*.lua`, or `./index.lua`)
- `find(path)`: finds a `file` from the given path

`file`:
- can be created with the `file(text)` function
- `parseMd()`: parses the file as markdown
- `parseJson()`: parses the file as json, into a table
- `parseYaml()`:  parses the file as yaml, into a table
- `parseToml()`: parses the file as toml, into a table
- `parseTxt()`: loads the file as a string
- `parseBibtex()`: loads the file as bibtex, into a table
- `stem`: file stem if any, or nil
- `name`: file name if any, or nil
- `extention`: file extention if any, or nil

`markdown`:
- `front`: the front matter as a table, or nil if none. --- is parsed as yaml, +++ is parsed as toml
- `raw`: the raw markdown
- `html`: the markdown as html
- `events`: table of all events in the markdown stream

`page`:
- can be created with the `page(name)` function
- `withFile(path, file)`: adds a file at the given relative path
- `withHtml(html)`: adds html to the page. If no html is used, no index.html file is generated for the directory
- `withPage(page)`: adds a subpage to the page

# Other globals
- `warn`: Accepts a single string, warnings will be shown in the terminal and error page

# lssg library
- These are available under the `site` global table
- `debug`: bool, true if the site is built from the `serve` command
- `escapeHtml`: escapes the given html string TODO
- `unescapeHtml`: unescapes the given html string TODO

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
the `minifyhtml`, `minifycss` and `minifyjs` functions can be used to minify html, css and javascript respectively
TODO

# Config
the `site.toml` file can be used for configuring.
everything under the `[config]` section is loaded into the `config` global

# Known bugs:
- Syntax highlighting currently leaks memory if a prefix string is given, 
  due to the lifetime of syntects's ClassStyle::SpacedPrefixed needing to be static

# Current TODO:
- have example site also serve as short intro to lssg
- clippy
- code highlighting rules for common language set
- tex (as in, parse a subset of latex)
- minification
- docs
- Image resizing

# Cookbook TODO:
- manual markdown rendering
- code highlighting
- markdown based blog
- bibtex bibliography
