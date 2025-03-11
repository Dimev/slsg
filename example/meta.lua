---@meta site
-- lua language server file, to help completions

---@alias DirIter userdata
---@alias FileIter userdata

---@class Site
---@field dev boolean Whether the site is run with `slsg dev`
---@field logo string SVG logo of SLSG
---@field icon string SVG icon of SLSG, same as the logo, but without text
site = {}

--- Emit a file to the site generator
---@param path string Path to serve the file on
---@param data string Contents of the file
function site.emit(path, data) end

--- Emit a file to the site generator, by copying it from the original
---@param path string Path to serve the file on
---@param original string path to the original file, relative `to main.lua`
function site.emit_file(path, original) end

--- Emit a file to the site generator, by running a command
---@param path string Path to serve the file on
---@param command string command to run
---@param ... string arguments to the command
function site.emit_command(path, command, ...) end

--- Mark a file to be used as 404 page
--- This only has effect in development mode, when run with `slsg dev`
--- @param path string file to mark as 404 page
function site.set_404(path) end

--- Read all directory names at the given path
---@param path string path to read
---@return DirIter Iterator over the directory names
function site.dirs(path) end

--- Reads all file names at the given path
---@param path string path to read
---@return FileIter Iterator over the file names
function site.files(path) end

--- Reads the file at the given path
---@param path string path to read
---@return string file contents
function site.read(path) end

--- Check if a directory exists
---@param path string path to check
---@return boolean whether the directory exists
function site.dir_exists(path) end

--- Check if a file exists
---@param path string path to check
---@return boolean whether the file exists
function site.file_exists(path) end

--- Get the name of a file in the given path
--- The name is the final component of the path
--- This function is the same as rust's `Path::file_name`
---@param path string the path to use
---@return string the file name
function site.file_name(path) end

--- Get the stem of a file in the given path
--- The stem is the file name without the extension
--- This function is the same as rust's `Path::file_stem`
---@param path string the path to use
---@return string the file stem
function site.file_stem(path) end

--- Get the extension of a file in the given path
--- The extension is everything after the final `.` in the file path,
--- if it does not start with that
--- This function is the same as rust's `Path::extension`
---@param path string the path to use
---@return string the file name
function site.file_ext(path) end

--- Get the directory a file is in
--- This function is the same as rust's `Path::parent`
---@param path string the path to use
---@return string the directory name
function site.file_parent(path) end

--- Compile a tex expression to mathml
--- @param tex string the tex expression
--- @param inline? boolean whether to inline the mathml. Doing so sets `inline` on the `<math>` element to true
--- @return string the mathml string
function site.compile_tex(tex, inline) end

--- Compile a sass or scss file to css
---@param sass string the sass/scss to compile
---@param loader? function Function that is called with a path when a file needs to be loaded
---@param expand? boolean Whether to expand the css to be more readable
---@return string the resulting css
function site.compile_sass(sass, loader, expand) end

