use mlua::{Error, ErrorContext, ExternalResult, Lua, Result, Table, Value};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
        arguments: Vec<String>,
    },
}

impl Output {
    pub(crate) fn as_stream<'a>(&'a self) -> std::io::Result<Box<dyn Read + 'a>> {
        Ok(match self {
            Self::Data(vec) => Box::new(vec.as_slice()),
            Self::File(path) => Box::new(File::open(path)?),
            Self::Command {
                original,
                command,
                arguments,
            } => {
                // TODO: make the file optional?
                let file = File::open(original)?;
                let stream = Stdio::piped();
                let mut child = Command::new(command)
                    .args(arguments)
                    .stdin(file)
                    .stdout(stream)
                    .spawn()?;

                // TODO: proper error handling
                let stdout = child.stdout.take().unwrap();
                Box::new(stdout)
            }
        })
    }

    pub(crate) fn to_file(&self, path: &Path) -> std::io::Result<()> {
        match self {
            Self::Data(vec) => fs::write(path, vec)?,
            Self::File(original) => {
                fs::copy(original, path)?;
            }
            Self::Command {
                original,
                command,
                arguments,
            } => {
                // TODO: make the file optional?
                let file = File::open(original)?;
                let stream = Stdio::piped();
                let mut child = Command::new(command)
                    .args(arguments)
                    .stdin(file)
                    .stdout(stream)
                    .spawn()?;

                // TODO: proper error handling
                let mut stdout = child.stdout.take().unwrap();
                let mut new = File::open(path)?;
                io::copy(&mut stdout, &mut new)?;

                ()
            }
        }
        Ok(())
    }
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

    // get the current directory
    let current_dir = std::env::current_dir()
        .into_lua_err()
        .context("Failed to get the current directory")?;

    // run the script
    let res = lua.load(PathBuf::from("./main.lua")).exec();

    // reset the current directory in case it changed
    std::env::set_current_dir(current_dir)
        .into_lua_err()
        .context("Failed to reset the current directory")?;

    // emit the error,
    // doing this now in order to ensure we reset the directory
    res?;

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
                arguments: value.get("arguments")?,
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
