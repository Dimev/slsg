use std::collections::VecDeque;

use mlua::{Lua, Result, Value};

use crate::Config;

enum Modifier {
    Bold,
    Italic,
    Mono,
}

/// Parse minimark to html
pub(crate) fn minimark(
    lua: &Lua,
    content: &str,
    name: &str,
    conf: &Config,
) -> Result<(String, VecDeque<Value>)> {
    // TODO
    // basic syntax:
    // = header
    // *bold*
    // _italic_
    // `mono`
    // % comment
    // <? html ?>
    // <?fnl fennel ?>
    // <?lua lua ?>
    // $ math $
    // $$ math $$
    // ```lang code block```
    // \x escape the next character (works for `, *, _, %, <? needs to be escaped with <??)
    let mut out = String::with_capacity(content.len());
    let mut functions = VecDeque::new();
    let mut chars = content.char_indices();
    let mut stack = Vec::<()>::new();

    while let Some((_, c)) = chars.next() {
        // comment
        if c == '%' {
            // read until the next \n
            while !chars.as_str().starts_with('\n') {
                chars.next();
            }
        }
        // header
        else if c == '=' {
            // count rest
            let head_count = chars.as_str().chars().take_while(|c| *c == '=').count() + 1;
            for _ in 1..head_count {
                chars.next();
            }
        }
        // paragraph break
        else if c == '\n' {
            // take the whitespace
            while chars.as_str().starts_with(char::is_whitespace) {
                chars.next();
            }

            // TODO: break paragraph
            out.push(' ');
        }
        // whitespace
        else if c.is_whitespace() {
            // add to the end of output
            if !out.ends_with(char::is_whitespace) {
                out.push(' ')
            }
            // skip until the next non-newline whitespace
            while chars.as_str().starts_with(char::is_whitespace) {
                chars.next();
            }
        } else {
            // push
            out.push(c);
        }
    }

    Ok((out, functions))
}
