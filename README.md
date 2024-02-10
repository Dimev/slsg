# YASSG
Yet another static site generator

# How does this differ from other site generators?
- YASSG uses lua as it's scripting and templating language, allowing for more advanced logic for generating sites
- YASSG allows pages to be made from lua, thus allowing to generate a different file structure than the defined one

# How to use it
`yassg new [name]` to initialize a new project. This will create a directory with that name, 
a `site.lua` file, the content directory, style, static and lib directory (TODO)

content is for the site, lib is for lua scripts, style is for sass stylesheets, 
static is for statically accessible files 

The completed website is output to public/

`yassg build` builds the site by looking at the first found `site.lua` file in the current or any ancestor directories

# How to generate sites
TODO

# API
## TODO: move to a single object called currentPage?

## Available globals
the global colocatedFiles is used to help find files that are next to the index.lua files
these consist of userdata with a type
the type can be asset, page, meta or dir

assets can be loaded with the :loadMd, :loadJson, :loadYaml, :loadToml, :loadBibtex functions
asset file names are under the name property, with file stems (without extention) and extentions under stem and extention respectively

pages have a name, meta (table with extra info), and html
these can be created with the page() function

meta just has an info table

dirs contain a table of subs

stylesheets is a table for each of the stylesheets, with the extention removed

## Making pages
TODO

## Loading files
any assets can be loaded by calling their respective functions

# Current TODO:
- allow global files for static and stylesheets
- rename meta to table
- convert tables to custom vals instead
- add asset loading (markdown, json/yaml/toml/bibtex)
- allow making/inserting text files
- add cookbook
- add code highlighting
- better error management/propagation
- subcommands for new, build, dev
- dev server
- docs
