/*

Features:
Comment, --
Commands, start with @, followed by name, optional string argument or function call
String, same as lua string literal
Escaping: done with \, not needed in string literal

@article(Hello world!, 1-1-1970)

-- Comment
This is a paragraph.
It consists of multiple lines

This is a second paragraph
Again, multiple lines

@section(And a third paragraph)
Now with a header if it's set up that way

@code(lua, [[
-- and some lua sample code!
print("Hello, world!")
]])

*/

use mlua::{Lua, Result, Table};

pub(crate) struct Parser<'a> {
    row: usize,
    column: usize,
    string: &'a str,
    lua: &'a Lua,
    commands: Table<'a>,
}

impl<'a> Parser<'a> {
    pub(crate) fn parse(lua: &Lua, commands: Table, string: &str) -> Result<()> { 
        // make the parser
        let parser = Parser {
            row: 1,
            column: 1,
            string,
            lua,
            commands,
        };

        // start parsing

        
        Ok(())
    }
}
