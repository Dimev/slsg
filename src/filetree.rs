use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

/// A single item in the file tree
#[derive(Debug)]
pub(crate) enum FileNode {
    /// A lua file to process, as well as all of it's siblings
    Lua {
        code: String,
        subs: HashMap<String, FileNode>,
    },

    /// An asset file
    Asset { path: PathBuf },

    /// A directory
    Dir { subs: HashMap<String, FileNode> },
}

impl FileNode {
    /// Try to load all files into the tree
    pub(crate) fn load<P: AsRef<Path>>(root: P) -> Option<FileNode> {
        load_tree(root)
    } 
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
    let siblings = assetfiles.map(|x| {
        (
            x.file_name()
                .to_os_string()
                .into_string()
                .expect("Could not convert OsString to string!"),
            FileNode::Asset { path: x.path() },
        )
    });

    // load any lua files
    let scripts = luafiles.map(|x| {
        let code = read_to_string(x.path()).expect("Failed to read code in file!");
        (
            x.file_name()
                .to_os_string()
                .into_string()
                .expect("Could not convert OsString into string!"),
            FileNode::Lua {
                code,
                subs: HashMap::new(),
            },
        )
    });

    // create the subdirs, as well as the lua files and assets
    let subs = subdirs
        .map(|x| {
            let node = load_tree(x.path()).expect("Failed to load subtree!");
            (
                x.file_name()
                    .into_string()
                    .expect("Could not convert OsString into string!"),
                node,
            )
        })
        .chain(scripts)
        .chain(siblings)
        .collect::<HashMap<String, FileNode>>();

    // if there is an index.lua, load it and return this as a lua file
    if let Ok(code) = read_to_string(root.as_ref().join("index.lua")) {
        Some(FileNode::Lua { code, subs })

    // if not, this is a directory
    } else {
        Some(FileNode::Dir { subs })
    }
}
