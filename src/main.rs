// idea: lua based template thingy
// config in site.toml
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

use std::{
    collections::HashMap,
    env::{self, current_dir, set_current_dir},
    ffi::OsString,
    fmt::Pointer,
    fs::{read_dir, read_to_string, DirEntry, File},
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;
use mlua::{AsChunk, Table};

/// A single item in the page tree
#[derive(Debug)]
enum SiteNode {
    /// Asset, any file that can be included or loaded
    Asset { name: String, path: PathBuf },

    /// Page, with siblings
    Page {
        html: String,
        name: String,
        table: (),
        sibs: HashMap<OsString, SiteNode>,
    },

    /// Lua table
    Table { table: () },

    /// Subdirectory
    Dir { subs: HashMap<OsString, SiteNode> },
}

/// A single item in the file tree
#[derive(Debug)]
enum FileNode {
    /// A lua file to process, as well as all of it's siblings
    Lua {
        code: String,
        name: OsString,
        sibs: HashMap<OsString, FileNode>,
    },

    /// An asset file
    Asset { path: PathBuf },

    /// A directory
    Dir { subs: HashMap<OsString, FileNode> },
}

#[derive(Parser)]
struct Args {
    /// directory to the site.toml file of the site to build, current working directory by default
    #[clap(short, long)]
    dir: Option<PathBuf>,

    /// directory to write to, public/ by default
    #[clap(short, long)]
    output: Option<PathBuf>,
}

// load the file tree
fn load_tree<P: AsRef<Path>>(root: P) -> Option<FileNode> {
    // find all the lua files
    let luafiles = read_dir(&root)
        .expect("Failed to read directory!")
        .filter_map(|x| x.ok())
        .filter(|x| x.file_type().expect("Failed to get file type").is_file())
        .filter(|x| x.file_name() != "index.lua")
        .filter(|x| x.path().extension().map(|x| x == "lua").unwrap_or(false));

    // find all the other files
    let assetfiles = read_dir(&root)
        .expect("Failed to read directory!")
        .filter_map(|x| x.ok())
        .filter(|x| x.file_type().expect("Failed to get file type").is_file())
        .filter(|x| x.path().extension().map(|x| x != "lua").unwrap_or(true));

    // find all the subdirectories
    let subdirs = read_dir(&root)
        .expect("Failed to read directory!")
        .filter_map(|x| x.ok())
        .filter(|x| x.file_type().expect("Failed to get file type").is_dir());

    // load the assets
    let siblings = assetfiles.map(|x| (x.file_name(), FileNode::Asset { path: x.path() }));

    // load any lua files
    let scripts = luafiles.map(|x| {
        let code = read_to_string(x.path()).unwrap();
        (
            x.file_name(),
            FileNode::Lua {
                code,
                name: x
                    .path()
                    .file_stem()
                    .map(|x| x.to_os_string())
                    .unwrap_or(x.file_name().to_os_string()),
                sibs: HashMap::new(),
            },
        )
    });

    // create the subdirs, as well as the lua files and assets
    let subs = subdirs
        .map(|x| {
            let node = load_tree(x.path()).expect("Failed to load subtree!");
            (x.file_name(), node)
        })
        .chain(scripts)
        .chain(siblings)
        .collect::<HashMap<OsString, FileNode>>();

    // if there is an index.lua, load it and return this as a lua file
    if let Ok(code) = read_to_string(root.as_ref().join("index.lua")) {
        Some(FileNode::Lua {
            code,
            name: root
                .as_ref()
                .file_name()
                .map(|x| x.to_os_string())
                .unwrap_or(OsString::from("index.lua")),
            sibs: subs,
        })

    // if not, this is a directory
    } else {
        Some(FileNode::Dir { subs })
    }
}

fn main() {
    let args = Args::parse();

    // path to load from
    let path =
        args.dir
            .unwrap_or_else(|| {
                let dir = current_dir().expect("Could not run from the current working directory!");

                // go up to find the dir containing the site.toml file
                dir.ancestors()
            .find(|x| x.join("site.toml").exists())
            .expect("Could not find a site.toml file in this directory or any of it's ancestors!")
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
    let filetree = load_tree("content/");

    println!("{:?}", filetree);
}
