use std::collections::VecDeque;

use mlua::{Function, Lua, Result, Value, chunk};

use crate::conf::Config;

// mode to parse next
// Fennel does not support '' and [[]] strings
#[derive(PartialEq, Eq)]
enum ParseMode {
    Raw,
    Fennel,
    Lua,
    TexBlock, // TODO: latex
    TexInline,
}

pub(crate) fn template(
    lua: &Lua,
    content: &str,
    conf: &Config,
) -> Result<(String, VecDeque<Value>)> {
    let mut out = String::with_capacity(content.len());
    let mut functions = VecDeque::new();

    let mut chars = content.char_indices();
    let mut mode = ParseMode::Raw;
    while let Some((_, c)) = chars.next() {
        if mode == ParseMode::Raw {
            // open tag and a ?lua? parse lua
            if c == '<' && chars.as_str().starts_with("?lua") {
                if !conf.lua {
                    return Err(mlua::Error::external(
                        "Found a lua code block, but lua is not enabled in `site.conf`",
                    ));
                }
                mode = ParseMode::Lua;
                chars.next();
                chars.next();
                chars.next();
                chars.next();
            }
            // open tag and a ?fnl? parse fennel
            else if c == '<' && chars.as_str().starts_with("?fnl") {
                if !conf.fennel {
                    return Err(mlua::Error::external(
                        "Found a fennel code block, but fennel is not enabled in `site.conf`",
                    ));
                }
                mode = ParseMode::Fennel;
                chars.next();
                chars.next();
                chars.next();
                chars.next();
            }
            // else, simply push the character
            else {
                out.push(c);
            }
        } else if mode == ParseMode::Lua {
            // TODO: fix
            // skip until the closing ?>
            let mut code = String::new();
            while !chars.as_str().starts_with("?>") && !chars.as_str().is_empty() {
                code.push(chars.next().unwrap().1);
            }

            // exec
            let result: Value = lua.load(code).eval()?;

            if result.is_string() || result.is_number() || result.is_boolean() {
                out.push_str(&result.to_string()?);
            }

            if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }

            // skip closing ?>
            chars.next();
            chars.next();
            mode = ParseMode::Raw;
        } else if mode == ParseMode::Fennel {
            // TODO: fix
            // skip until the closing ?>
            let mut code = String::new();
            while !chars.as_str().starts_with("?>") && !chars.as_str().is_empty() {
                code.push(chars.next().unwrap().1);
            }

            // exec
            let result: Value = lua.load(chunk!(require("fennel").eval($code))).eval()?;

            if result.is_string() || result.is_number() || result.is_boolean() {
                out.push_str(&result.to_string()?);
            }

            if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }

            // skip closing ?>
            chars.next();
            chars.next();
            mode = ParseMode::Raw;
        }
    }

    Ok((out, functions))
}
