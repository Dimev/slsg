// Parse tex and run lua with it
// commands are in the form of \name[1, 2, 3]{arg4, arg5, arg6}
// arguments are passed as tables of strings, if any commands are used, they are also added
// verbatim is done with \verb||, where | can be any character
// any top level text is put into a paragraph, if there are no empty lines between it
// this is done using the special paragraph function, which by default simply concatenates all text

use std::collections::HashMap;

use anyhow::anyhow;
use mlua::{Error, Function, Lua, Table, Value};

pub(crate) fn parse_tex<'lua>(
    lua: &'lua Lua,
    tex: &str,
    functions: Table<'lua>,
) -> Result<Value<'lua>, Error> {
    // current paragraph
    let paragraph = lua.create_table()?;

    let mut chars = tex.chars().peekable();
    while let Some(c) = chars.next() {
        // skip comment
        if c == '%' {
            // skip until a newline
            while let Some(c) = chars.next() {
                if c == '\n' {
                    break;
                }
            }
        }
        // math block
        else if c == '$' && chars.peek() == Some(&'$') {
            // skip second one
            chars.next();
        }
        // inline math block
        else if c == '$' {
        }
        // command
        else if c == '\\' {
            // read name
            let mut name = String::new();
            while let Some(c) = chars.peek() {
                // alphanum and underscor are allowed to be names
                if c.is_alphanumeric() || *c == '_' {
                    name.push(*c);
                    chars.next();
                } else {
                    break;
                }

                // if the name is verb for verbatim, read the next character,
                // then read everything literally until that char is encountered again
                if name == "verb" {
                    let mut verbatim = String::new();
                    let delim = chars.peek().copied();
                    while let Some(c) = chars.next() {
                        if delim == Some(c) {
                            break;
                        }
                        verbatim.push(c);
                    }

                    // push it to the table
                } else {
                    // parse the options
                }
            }
        }
        // paragraph
        else {
        }
    }

    Ok(Value::Nil)
}
