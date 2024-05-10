// TODO: parse lua markdown
// Features:
// comments with --
// # for headings
// *text* for bold
// _text_ for underlined
// `text` for monospace
// ```lang\ntext``` for code
// ~text~ for strikethrough
// $text$ for inline math
// $$text$$ for block math
// @ for inline lua
// @@ for block lua
// paragraphs are seperated by empty lines

use std::collections::HashMap;

use mlua::{Lua, Result, Value};

pub fn eval_mdl<'a>(
    lua: &'a Lua,
    env: HashMap<String, Value<'a>>,
    mut text: &str,
) -> Result<Value<'a>> {
    // current line and column
    let mut line = 1;
    let mut column = 1;

    // paragraphs in this file
    let mut paragraphs = Vec::<Value>::new();

    // current block in the paragraph we are building
    let mut block_entries = Vec::<Value>::new();

    // current string we are building from text in the entries
    let mut string = String::new();

    // parse!
    while !text.is_empty() {
        // empty space
        if text.starts_with(|x: char| x.is_whitespace()) {
            // newline, reset line
            // TODO
        }
        // comment
        else if let Some((_, rest)) = text.split_once("--") {
            // skip it
            text = rest;
            line += 1;
            column = 1;
        }
        // heading
        else if text.starts_with('#') {
            // read all headings
            let mut count = 0;
            while text.starts_with('#') {
                count += 1;
                column += 1;
                text = &text[1..];
            }

            // process them
        }
        // lua
        else if text.starts_with('@') {
            // trim first one
            text = &text[1..];
            column += 1;

            // check if it's block or inline
            let block = if text.starts_with('@') {
                text = &text[1..];
                column += 1;
                true
            } else {
                false
            };

            // read the lua
            let mut code = String::new();

            // TODO: actually read it

            // convert it to code
            let function = lua
                .load(code)
                .set_name(format!("mdl:{line}:{column}"))
                .into_function()?;

            if let Some(fun_env) = function.environment() {
                for (key, value) in env.iter() {
                    fun_env.set(key.as_str(), value)?;
                }
            }

            // save the result
            let result: Value = function.call(())?;

            // push it to the paragraph or block
            if block {
                // TODO: eval
            } else {
                block_entries.push(result);
            }
        }
    }

    Ok(Value::Nil)
}
