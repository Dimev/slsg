use std::path::Path;

use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{Error, ErrorContext, ExternalResult, Lua, Result, Value};

/// Generate the site from the given directory or lua file
pub(crate) fn generate(path: &Path, dev: bool) -> Result<()> {
    let lua = unsafe { Lua::unsafe_new() };

    // load our custom functions
    let internal = lua.create_table()?;

    // list files
    // TODO

    // read file
    // TODO

    // emit lua string
    // TODO

    // emit existing file
    // TODO

    // emit file using command
    // TODO

    // latex to mathml
    internal.set(
        "latex_to_mathml",
        lua.create_function(|_, (latex, inline): (String, Option<bool>)| {
            latex_to_mathml(
                &latex,
                if inline.unwrap_or(false) {
                    DisplayStyle::Inline
                } else {
                    DisplayStyle::Block
                },
            )
            .into_lua_err()
            .context("Failed to convert latex to mathml")
        })?,
    )?;

    // whether we are in dev mode
    internal.set("dev", dev)?;

    // add custom functions to global scope
    lua.globals().set("internal", internal)?;

    // load our standard library
    let stdlib: Value = lua
        .load(include_str!("stdlib.lua"))
        .set_name("stdlib.lua")
        .call(())?;

    // unload our custom functions so they are no longer needed
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

        // move to the directory for lua to work properly
        std::env::set_current_dir(path)
            .into_lua_err()
            .context("Failed to change directory to the given path")?;

        code
    } else {
        let code = std::fs::read_to_string(path)
            .into_lua_err()
            .context(format!("Failed to load the file {:?}", path))?;

        // move to the directory for lua to work properly
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

    Ok(())
}
