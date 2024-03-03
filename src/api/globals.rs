use std::{
    cell::RefCell,
    fs::{self},
    path::Path,
    rc::Rc,
};

use mlua::Lua;
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::highlighted_html_for_string,
    parsing::{SyntaxReference, SyntaxSet},
};

use super::{file::File, script};

/// Load all program globals into the lua globals
pub(crate) fn load_globals(
    lua: &Lua,
    path: impl AsRef<Path>,
    debug: bool,
) -> Result<Rc<RefCell<Vec<String>>>, anyhow::Error> {
    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // escape html
    // TODO let html_escape = lua.create_function(|_, text: String| Ok())?;

    // syntax highlighting
    let highlight = lua.create_function(
        |_, (text, language, line_numbers): (String, String, bool)| {
            let syntax = SyntaxSet::load_defaults_newlines();
            let themes = ThemeSet::load_defaults();
            //let highlighter = highlighted_html_for_string(&text, &syntax, &themes["base16-ocean.dark"])
            Ok("mogus")
        },
    )?;

    // config
    // TODO

    // warn function
    let warnings = Rc::new(RefCell::new(Vec::new()));

    // require function
    let path = path.as_ref().to_owned();
    let require = lua.create_function(move |lua, script: String| {
        let path = path.join("scripts").join(&script);
        let code = fs::read_to_string(path).map_err(mlua::Error::external)?;
        lua.load(code)
            .set_name(format!("scripts/{}", script))
            .exec()
    })?;

    // load
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("debug", debug)?;
    lua.globals().set("yassg", table)?;
    lua.globals().set("require", require)?;

    Ok(warnings)
}