--- Compile luamark by calling the given macros
--- Luamark is a custom scripting language made for SLSG, it's preferred file extention is .lmk
--- Luamark has the following syntax:
--- comments: `% hello` started by a `%`, then goes until the next newline. These are ignored
--- text: any consecutive piece of text, including escaped text. calls the `text` macro with the escaped text as argument.
--- escaped text: any `\` followed by any number of non-whitespace characters
--- paragraph: any consecutive piece of text and macros, seperated by one or more empty lines.
--- calls the `paragraph` macro, with the result of it's containing text and macro results as a table.
--- line macros: `@name arguments` call the macro `name` with the remaining text on the line as the first argument
--- inline macros: `@name(arg1, arg2, arg3)` call the macro `name` with the arguments seperated by a `,`
--- inline macros also accepts `[]` instead of normal braces.
--- `||`, `$$`, `<>` and `{}` also work, but ignore the seperating `,`
--- block macros: `@begin@name@tag(arg1, arg2) arg3 @end@name@tag`
--- block macros work similar to line and inline macros,
--- but give everything after them until the closing `@end@name@tag` as the last argument.
--- the `@tag` is optional, but can be used to prevent closing when the body has `@end@name` as content.
--- when done, the `document` macro is called, with one table containing the result of all previous macro invocations as argument.
--- Example syntax:
--- ```
--- % Line macros
--- @name This is the rest!
--- 
--- % Inline macros
--- % also possible with (), []
--- % {}, <>, || and $$ don't do multiple arguments
--- @name(arg1, arg2, arg3)
--- 
--- % We can also do block macros, to include code verbatim
--- % These take all text in them literally, 
--- % and only end at the closing @end@name tag
--- @begin@name(arg1, arg2, arg3)
--- This is all verbatim!
--- @end@name
--- ```
---@param luamark string The luamark to compile
---@param macros table<string, function> The macros
---@return any the result of the macros being run
function site.compile_luamark(luamark, macros) end

---@class SyntaxHighlighter
highlighter = {}

---@class rule
---@field token string The token to emit when this matches
---@field regex string The regex to match for this rule
---@field next? string the optional next ruleset to go to

--- Create a syntax highlighter from the given rules
--- The highlighter starts at the `start` ruleset, then tries to match all rules on the text
--- it emits a token for each matching rule
--- The regex is provided by the `fancy-regex` crate
--- The highlighter is inspired by the one provided by the `ace` editor
--- A simple syntax highlighter for luamark would look as follows:
--- ```
--- site.create_highlighter {
---   start = {
---     { token = 'comment', regex = '%.*' },
---     { token = 'macro',   regex = [[@\w+]] },
---   }
--- }
--- ```
---@param rules table<string, table<rule>> the rules to use
---@return SyntaxHighlighter the syntax highlighter
function site.create_highlighter(rules) end

--- Highlight code into html
--- @param text string the code to higlight
--- @param class? string the prefix to append to each token
--- @return string the html, in the form of a series of <span> elements
function highlighter:highlight_html(text, class) end

---@class Span
---@field text string The text of this span
---@field token string The associated token of this span

--- Highlight code into an ast
--- @param text string the code to higlight
--- @return table<Span> the resulting ast, as a list of spans
function highlighter:highlight_ast(text) end

--- Escape html, replaces <, > and & with &lt;, &gt; and &amp;
---@param html string
---@returns html string the escaped html
function site.escape_html(html) end

--- Escape html quotes, replaces ' and " with &#39; and &quot;
---@param html string
---@returns html string the escaped html
function site.escape_html_quote(html) end

---@class Elem An html/xml element
---@field elem string The element type
---@field attrs table<string, string> the element atributes
---@field elems table<string, any> the child elements

--- Render a html element
---@param elem Elem the html element
---@return string the resulting html
function site.html_render(elem) end

--- Create a html fragment, from a list of elements
---@param elems table<Elem> the elements
-- @return Elem the fragment
function site.html_fragment(elems) end

--- Merge a list of elements into a fragment
--- This merges any consecutive elements of the same type into one longer element
--- This is useful for merging text into a paragraph for luamark
---@param elems table<Elem> the elements
---@return Elem the resulting merged fragment
function site.html_merge(elems) end

--- Create an html element
--- This automatically omits closing tags that are not needed when rendering
---@param elem string the element type
---@param content table the content of the element, pairs are attributes, ipairs are children
---@return Elem the resulting element
function site.html_element(elem, content) end

--- Render HTML, or render an HTML element
--- When accessing a field of this table,
--- returns a function that creates an element with that name
--- @see site.html_element
--- When called as a function, renders the given element,
--- or returns the text as a raw html element
--- @see site.html_render
--- This can be used to create html as follows:
--- ```lua
--- local h = site.html
--- local page = h {
---   h.h1 'Hello world!'
---   h.div {
---     class = 'center',
---     h.p 'This is my site!',
---     h.p 'See, more text!',
---     h.img { src = 'logo.png', alt = 'Site logo' },
---   }
--- }
--- ```
--- @type { [string]: fun(content: table): Elem }
--- @overload fun(elem: table<Elem>): string
--- @overload fun(elem: string): Elem
site.html = {}

--- Render a xml element
---@param elem Elem the html element
---@return string the resulting html
function site.xml_render(elem) end

--- Create a xml fragment, from a list of elements
---@param elems table<Elem> the elements
-- @return Elem the fragment
function site.xml_fragment(elems) end

--- Create an xml element
---@param elem string the element type
---@param content table the content of the element, pairs are attributes, ipairs are children
---@return Elem the resulting element
function site.xml_element(elem, content) end

--- Render XML, or render an XML element
--- When accessing a field of this table,
--- returns a function that creates an element with that name
--- @see site.html_element
--- When called as a function, renders the given element,
--- or returns the text as a raw xml element
--- @see site.html_render
--- This can be used to create html as follows:
--- ```lua
--- local svg = site.xml
--- svg {
---   svg.svg {
---     version = '1.1',
---     width = '100',
---     height = '100',
---     xmlns = 'http://www.w3.org/2000/svg',
---     svg.circle { cx = 50, cy = 50, r = 50, fill = '#1D2951' },
---     svg.circle { cx = 65, cy = 35, r = 15, fill = 'white' },
---   }
--- }
--- ```
--- @type { [string]: fun(content: table): Elem }
--- @overload fun(elem: table<Elem>): Elem
--- @overload fun(elem: string): Elem
site.xml = {}

return site
