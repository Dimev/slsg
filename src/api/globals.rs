use mlua::{Lua, Table};

use super::file::File;

/// Load all program globals into the lua globals
pub(crate) fn load_globals(lua: &Lua, debug: bool) -> Result<(), anyhow::Error> {
    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // escape html
    // TODO let html_escape = lua.create_function(|_, text: String| Ok())?;

    // config
    // TODO

    // load
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("debug", debug)?;
    lua.globals().set("yassg", table)?;

    // standard lib
    lua.load(include_str!("lib.lua"))
        .set_name("builtin://stdlib.lua")
        .exec()?;

    Ok(())
}
