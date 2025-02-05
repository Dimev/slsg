use std::{
    collections::BTreeMap,
    io::{stdout, Stdout, Write},
};

use crossterm::{
    queue,
    style::{Attribute, Print, SetAttribute, Stylize},
    QueueableCommand,
};
use fancy_regex::RegexBuilder;

use crate::highlight::{Highlighter, Rule};

const DOCSTRING: &str = r#"## SLSG
# Scriptable Lua Site Generator
# SLSG will run the file `main.lua` in the specified directory
# see `slsg --help` for all command line options

## Arguments
# Any argument from the command line after -- are passed as function arguments to the `main.lua` file
# This means they can be accessed as follows:
> local args = { ... }
# If the arguments are `slsg dev -- a b c`, this results in the following:
>> local args = { 'a', 'b', 'c' }

## Debugging
site.dev
# Boolean indicating whether slsg is currendly running under dev mode (`slsg dev`) or not

## Filesystem
function site.dir(path)
# Read a directory at the specified path
# Returns an iterator over all files and directories, excluding `.` and `..`
# This is the same as LFS' dir

function site.read(path)
# Returns the content of the file at `path` as a string

function site.file_name(path)
# Returns the file name at `path`
# This is the final component of the path, and corresponds to rust's `Path::file_name`

function site.file_stem(path)
# Returns the file stem at `path`
# This is the file name without the extension, and corresponds to rust's `Path::file_stem`

function site.file_ext(path)
# Returns the file extension at `path`
# This is the file extension of the file name, and corresponds to rust's `Path::extension`

## MathML
function site.compile_tex(latex, inline)
# Convert the given LaTeX string to a MathML string
# The `inline` flag determines whether to display the LaTeX as inline,
# and corresponds to the `inline` flag on the MathML `<math>` element
# Example:
> site.compile_tex [[ \int{1} dx = x + C ]]
>> '<math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><msqrt><mi>x</mi></msqrt></math>'

## Sass
function site.sass(sass, loader, expand)
# Compiles the given sass/scss/css code
# When encountering file imports, load them with loader.
# `site.read` can be passed as loader here
# `expand` determines whether to expand the resulting code
# false by default, this minifies the resulting css

## Luamark
# Luamark is a special markdown language made specifically for SLSG
# It makes writing code using custom functionality provided from lua easier,
# compared to using markdown.
# The preferred file extention for luamark is lmk

function site.compile_luamark(luamark, macros)
# Parses the given luamark, then builds the result from the given macro table
# Example:
> local h = site.html
> local function parse(article)
>   -- table with all values
>   local macros = {
>     title = '',
>   }
> 
>   -- text is wrapped in <p>
>   function macros:text(args)
>     return h.p(args)
>   end
> 
>   -- paragraphs are concatenated from the results
>   function macros:paragraph(args)
>     return site.html_merge(args)
>   end
> 
>   -- same with the resulting document
>   function macros:document(args)
>     return site.html_fragment(args)
>   end
> 
>   -- add a title
>   function macros:title(args)
>     self.title = args
>   end
> 
>   -- add an image
>   function macros:img(path, alt)
>     return h.img { src = path, alt = alt }
>   end
> 
>   -- inline code
>   function macros:inline(args)
>     return h.p { h.code { class = 'codeline', args } }
>   end
> 
>   -- parse a luamark article
>   local res = site.compile_luamark(article, macros)
>   return h.main {
>     class = 'main',
>     h.h1(macros.title),
>     res
>   }
> end
# This can then be used to parse a luamark file, like so
> @title Hello SLSG!
> @img(logo.svg, SLSG logo)
> 
> Edit the files to start making your site!
> 
> Run @inline|slsg api| to see what all functions do, 
> including examples!

## Syntax highlighting
function site.create_highlighter(rules)
# Create a new syntax highlighter, from the given highlighting rules
# The regex engine used is from rust's `fancy-regex` crate
# The highlighter expects a table of table of rules,
# where each rule tries to match the regex, and if it matches, emits the token
# Optionally, there can also be a `next` rule, to move to a new rule
> local highlighter = site.create_highlighter {
>   start = {
>     { token = 'comment', regex = '%.*' },
>     { token = 'macro',   regex = [[@\w+]] },
>     { token = 'multi',   regex = [[@begin@\w+]], next = 'multi' }
>   },
>   multi = {
>     { token = 'end', regex = [[@end@\w+]], next = 'start' }
>   }
> }

