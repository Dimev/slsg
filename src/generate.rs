use mlua::{Error, ErrorContext, ExternalResult, IntoLua, Lua, MultiValue, Result, Table, Value};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{message::notify, stdlib::stdlib};

/// Contain the path to the current directory
pub fn contain_path(path: String) -> Result<PathBuf> {
    // backslashes means it's invalid
    if path.contains('\\') {
        return Err(Error::external(
            "Path contains a `\\`, which is not allowed",
        ));
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
                return Err(Error::external(format!(
                    "Path {path:?} tried to escape the directory using `..`"
                )));
            }
        }
        // only advance if this is not the current directory
        else if component != "." {
            resolved.push(component);
        }
    }

    Ok(resolved)
}
/// Output result
pub(crate) enum Output {
    /// Export the data in a string
    Data(Vec<u8>),

    /// Copy a file
    File(PathBuf),

    /// Run a command on a file
    Command {
        command: String,
        arguments: Vec<String>,
    },
}

impl Output {
    pub(crate) fn as_stream<'a>(&'a self) -> std::io::Result<Box<dyn Read + 'a>> {
        Ok(match self {
            Self::Data(vec) => Box::new(vec.as_slice()),
            Self::File(path) => Box::new(File::open(path)?),
            Self::Command { command, arguments } => {
                notify(
                    "Running command",
                    &format!("{command} {}", arguments.join(" ")),
                );
                let stream = Stdio::piped();
                let mut child = Command::new(command)
                    .args(arguments)
                    .stdout(stream)
                    .spawn()?;

                let stdout = child
                    .stdout
                    .take()
                    .ok_or(std::io::Error::other("Failed to take stdout of command!"))?;

                Box::new(stdout)
            }
        })
    }

    pub(crate) fn to_file(&self, path: &Path) -> std::io::Result<()> {
        // ensure the path exists
        fs::create_dir_all(
            path.parent()
                .ok_or(std::io::Error::other("Tried to output a directory"))?,
        )?;

        match self {
            Self::Data(vec) => fs::write(path, vec)?,
            Self::File(original) => {
                fs::copy(original, path)?;
            }
            Self::Command { command, arguments } => {
                notify(
                    "Running command",
                    &format!("{command} {}", arguments.join(" ")),
                );
                let stream = Stdio::piped();
                let mut child = Command::new(command)
                    .args(arguments)
                    .stdout(stream)
                    .spawn()?;

                let mut stdout = child
                    .stdout
                    .take()
                    .ok_or(std::io::Error::other("Failed to take stdout of command!"))?;

                // copy the output over
                let mut new = File::create_new(path)?;
                io::copy(&mut stdout, &mut new)?;
            }
        }
        Ok(())
    }
}

/// Generate the site from the ./main.lua file
pub(crate) fn generate(dev: bool) -> Result<HashMap<PathBuf, Output>> {
    let lua = unsafe { Lua::unsafe_new() };

    // load arguments
    let args: Vec<String> = env::args().skip_while(|x| x != "--").skip(1).collect();

    // load our custom functions
    let internal = stdlib(&lua)?;

    // whether we are in dev mode
    internal.set("dev", dev)?;

    // add the table we read our output from to the global scope
    let output = lua.create_table()?;

    // load our standard library
    let stdlib: Value = lua
        .load(include_str!("stdlib.lua"))
        .set_name("=stdlib.lua")
        .call((internal, &output))?;

    // add stdlib to the globals
    lua.globals().set("site", stdlib)?;

    // get the current directory
    let current_dir = std::env::current_dir()
        .into_lua_err()
        .context("Failed to get the current directory")?;

    // convert the arguments
    let mut arguments = MultiValue::new();
    for arg in args {
        arguments.push_back(arg.into_lua(&lua)?);
    }

    // run the script
    let res: Result<()> = lua.load(PathBuf::from("./main.lua")).call(arguments);

    // reset the current directory in case it changed
    std::env::set_current_dir(current_dir)
        .into_lua_err()
        .context("Failed to reset the current directory")?;

    // emit the error,
    // doing this now in order to ensure we reset the directory
    res.context("Failed to run main.lua")?;

    // read the files we emitted
    let mut files = HashMap::with_capacity(output.len()? as usize);
    for pair in output.pairs() {
        let (key, value): (String, Table) =
            pair.context("Failed to read pair of the emitted files")?;

        let value = match value.get::<String>("type")?.as_str() {
            "data" => Output::Data(value.get::<mlua::String>("data")?.as_bytes().to_owned()),
            "file" => Output::File(PathBuf::from(value.get::<String>("original")?)),
            "command" => Output::Command {
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
