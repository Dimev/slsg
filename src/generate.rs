use mlua::{Error, ErrorContext, ExternalResult, Lua, Result, Value};
use std::path::Path;

use crate::stdlib::stdlib;

/// Generate the site from the given directory or lua file
pub(crate) fn generate(path: &Path, dev: bool) -> Result<()> {
    let lua = unsafe { Lua::unsafe_new() };

    // load our custom functions
    let internal = stdlib(&lua)?;

    // whether we are in dev mode
    internal.set("dev", dev)?;

    // add custom functions to global scope
    lua.globals().set("internal", internal)?;

    // load our standard library
    let stdlib: Value = lua
        .load(include_str!("stdlib.lua"))
        .set_name("stdlib.lua")
        .call(())?;

    // unload our custom functions as they are no longer needed in the global scope
    lua.globals().set("internal", Value::Nil)?;

    // add stdlib to the globals
    lua.globals().set("site", stdlib)?;

    // load the script
    let script = if path.is_dir() {
        let code = std::fs::read_to_string(path.join("main.lua"))
            .into_lua_err()
            .context(format!(
                "Failed to load the file {:?}",
                path.join("main.lua")
            ))?;

        // move to the directory the script is in for lua to work properly
        std::env::set_current_dir(path)
            .into_lua_err()
            .context("Failed to change directory to the given path")?;

        code
    } else {
        let code = std::fs::read_to_string(path)
            .into_lua_err()
            .context(format!("Failed to load the file {:?}", path))?;

        // move to the directory the script is in for lua to work properly
        std::env::set_current_dir(path.parent().ok_or(Error::external(format!(
            "No parent directory for path {:?}",
            path
        )))?)
        .into_lua_err()
        .context("Failed to change directory to the given path")?;

        code
    };

    // run the script
    lua.load(script).set_name(path.to_string_lossy()).exec()?;

    // TODO: read out emitted files

    Ok(())
}

// TODO: report error
