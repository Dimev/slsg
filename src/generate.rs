use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{Lua, LuaOptions, StdLib, Value};
use std::{fs, path::Path};

use crate::file::File;

/// Generate the site from the given lua file
pub async fn generate(path: &Path) -> anyhow::Result<HashMap<String, File>> {
    // lua
    let lua = Lua::new_with(
        StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH,
        LuaOptions::new(),
    )?;

    // set up our own require function to only load files from this directory
    let path_owned = path.to_owned();
    let require = lua.create_function(move |lua, script: String| {
        let path = path_owned.join("scripts").join(&script);
        let code = fs::read_to_string(path).map_err(mlua::Error::external)?;
        let function = lua.load(code).into_function()?;
        lua.load_from_function::<Value>(&script, function)
    })?;

    lua.globals().set("require", require)?;

    // load our library functions
    let lib = lua.create_table()?;

    // TODO
    // list directories
    // list all directories

    // list files
    // list all files

    // read file
    // TODO: file api(?)

    // read toml
    // read yaml
    // read json
    // read bibtex

    // read and eval mdl

    // add highlighter

    // highlight code
    // highlight code to ast

    // highlight latex math as mathml

    // minify/bundle(?)

    // dev mode?

    lua.globals().set("site", lib)?;

    // set our own warning function
    let warnings = Rc::new(RefCell::new(Vec::<String>::new()));
    let warnings_cloned = warnings.clone();
    lua.set_warning_function(move |lua, text, _| {
        // Get the stack trace
        let mut trace = Vec::new();
        for frame in (0..).map_while(|i| lua.inspect_stack(i)) {
            let name = frame.source().short_src.unwrap_or("?".into());
            let what = frame.names().name_what;
            let func = frame
                .names()
                .name
                .unwrap_or(if frame.source().what == "main" {
                    "main chunk".into()
                } else {
                    "?".into()
                });
            let line = frame.curr_line();
            let line = if line < 0 {
                String::new()
            } else {
                format!(":{}", line)
            };
            if let Some(what) = what {
                trace.push(format!("\t{}{}: in {} '{}'", name, line, what, func));
            } else {
                trace.push(format!("\t{}{}: in {}", name, line, func));
            }
        }

        // give the stack trace to the warnings
        let warning = format!(
            "runtime warning: {}\nstack traceback:\n{}",
            text,
            trace.join("\n")
        );
        warnings_cloned.borrow_mut().push(warning);
        Ok(())
    });

    // load our own standard library
    let _: Value = lua.load_from_function(
        "slsg",
        lua.load(include_str!("stdlib.lua")).into_function()?,
    )?;

    // run file
    let script = async_std::fs::read_to_string(path).await?;
    lua.load(script)
        .set_name("site.lua")
        .eval_async()
        .await
        .map_err(|x| x.into())
}

// TODO: mdl
// file consists of paragraphs
// you can do some standard markdown functions
// per paragraph, a function is called to eval the paragraph
// => evals an inline lua function
// ==> evals a block lua function (outside paragraph)
