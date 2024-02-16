use mlua::Lua;

use super::{file::File, page::Page};

/// Load all program globals into the lua globals
pub(crate) fn load_globals(lua: &Lua) -> Result<(), anyhow::Error> {
    // create a new page
    let page = lua.create_function(|_, _: ()| Ok(Page::new()))?;

    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // config
    // TODO

    // load
    lua.globals().set("page", page)?;
    lua.globals().set("file", file)?;

    // standard lib
    lua.load(include_str!("lib.lua"))
        .set_name("builtin://stdlib.lua")
        .exec()?;

    Ok(())
}
