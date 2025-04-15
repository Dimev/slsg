use mlua::{
    Error, ErrorContext, ExternalError, ExternalResult, IntoLua, Lua, MultiValue, Result, Table,
    Value,
};
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs::{self, File, FileType},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{luamark::Luamark, message::notify, stdlib::stdlib};

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

/// Read a directory for luamark files
fn read_directory(lua: &Lua, path: &Path) -> Result<()> {
    // read for files
    for entry in path
        .read_dir()
        .into_lua_err()
        .context("Failed to read directory")?
    {
        let entry = entry
            .into_lua_err()
            .context("Failed to read directory entry")?;

        // is a luamark file?
        if entry.file_type()?.is_file()
            && entry.path().extension().map(OsStr::to_str).flatten() == Some("lmk")
        {
            // read
            let content = fs::read_to_string(entry.path())
                .into_lua_err()
                .context("Failed to read file")?;

            // parse
            //let luamark = Luamark::parse(&content)?;

            // This will work by manually calling the stdlib API
            // macros are function calls that are resolved when rendering to html
            // this is done by making the table they are part of empty before parsing,
            // then adding things to it after the templates are loaded
        } else if entry.file_type()?.is_dir() && entry.path().join("index.lmk").exists() {
            // is there a luamark file in the directory?
            // read
            let content = fs::read_to_string(entry.path().join("index.lmk"))
                .into_lua_err()
                .context("Failed to read file")?;

            // parse
            //let luamark = Luamark::parse(&content)?;

            // This will work by manually calling the stdlib API
            // macros are function calls that are resolved when rendering to html
            // this is done by making the table they are part of empty before parsing,
            // then adding things to it after the templates are loaded
            println!("lmk found: {:?}", entry.path());
        } else if entry.file_type()?.is_dir() {
            // recurse to the other directories
            read_directory(lua, &entry.path())?
        }
    }

    Ok(())
}

/// Generate the site from the ./main.lua file
pub(crate) fn generate(dev: bool) -> Result<HashMap<PathBuf, Output>> {
    // set up lua
    let lua = unsafe { Lua::unsafe_new() };

    // dev mode?
    lua.globals().set("development", dev)?;

    // output
    let output = lua.create_table()?;

    // and standard library
    stdlib(&lua, &output)?;

    // and read the files
    read_directory(&lua, &PathBuf::from("./"))?;

    // load arguments
    let args: Vec<String> = env::args().skip_while(|x| x != "--").skip(1).collect();

    // convert the arguments
    let mut arguments = MultiValue::new();
    for arg in args {
        arguments.push_back(arg.into_lua(&lua)?);
    }

    // build the templates

    // load the output table
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

        files.insert(PathBuf::from(key), value);
    }

    Ok(files)
}
