// Parse tex and run lua with it
// commands are in the form of \name[1, 2, 3]{arg4, arg5, arg6}
// arguments are passed as tables of strings, if any commands are used, they are also added
// verbatim is done with \verb||, where | can be any character
// any top level text is put into a paragraph, if there are no empty lines between it
// this is done using the special paragraph function, which by default simply concatenates all text

use std::collections::HashMap;

use anyhow::anyhow;
use mlua::{Error, Function, Lua, Table, Value};

struct Lexer<'a> {
    chars: std::str::Chars<'a>,
    cur: Option<char>,
    peek: Option<char>,
}

impl<'a> Lexer<'a> {
    fn new(chars: &'a str) -> Self {
        let mut out = Self {
            chars: chars.chars(),
            cur: None,
            peek: None,
        };

        out.next();
        out.next();

        out
    }

    fn next(&mut self) -> Option<char> {
        let cur = self.cur;
        self.cur = self.peek;
        self.peek = self.chars.next();
        cur
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.next() {
            if c == '\n' {
                break;
            }
        }
    }

    // assumes cur is $
    fn inline_math(&mut self) -> String {
        let mut math = String::new();

        // read until the closing $
        while let Some(c) = self.next() {
            // if c is the closing $, stop
            if c == '$' {
                break;
            }
            // if the current one is \$, append $
            else if c == '\\' && self.peek == Some('$') {
                // skip the \
                self.next();

                // append
                math.push(c);
            }
            // just append
            else {
                math.push(c);
            }
        }

        math
    }

    // assumes cur is $ and next is $
    fn math(&mut self) -> String {
        let mut math = String::new();

        // read until the closing $
        while let Some(c) = self.next() {
            // if c is the closing $, stop
            if c == '$' && (self.peek == Some('$') || self.peek.is_none()) {
                self.next();
                break;
            }
            // if the current one is \$, append $
            else if c == '\\' && self.peek == Some('$') {
                // skip the \
                self.next();

                // append
                math.push(c);
            }
            // just append
            else {
                math.push(c);
            }
        }

        math
    }

    // assumes cur is \
    fn command<'lua>(
        &mut self,
        lua: &'lua Lua,
        functions: Table<'lua>,
    ) -> Result<Value<'lua>, Error> {
        // read the command name
        let mut name = String::new();
        while let Some(c) = self.next() {
            // stop if it's not alphanum or _
            if !(c.is_alphanumeric() || c == '_') {
                break;
            } else {
                name.push(c);
            }
        }

        // verbatim means we directly copy the content
        if name == "verb" {
            // read the verbatim character we use to stop
            let delim = self.next();
            let mut verb = String::new();
            while let Some(c) = self.next() {
                if Some(c) == delim {
                    break;
                } else {
                    verb.push(c)
                }
            }

            // eval the verbatim, if any
            let fun: Option<Function> = functions.get("verb")?;
            if let Some(f) = fun {
                Ok(f.call((functions, verb))?)
            } else {
                Ok(Value::Nil)
            }
        } else {
            // parse arguments
            let args = lua.create_table()?;
            if self.next() == Some('{') {
                let mut text = String::new();
                while let Some(c) = self.next() {
                    // stop if it's the closing bracket
                    if c == '}' {
                        break;
                    }

                    // skip if it's escape
                    if c == '\\' && self.cur == Some('$') {
                        // TODO
                    }

                    // , delimit
                }
            }

            Ok(Value::Nil)
        }
    }
}

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
