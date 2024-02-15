mod api;

use std::{
    collections::HashMap,
    env::{current_dir, set_current_dir},
    fs::read_to_string,
    path::PathBuf,
};

use anyhow::anyhow;
use api::{directory::Directory, script::Script};
use clap::Parser;
use mlua::Lua;

#[derive(Parser)]
struct Args {
    /// directory to the site.lua file of the site to build, current working directory by default
    #[clap(short, long)]
    dir: Option<PathBuf>,

    /// directory to write to, public/ by default
    #[clap(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // path to load from
    let path = args
        .dir
        .map(|x| Ok(x) as Result<PathBuf, anyhow::Error>)
        .unwrap_or_else(|| {
            let dir = current_dir()?;

            // go up to find the dir containing the site.toml file
            Ok(dir
                .ancestors()
                .find(|x| x.join("site.toml").exists())
                .ok_or_else(|| anyhow!("No site.toml found in the current directory or ancestors ({:?})", dir))?
                .to_path_buf())
        })?
        .join("site.toml");

    // load the config
    let config = read_to_string(&path)?;

    // set active dir to the path where we are
    set_current_dir(path.parent().unwrap())?;

    // start lua
    let lua = Lua::new();

    // load the config to the global scope

    // load the library
    lua.load(include_str!("lib.lua"))
        .set_name("builtin://stdlib.lua")
        .exec()?;

    // load the static files
    let static_files = Directory::empty();

    // load the styles
    let styles = HashMap::new();

    // load the root script
    let script = Script::load(&"site/", &lua, static_files, styles)?;

    // load the settings into the lua environment
    // TODO

    // run the script
    let page = script.run(&lua)?;

    // store the tree
    page.write_to_directory("public/")?;

    Ok(())
}