function highlighter:highlight_html(text, class)
# Highlight the given text, and return it as html
# `class` is appended in front of all token names, 
# and then used as class names for the spans of code
> highlighter.highlight_html('Hello % comment', 'code-')
>> '<span class="code-">Hello </span><span class="code-comment">% comment</span>'

function highlighter:highlight_ast(text)
# Highlight the given text, then return it as a table of nodes
# Each node is a table with the token name, and it's corresponding text
> highlighter.highlight_html('Hello % comment', 'code-')
>> {
>>    { token = '', text = 'Hello ' },
>>    { token = 'comment', text = '% comment' },
>> }

## HTML
function site.escape_html(html)
# Escapes the given html
> site.escape_html '<p class="greeting">Hello world!</p>'
>> '&lt;p class=&quot;greeting&quot;&gt;Hello world!&lt;/p&gt;'

function site.html_render(elem)
# Render a html element, which can be created with `create_element('elem', { ... })`, 
# or `site.html.elem { ... }`
# This corresponds `to elem:render()`

function site.html_fragment(elems)
# Create a fragment from the given table of html elements
# Fragments are simply a list of elements without surrounding tags

function site.html_merge(elems)
# Merge a list of html elements
# If the tags are the same, this will merge the attributes and content
# Helpfull if you want to merge together blocks of text in luamark, as in the example

function site.html_element(elem, content)
# Create a new html element of type `elem`
# Content is the table of attributes and elements
# Any entry in the table using a string as key is considered an attribute,
# any entry in the table using a number as key is considered an element
> site.html_element('p', { class = 'greetings', 'Hello world!' }):render()
>> '<p class="greetings">Hello world!</p>'

site.html
# A special table to make writing html easier
# When called directly, it renders the given elements
# Wen an index is required from the table, it creates a new element
> local h = site.html
> local page = h {
>   h.h1 'Hello world!'
>   h.div {
>     class = 'center',
>     h.p 'This is my site!',
>     h.p 'See, more text!',
>     h.img { src = 'logo.png', alt = 'Site logo' },
>   }
> }
>> '<h1>Hello world!</h1><div class="center"><p>This is my site</p><img src="logo" alt="Site logo"></div>'

## XML
# A similar set of functions for XML are given as the HTML ones.
# These don't omit closing tags, where HTML allows omitting them.

function site.xml_render(elem)
# Render a xml element, which can be created with `create_element('elem', { ... })`, 
# or `site.xml.elem { ... }`
# This corresponds `to elem:render()`

function site.xml_element(elem, content)
# Create a new xml element of type `elem`
# Content is the table of attributes and elements
# Any entry in the table using a string as key is considered an attribute,
# any entry in the table using a number as key is considered an element
> site.xml_element('p', { class = 'greetings', 'Hello world!' }):render()
>> '<p class="greetings">Hello world!</p>'

site.xml
# A special table to make writing xml easier
# When called directly, it renders the given elements
# Wen an index is required from the table, it creates a new element
> local svg = site.xml
> svg {
>   svg.svg {
>     version = '1.1',
>     width = '100',
>     height = '100',
>     xmlns = 'http://www.w3.org/2000/svg',
>     svg.circle { cx = 50, cy = 50, r = 50, fill = '#1D2951' },
>     svg.circle { cx = 65, cy = 35, r = 15, fill = 'white' },
>   }
> }
>> '<svg width="100" xmlns="http://www.w3.org/2000/svg" height="100" version="1.1"><circle cy="50" r="50" cx="50" fill="\#1D2951"></circle><circle cy="35" r="15" cx="65" fill="white"></circle></svg>'

## Other
site.logo
# Returns the logo of SLSG

site.icon
# Returns the icon of SLSG (logo without text)
"#;

/// Print the stdlib
pub(crate) fn print_stdlib() {
    let mut stdout = stdout();
    print_lua(&mut stdout, include_str!("stdlib.lua"), &lua_highlighter());
}

