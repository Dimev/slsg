use std::collections::VecDeque;

use latex2mathml::{DisplayStyle, latex_to_mathml};
use mlua::{ErrorContext, ExternalResult, Lua, Result, Value, chunk};

use crate::conf::Config;

// mode to parse next
// Fennel does not support '' and [[]] strings
#[derive(PartialEq, Eq)]
enum ParseMode {
    Raw,
    Fennel,
    Lua,
    InlineMath,
    BlockMath,
}

pub(crate) fn template(
    lua: &Lua,
    content: &str,
    name: &str,
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
            }
            // block math
            else if c == '<' && chars.as_str().starts_with("?$$") {
                mode = ParseMode::BlockMath;
                chars.next();
                chars.next();
            } else if c == '<' && chars.as_str().starts_with("?$") {
                mode = ParseMode::InlineMath;
                chars.next();
                chars.next();
            }
            // inline math
            // else, simply push the character
            else {
                out.push(c);
            }
        } else if mode == ParseMode::Lua {
            // TODO: keep track of number of whitespace so we can push that to the front, in order to set the line number
            let mut code = String::new();

            // parse the string
            while let Some((_, c)) = chars.next() {
                // escaped ??>, emit it
                if c == '?' && chars.as_str().starts_with("?>") {
                    chars.next();
                    chars.next();
                    code.push_str("?>");
                }
                // closing ?>, stop
                else if c == '?' && chars.as_str().starts_with(">") {
                    chars.next();
                    break;
                } else {
                    code.push(c);
                }
            }

            // run code
            let result: Value = lua.load(code).set_name(format!("={name}")).eval()?;

            // string, numbers or booleans can be embedded directly
            if result.is_string() || result.is_number() || result.is_boolean() {
                out.push_str(&result.to_string()?);
            }

            // functions and tables can be called, so run them later
            if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }

            // return to normal parsing
            mode = ParseMode::Raw;
        } else if mode == ParseMode::Fennel {
            let mut code = String::new();

            // parse the string
            while let Some((_, c)) = chars.next() {
                // escaped ??>, emit it
                if c == '?' && chars.as_str().starts_with("?>") {
                    chars.next();
                    chars.next();
                    code.push_str("?>");
                }
                // closing ?>, stop
                else if c == '?' && chars.as_str().starts_with(">") {
                    chars.next();
                    break;
                } else {
                    code.push(c);
                }
            }

            // run code
            let result: Value = lua
                .load(chunk!(require("fennel").eval($code)))
                .set_name(format!("={name}"))
                .eval()?;

            // string, numbers or booleans can be embedded directly
            if result.is_string() || result.is_number() || result.is_boolean() {
                out.push_str(&result.to_string()?);
            }

            // functions and tables can be called, so run them later
            if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }

            // return to normal parsing
            mode = ParseMode::Raw;
        } else if mode == ParseMode::InlineMath {
            let mut math = String::new();

            // parse the string
            while let Some((_, c)) = chars.next() {
                // closing ?>, stop
                if c == '$' && chars.as_str().starts_with("?>") {
                    chars.next();
                    chars.next();
                    break;
                } else {
                    math.push(c);
                }
            }

            let mathml = latex_to_mathml(&math, DisplayStyle::Inline)
                .into_lua_err()
                .context("Failed to compile math")?;
            out.push_str(&mathml);

            // return to normal parsing
            mode = ParseMode::Raw;
        } else if mode == ParseMode::BlockMath {
            let mut math = String::new();

            // parse the string
            while let Some((_, c)) = chars.next() {
                // closing ?>, stop
                if c == '$' && chars.as_str().starts_with("$?>") {
                    chars.next();
                    chars.next();
                    chars.next();
                    break;
                } else {
                    math.push(c);
                }
            }

            let mathml = latex_to_mathml(&math, DisplayStyle::Block)
                .into_lua_err()
                .context("Failed to compile math")?;
            out.push_str(&mathml);

            // return to normal parsing
            mode = ParseMode::Raw;
        }
    }

    Ok((out, functions))
}
