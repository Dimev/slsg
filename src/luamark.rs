// TODO: don't do tex, do our own mini tex
// format is very simple
// -- is ignored as a comment
// empty line seperates paragraphs
// paragraphs are passed into paragraph function, document returned as multivalue
// @fn() calls a lua function
// inside function calls, , is used to seperate arguments
// everything is passed as a string or another function call, escaping can be done with \, or literals can be done with the lua string syntax [[ and ]] 
// We give special treatment to $ and $$ to make math easier

use anyhow::anyhow;
use mlua::{Error, Function, Lua, Table, Value};

struct Lexer<'a> {
    chars: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(chars: &'a str) -> Self {
        Self { chars }
    }
}