/// Print the API documentation
pub(crate) fn print_docs() {
    let mut stdout = stdout();
    let lua_highlighter = lua_highlighter();
    for line in DOCSTRING.lines() {
        if let Some(rest) = line.strip_prefix("##") {
            // ## means a bold line
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print("## "),
                Print(rest.trim()),
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .expect("Failed to print line");
        } else if let Some(rest) = line.strip_prefix("#") {
            // # means a normal doc line
            queue!(stdout, Print("  "), Print(rest.trim()), Print("\n"))
                .expect("Failed to print line");
        } else if let Some(rest) = line.strip_prefix(">>") {
            // Highlight example return value
            queue!(stdout, Print(">>".dark_grey()),).expect("Failed to print line");
            print_lua(&mut stdout, rest, &lua_highlighter);
            queue!(stdout, Print("\n")).expect("Failed to print line");
        } else if let Some(rest) = line.strip_prefix(">") {
            // Highlight example code
            queue!(stdout, Print(">".dark_grey())).expect("Failed to print line");
            print_lua(&mut stdout, rest, &lua_highlighter);
            queue!(stdout, Print("\n")).expect("Failed to print line");
        } else if !line.is_empty() {
            // Highlight code
            print_lua(&mut stdout, line, &lua_highlighter);
            queue!(stdout, Print("\n")).expect("Failed to print line");
        } else {
            queue!(stdout, Print("\n")).expect("Failed to print line");
        }
    }

    stdout.flush().expect("Failed to flush stdout");
}

/// Print a lua code line
fn print_lua(stdout: &mut Stdout, line: &str, highlighter: &Highlighter) {
    for span in highlighter.highlight(line).into_iter() {
        match span.token.as_str() {
            "keyword" => {
                stdout
                    .queue(Print(span.text.yellow()))
                    .expect("Failed to print line");
            }
            "special" => {
                stdout
                    .queue(Print(span.text.cyan()))
                    .expect("Failed to print line");
            }
            "string" => {
                stdout
                    .queue(Print(span.text.green()))
                    .expect("Failed to print line");
            }
            "control" => {
                stdout
                    .queue(Print(span.text.yellow()))
                    .expect("Failed to print line");
            }
            "name" => {
                stdout
                    .queue(Print(span.text.bold()))
                    .expect("Failed to print line");
            }
            "comment" => {
                stdout
                    .queue(Print(span.text.blue()))
                    .expect("Failed to print line");
            }
            "number" => {
                stdout
                    .queue(Print(span.text.red()))
                    .expect("Failed to print line");
            }
            _ => {
                stdout
                    .queue(Print(span.text))
                    .expect("Failed to print line");
            }
        }
    }
}

/// Highlighter for lua
fn lua_highlighter() -> Highlighter {
    let start_rules = vec![
        Rule {
            token: "comment".to_string(),
            regex: RegexBuilder::new("--.*").build().unwrap(),
            next: None,
        },
        Rule {
            token: "special".to_string(),
            regex: RegexBuilder::new("\\w+(?=\\.|:)").build().unwrap(),
            next: None,
        },
        Rule {
            token: "name".to_string(),
            regex: RegexBuilder::new("(?<=\\.|:)\\w+(?=\\()").build().unwrap(),
            next: None,
        },
        Rule {
            token: "keyword".to_string(),
            regex: RegexBuilder::new(r"\b(function|local|return|end|for|do|if|else|elseif|then)\b")
                .build()
                .unwrap(),
            next: None,
        },
        Rule {
            token: "control".to_string(),
            regex: RegexBuilder::new("\\(|\\)|\\,|{|}|=").build().unwrap(),
            next: None,
        },
        Rule {
            token: "string".to_string(),
            regex: RegexBuilder::new("(\\[\\[[^(\\]\\])]*\\]\\])|(\"[^\\\"]*\")|('[^\\']*')")
                .build()
                .unwrap(),
            next: None,
        },
        Rule {
            token: "number".to_string(),
            regex: RegexBuilder::new(r"\b(\d+)\b")
                .build()
                .unwrap(),
            next: None,
        },
    ];

    let lua_highlighter = Highlighter {
        rules: BTreeMap::from_iter([("start".to_string(), start_rules)].into_iter()),
    };
    lua_highlighter
}

