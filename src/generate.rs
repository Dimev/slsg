use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::anyhow;
use fancy_regex::Regex;
use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{Lua, LuaOptions, StdLib, Value};
use std::{fs, path::Path};

use crate::file::File;

// rule for the highlighter
enum HighlightRule {
    Raw(String, String),
    Compiled(String, Regex),
}

/// Generate the site from the given lua file
pub fn generate(path: &Path, dev: bool) -> anyhow::Result<HashMap<String, File>> {
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

    // language highlighters
    let highlighters = Rc::new(RefCell::new(HashMap::<String, Vec<HighlightRule>>::new()));

    // load our library functions
    let lib = lua.create_table()?;

    // TODO
    // list directories
    // list all directories

    // list files
    // list all files

    // read file
    // TODO: file api(?)

    // new file

    // read toml
    // read yaml
    // read json
    // read bibtex

    // read and eval mdl

    // add highlighters
    let highlighters_cloned = highlighters.clone();
    let add_highlighters = lua.create_function(move |_, text: String| {
        // parse the highlighter
        let raw = toml::from_str::<HashMap<String, Vec<(String, String)>>>(&text)
            .map_err(mlua::Error::external)?;
        let mut highlight = highlighters_cloned.borrow_mut();

        // add the language to the highlighters
        for (key, value) in raw.into_iter() {
            highlight.insert(
                key,
                value
                    .into_iter()
                    .map(|(rule, regex)| HighlightRule::Raw(rule, regex))
                    .collect(),
            );
        }

        Ok(())
    })?;

    // highlight code
    let highlighters_cloned = highlighters.clone();
    let highlight_code = lua.create_function(move |_, (lang, code): (String, String)| {
        // get the language
        let rules = highlighters_cloned
            .borrow()
            .get(&lang)
            .ok_or(mlua::Error::external(anyhow!(
                "Language {lang} not in highlighters!"
            )));

        // highlight

        Ok("sus mogus")
    })?;
    // highlight code to ast

    // highlight latex math as mathml
    let mathml = lua.create_function(|_, (text, inline): (String, Option<bool>)| {
        latex_to_mathml(
            &text,
            if inline.unwrap_or(false) {
                DisplayStyle::Inline
            } else {
                DisplayStyle::Block
            },
        )
        .map_err(mlua::Error::external)
    })?;

    // minify/bundle(?)

    // dev mode?
    lib.set("dev", dev)?;

    // add all to the site
    lib.set("latex2Mathml", mathml)?;
    lib.set("addHighlighters", add_highlighters)?;
    lib.set("highlightCodeToHtml", highlight_code)?;

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
    let script = fs::read_to_string(path)?;
    lua.load(script)
        .set_name("site.lua")
        .eval()
        .map_err(|x| x.into())
}

// TODO: mdl
// file consists of paragraphs
// you can do some standard markdown functions
// per paragraph, a function is called to eval the paragraph
// => evals an inline lua function
// ==> evals a block lua function (outside paragraph)
