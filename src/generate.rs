use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use mlua::{ErrorContext, ExternalResult, Lua, Result, chunk};

use crate::conf::Config;

/// Generate the site
/// Assumes that the current directory contains the site.conf file
fn generate() -> Result<BTreeMap<PathBuf, Vec<u8>>> {
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

    // depth first traversal of the directories
    // TODO

    // filter out all paths to ignore
    // with globbing
    // TODO

    // go over all files, process them if needed
    
    todo!()
}
