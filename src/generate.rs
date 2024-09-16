use mlua::{Error, ErrorContext, ExternalResult, Lua, Result, Table, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::stdlib::stdlib;

/// Output result
pub(crate) enum Output {
    /// Export the data in a string
    Data(Vec<u8>),

    /// Copy a file
    File(PathBuf),

    /// Run a command on a file
    Command {
        original: PathBuf,
        command: String,
        placeholder: Vec<u8>,
    },
}

fn contain_path(path: String) -> Result<PathBuf> {
    // backslashes means it's invalid
    if path.contains('\\') {
        return Err(Error::external("Path contains a \\, which is not allowed"));
    }

    // trim any initial /
    let path = path.trim_start_matches('/');

    // parts of the path
    let mut resolved = PathBuf::new();

    // go over all parts of the original path
    for component in path.split('/') {
        if component == ".." {
            // break if this path is not valid due to not being able to drop a component
            if !resolved.pop() {
                return Err(Error::external("Path tries to escape directory using `..`"));
            }
        }
        // only advance if this is not the current directory
        else if component != "." {
            resolved.push(component);
        }
    }

    Ok(resolved)
}

/// Generate the site from the given directory or lua file
pub(crate) fn generate(path: &Path, dev: bool) -> Result<HashMap<PathBuf, Output>> {
    let lua = unsafe { Lua::unsafe_new() };

    // load our custom functions
    let internal = stdlib(&lua)?;

    // whether we are in dev mode
    internal.set("dev", dev)?;

    // add custom functions to global scope
    lua.globals().set("internal", internal)?;

    // add the table we read our output from to the global scope
    let output = lua.create_table()?;
    lua.globals().set("output", &output)?;

    // load our standard library
    let stdlib: Value = lua
        .load(include_str!("stdlib.lua"))
        .set_name("stdlib.lua")
        .call(())?;

    // unload our custom functions as they are no longer needed in the global scope
    lua.globals().set("internal", Value::Nil)?;

    // unload the output table
    lua.globals().set("output", Value::Nil)?;

    // add stdlib to the globals
    lua.globals().set("site", stdlib)?;

    // current directory so we can restore it
    let current_dir = std::env::current_dir()
        .into_lua_err()
        .context("Failed to get current directory, making it impossible to restore later on")?;

    // load the script
    let (script, name) = if path.is_dir() {
        let code = std::fs::read_to_string(path.join("main.lua"))
            .into_lua_err()
            .context(format!(
                "Failed to load the file {:?}",
                path.join("main.lua")
            ))?;

        let name = path.join("main.lua").into_os_string();

        // move to the directory the script is in for lua to work properly
        std::env::set_current_dir(path)
            .into_lua_err()
            .context("Failed to change directory to the given path")?;

        (code, name)
    } else {
        let code = std::fs::read_to_string(path)
            .into_lua_err()
            .context(format!("Failed to load the file {:?}", path))?;

        let name = path.as_os_str().to_os_string();

        // move to the directory the script is in for lua to work properly
        std::env::set_current_dir(path.parent().ok_or(Error::external(format!(
            "No parent directory for path {:?}",
            path
        )))?)
        .into_lua_err()
        .context("Failed to change directory to the given path")?;

        (code, name)
    };

    // run the script
    lua.load(script).set_name(name.to_string_lossy()).exec()?;

    // read the files we emitted
    let mut files = HashMap::with_capacity(output.len()? as usize);
    for pair in output.pairs() {
        let (key, value): (String, Table) =
            pair.context("Failed to read pair of the emitted files")?;

        let value = match value.get::<&str, String>("type")?.as_str() {
            "data" => Output::Data(
                value
                    .get::<&str, mlua::String>("data")?
                    .as_bytes()
                    .to_owned(),
            ),
            "file" => Output::File(contain_path(value.get("original")?)?),
            "command" => Output::Command {
                original: contain_path(value.get("original")?)?,
                command: value.get("command")?,
                placeholder: value
                    .get::<&str, mlua::String>("placeholder")?
                    .as_bytes()
                    .to_owned(),
            },
            _ => {
                return Err(Error::external(
                    "Unknown type of output in the output table",
                ))
            }
        };

        files.insert(contain_path(key)?, value);
    }

    // restore the current directory
    std::env::set_current_dir(current_dir)
        .into_lua_err()
        .context("Failed to restore current directory")?;

    Ok(files)
}
