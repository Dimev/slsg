use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use mlua::{ErrorContext, ExternalResult, Lua, Result, chunk};

use crate::conf::Config;

pub(crate) struct Site {
    /// Generated files
    pub files: BTreeMap<PathBuf, Vec<u8>>,

    /// What file to use for 404
    pub not_found: Option<Vec<u8>>,
}

/// Generate the site
/// Assumes that the current directory contains the site.conf file
pub(crate) fn generate() -> Result<Site> {
    // read the config file
    let config = fs::read_to_string("./site.conf")
        .into_lua_err()
        .context("failed to read `site.conf`")?;

    let config = Config::parse(&config)?;

    // set up lua
    let lua = unsafe { Lua::unsafe_new() };

    // if fennel is enabled, add fennel
    if config.fennel {
        let fennel = include_str!("fennel.lua");
        let fennel = lua
            .load(fennel)
            .into_function()
            .context("Failed to load fennel")?;
        lua.load(chunk! {
            package.preload["fennel"] = $fennel
        })
        .exec()
        .context("failed to install fennel")?;
    }

    // final files
    let mut files = BTreeMap::new();

    // depth first traversal of the directories
    let mut stack = vec![PathBuf::from(".")];
    while let Some(path) = stack.pop() {
        println!("{path:?}");
        if path.is_dir() {
            // directory? recurse
            for path in fs::read_dir(path)
                .into_lua_err()
                .context("Could not read directory")?
            {
                // add to the stack
                stack.push(path?.path());
            }
        } else if path.is_file() {
            // file? read
            files.insert(
                path.strip_prefix("./")
                    .into_lua_err()
                    .context("Expected a `./` at the start of the path")?
                    .to_owned(),
                fs::read(path)
                    .into_lua_err()
                    .context("Failed to read file")?,
            );
        }
    }

    // filter out all paths to ignore
    // with globbing
    // TODO

    // go over all files, process them if needed
    Ok(Site {
        files,
        not_found: None,
    })
    //todo!()
}
