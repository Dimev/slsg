mod api;

use std::{
    env::{current_dir, set_current_dir},
    fs::read_to_string,
    path::PathBuf,
};

use anyhow::anyhow;
use api::{directory::Directory, script::Script};
use clap::Parser;
use mlua::Lua;

use crate::api::globals::load_globals;

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
                .ok_or_else(|| {
                    anyhow!(
                        "No site.toml found in the current directory or ancestors ({:?})",
                        dir
                    )
                })?
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

    // load the static files
    let static_files = Directory::load_static("static/", &lua)?;

    // load the styles
    // TODO: separate file for this
    let styles = lua.create_table()?;

    // load the settings into the lua environment
    // TODO

    // load the globals
    load_globals(&lua)?;

    // load the root script
    // TODO: make this load directly into lua tables
    let script = Script::load(&"site/", &lua, &static_files, &styles)?;

    // run the script
    let page = script.run()?;

    // get the warnings
    let warnings: Vec<String> = lua.globals().get("debugWarnings")?;

    // print the warnings
    for warning in warnings.into_iter() {
        println!("warning: {}", warning);
    }

    // store the tree
    page.write_to_directory("public/")?;

    // print the pages
    for (path, file) in page.to_hashmap("") {
        println!("{:?} -> {:?}", path, file);
    }

    Ok(())
}
