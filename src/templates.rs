use std::{collections::VecDeque, iter::repeat};

use mlua::{
    Lua, Result,
    Value::{self, Nil},
    chunk,
};
use relative_path::RelativePath;
use unicode_width::UnicodeWidthStr;

use crate::path::{DoubleFileExt, HtmlToIndex};

pub(crate) fn template(
    lua: &Lua,
    content: &str,
    name: &RelativePath,
) -> Result<(String, VecDeque<Value>)> {
    // translated name
    let path = name
        .without_double_ext()
        .ok_or(mlua::Error::external(format!(
            "Expected path `{name}` to have a second `.lua` or `.fnl` extension"
        )))?;
    let path = path.html_to_index().unwrap_or(path);

    // set up environment
    // current file
    lua.globals().set("curfile", name.as_str())?;

    // current directory
    lua.globals()
        .set("curdir", name.parent().map(RelativePath::as_str))?;

    // where this file will be emitted to
    lua.globals().set("curtarget", path.as_str())?;

    // directory this file will be emitted to
    lua.globals()
        .set("curtargetdir", path.parent().map(RelativePath::as_str))?;

    // output string
    let mut out = String::with_capacity(content.len());

    // functions to run after
    let mut functions = VecDeque::new();

    // what to parse
    let mut chars = content.char_indices();

    // parse!
    while let Some((_, c)) = chars.next() {
        // open tag and a ?lua? parse lua
        if c == '<' && chars.as_str().starts_with("?") && name.has_double_ext("lua") {
            chars.next();
            // add extra whitespace to the start to have the line numbers match up
            let position = chars.offset();
            let lines = content[..position].chars().filter(|x| *x == '\n').count();
            let width = content[..position]
                .rsplit_once('\n')
                .map(|x| x.1)
                .unwrap_or(&content[..position])
                .width();
            let mut code =
                String::from_iter(repeat('\n').take(lines).chain(repeat(' ').take(width + 1)));

            // parse the string
            while let Some((_, c)) = chars.next() {
                // closing ?>, stop
                if c == '?' && chars.as_str().starts_with(">") {
                    chars.next();
                    break;
                } else {
                    code.push(c);
                }
            }

            // run code
            let result: Value = lua.load(code).set_name(format!("@{name}")).eval()?;

            // string, numbers or booleans can be embedded directly
            if result.is_string() || result.is_number() || result.is_integer() {
                out.push_str(&lua.coerce_string(result)?.unwrap().to_str()?);
            }
            // boolean
            else if let Some(b) = result.as_boolean() {
                out.push_str(if b { "true" } else { "false" });
            }
            // functions and tables can be called, so run them later
            else if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }
        }
        // open tag and a ?fnl? parse fennel
        else if c == '<' && chars.as_str().starts_with("?") && name.has_double_ext("fnl") {
            chars.next();

            // add extra whitespace to the start to have the line numbers match up
            let position = chars.offset();
            let lines = content[..position].chars().filter(|x| *x == '\n').count();
            let width = content[..position]
                .rsplit_once('\n')
                .map(|x| x.1)
                .unwrap_or(&content[..position])
                .width();
            let mut code =
                String::from_iter(repeat('\n').take(lines).chain(repeat(' ').take(width + 1)));

            // parse the string
            while let Some((_, c)) = chars.next() {
                // closing ?>, stop
                if c == '?' && chars.as_str().starts_with(">") {
                    chars.next();
                    break;
                } else {
                    code.push(c);
                }
            }

            // run code
            let name = name.as_str();
            let result: Value = lua
                .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })))
                .set_name(format!("@{name}"))
                .eval()?;

            // string, numbers or booleans can be embedded directly
            if result.is_string() || result.is_number() || result.is_integer() {
                out.push_str(&lua.coerce_string(result)?.unwrap().to_str()?);
            }
            // boolean
            else if let Some(b) = result.as_boolean() {
                out.push_str(if b { "true" } else { "false" });
            }
            // functions and tables can be called, so run them later
            else if result.is_function() || result.is_table() {
                functions.push_back(result.clone());
            }
        }
        // else, simply push the character
        else {
            out.push(c);
        }
    }

    // unset environment
    lua.globals().set("curfile", Nil)?;
    lua.globals().set("curdir", Nil)?;
    lua.globals().set("curtarget", Nil)?;
    lua.globals().set("curtargetdir", Nil)?;

    Ok((out, functions))
}
