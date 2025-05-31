use std::{collections::VecDeque, iter::repeat};

use latex2mathml::{DisplayStyle, latex_to_mathml};
use mlua::{ErrorContext, ExternalResult, Lua, Result, Value, chunk};
use unicode_width::UnicodeWidthStr;

use crate::conf::Config;

pub(crate) fn template(
    lua: &Lua,
    content: &str,
    name: &str,
    conf: &Config,
) -> Result<(String, VecDeque<Value>)> {
    let mut out = String::with_capacity(content.len());
    let mut functions = VecDeque::new();
    let mut chars = content.char_indices();

    while let Some((_, c)) = chars.next() {
        // open tag and a ?lua? parse lua
        if c == '<' && chars.as_str().starts_with("?lua") {
            if !conf.lua {
                return Err(mlua::Error::external(
                    "Found a lua code block, but lua is not enabled in `site.conf`",
                ));
            }
            chars.next();
            chars.next();
            chars.next();
            chars.next();

            // add extra whitespace to the start to have the line numbers match up
            let position = chars.offset();
            let lines = content[..position].chars().filter(|x| *x == '\n').count();
            let width = content[..position]
                .split_once('\n')
                .map(|x| x.1)
                .unwrap_or(&content[..position])
                .width();
            let mut code =
                String::from_iter(repeat('\n').take(lines).chain(repeat(' ').take(width)));

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
        }
        // open tag and a ?fnl? parse fennel
        else if c == '<' && chars.as_str().starts_with("?fnl") {
            if !conf.fennel {
                return Err(mlua::Error::external(
                    "Found a fennel code block, but fennel is not enabled in `site.conf`",
                ));
            }
            chars.next();
            chars.next();
            chars.next();
            chars.next();

            // add extra whitespace to the start to have the line numbers match up
            let position = chars.offset();
            let lines = content[..position].chars().filter(|x| *x == '\n').count();
            let width = content[..position]
                .split_once('\n')
                .map(|x| x.1)
                .unwrap_or(&content[..position])
                .width();
            let mut code =
                String::from_iter(repeat('\n').take(lines).chain(repeat(' ').take(width)));

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
                .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })))
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
        }
        // block math
        else if c == '<' && chars.as_str().starts_with("?$$") {
            chars.next();
            chars.next();
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
        }
        // inline math
        else if c == '<' && chars.as_str().starts_with("?$") {
            chars.next();
            chars.next();
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
        }
        // else, simply push the character
        else {
            out.push(c);
        }
    }

    Ok((out, functions))
}
