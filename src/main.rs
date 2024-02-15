// idea: lua based template thingy
// config in site.lua
// css in style/main.scss
// content in content/
// more lua script in lib/
// copied out stuff in static/
// resulting build in public/

// use simple builders in lua to build the html

// how the templating works
// directories with index.lua or .lua files are converted into html if they return a page
// page can render html (with html function), and have subpages (added with sub)
// lua receives subdirs as list of tables
// subdirs are either pages (lua or html), assets (can be loaded with asset function, relative, name can be set or it's the last part in the file), text (markdown), or directories
// assets are NOT deduplicated, and are inserted in the page directory
// static assets can be loaded with static

mod api;
mod assets;
mod filetree;
mod sitefiles;
mod sitetree;

use std::{
    collections::HashMap,
    env::{current_dir, set_current_dir},
    fs::{self, read_to_string},
    path::PathBuf,
};

use api::{
    directory::Directory,
    script::{self, Script},
};
use clap::Parser;
use mlua::Lua;

use crate::filetree::FileNode;
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
        .unwrap_or_else(|| {
            let dir = current_dir().expect("Could not run from the current working directory!");

            // go up to find the dir containing the site.toml file
            dir.ancestors()
                .find(|x| x.join("site.toml").exists())
                .expect(
                    "Could not find a site.toml file in this directory or any of it's ancestors!",
                )
                .to_path_buf()
        })
        .join("site.toml");

    // load it
    // config contains base url?
    // and config settings?
    // do we even need it?
    let site = read_to_string(&path).expect(&format!("Could not open {}", path.display()));

    // set active dir to the path
    set_current_dir(path.parent().unwrap()).expect("Failed to change working directory!");

    // load the tree
    // let filetree = FileNode::load("content/").expect("Failed to load the file tree!");

    // load and convert the sass style

    // start lua
    let lua = Lua::new();

    // load the library
    lua.load(include_str!("lib.lua"))
        .exec()
        .expect("Failed to load stdlib!");

    // load the static files
    let static_files = Directory::empty();

    // load the styles
    let styles = HashMap::new();

    // load the root script
    let script = Script::load(&"content/", &lua, static_files, styles)?;

    // load the settings into the lua environment
    // TODO

    println!("{:?}", script); 
    
    // run the script
    let page = script.run(&lua)?;

    // store the tree
    page.write_to_directory("public/")?;

    // render the tree
    // let tree = filetree.evaluate(&lua);

    // convert it to files
    // fs::remove_dir_all("public/").expect("Failed to remove public dir!");
    // tree.write_to_files(PathBuf::from("public/"));

    // write out the files
    Ok(())
}
