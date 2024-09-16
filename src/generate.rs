use mlua::{Error, ErrorContext, Lua, Result, Table, Value};
use std::{collections::HashMap, path::PathBuf};

use crate::stdlib::stdlib;

/// Output result
pub(crate) enum Output {
    /// Export the data in a string
    Data(Vec<u8>),

    /// Copy a file
    File(PathBuf),

    /// Run a command on a file
    Command { original: PathBuf, command: String },
}

/*impl Output {
    pub(crate) fn as_stream(&self, path: &Path) -> Box<dyn Read> {
        match self {
            Self::Data(vec) => Box::new(vec.as_slice()),
            Self::File(local) => Box::new(File::open(path.join(local))),
        }
    }
}*/

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

/// Generate the site from the ./main.lua file
pub(crate) fn generate(dev: bool) -> Result<HashMap<PathBuf, Output>> {
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
        .set_name("=stdlib.lua")
        .call(())?;

    // unload our custom functions as they are no longer needed in the global scope
    lua.globals().set("internal", Value::Nil)?;

    // unload the output table
    lua.globals().set("output", Value::Nil)?;

    // add stdlib to the globals
    lua.globals().set("site", stdlib)?;

    // run the script
    lua.load(PathBuf::from("./main.lua")).exec()?;

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
            },
            _ => {
                return Err(Error::external(
                    "Unknown type of output in the output table",
                ))
            }
        };

        files.insert(contain_path(key)?, value);
    }

    Ok(files)
}
