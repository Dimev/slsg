use std::{cell::RefCell, rc::Rc};

use mlua::Lua;

use super::file::File;

/// Load all program globals into the lua globals
pub(crate) fn load_globals(
    lua: &Lua,
    debug: bool,
) -> Result<Rc<RefCell<Vec<String>>>, anyhow::Error> {
    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // escape html
    // TODO let html_escape = lua.create_function(|_, text: String| Ok())?;

    // syntax highlighting
    let highlight = lua.create_function(|_, text: String| Ok("mogus"))?;

    // config
    // TODO

    // warn function
    let warnings = Rc::new(RefCell::new(Vec::new()));

    // load
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("debug", debug)?;
    lua.globals().set("yassg", table)?;

    Ok(warnings)
}
