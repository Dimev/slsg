<?lua
  require('scripts/templates').page

?>

# SLSG --- Scriptable Lua Site Generator
Instead of a templating language like tera, handlebars, or other, there are `lua`
and `fennel`. This allows more freedom in deciding how you template your site!

## How it works
Any file that matches `*.lua.*` or `*.fnl.*` will be set for templating. In these
files, any `<?lua .. ?>` or `<?fnl ... ?>` blocks will have the `lua` or `fennel`
code they contain evaluated. If this results in a number, boolean or sting, this
result will be pasted into the final file. If it insteads returns a function or
table, this function or table will be called with the full

## Overview of features
- [Templating](/templating) -- See how to use templating
- [Math with TeX](/math) -- How to use the TeX to Mathml converter
- [Syntax highlighting](/syntax) -- How to use syntax highlighting
- [Font subsetting](/subset) -- How to use the font subsetter

## Get the project
The code can be found [here on github](https://github.com/Dimev/slsg).
The easy way to install is to install [the rust compiler](https://www.rust-lang.org/),
and then run the following:
```sh
cargo install --git https://github.com/Dimev/slsg
```
