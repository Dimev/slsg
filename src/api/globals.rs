use std::{cell::RefCell, fs, path::Path, rc::Rc};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{Lua, Value};

use super::{file::File, highlight::Languages};

/// Load all program globals into the lua globals
pub(crate) fn load_globals(
    lua: &Lua,
    path: impl AsRef<Path>,
    debug: bool,
) -> Result<Rc<RefCell<Vec<String>>>, anyhow::Error> {
    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // convert tex math to mathml
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

    // syntax highlighting
    let highlights = Languages::load(&path.as_ref().join("highlighting/"))?;
    let highlights_cloned = highlights.clone();
    let has_lang = lua.create_function(move |_, language: String| Ok(highlights_cloned.exists(&language)))?;
    let highlights_cloned = highlights.clone();
    let highlight = lua.create_function(
        move |_, (text, language, class_prefix): (String, String, Option<String>)| {
            highlights
                .highlight_html(&text, &language, class_prefix)
                .map_err(mlua::Error::external)
        },
    )?;

    let highlight_ast = lua.create_function(move |lua, (text, language): (String, String)| {
        let ranges = highlights_cloned
            .highlight(&text, &language)
            .map_err(mlua::Error::external)?;

        let table = lua.create_table()?;

        for range in ranges {
            let t = lua.create_table()?;
            t.set("text", range.text)?;
            t.set("style", range.style)?;
            table.push(t)?;
        }

        Ok(table)
    })?;
    // warn function
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

    // require function
    let path_owned = path.as_ref().to_owned();
    let require = lua.create_function(move |lua, script: String| {
        let path = path_owned.join("scripts").join(&script);
        let code = fs::read_to_string(path).map_err(mlua::Error::external)?;
        let function = lua
            .load(code)
            .set_name(format!("scripts/{script}"))
            .into_function()?;
        lua.load_from_function::<Value>(&format!("scripts/{script}"), function)
    })?;

    // standard lib
    lua.load(include_str!("lib.lua"))
        .set_name("builtin://stdlib.lua")
        .exec()?;

    // load
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("debug", debug)?;
    table.set("highlightCodeHtml", highlight)?;
    table.set("highlightCodeAst", highlight_ast)?;
    table.set("highlightExists", has_lang)?;
    table.set("latexToMathml", mathml)?;
    lua.globals().set("site", table)?;
    lua.globals().set("require", require)?;

    Ok(warnings)
}
