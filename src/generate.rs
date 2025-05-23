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

const INDEX_FILES: &[&str] = &[
    "index.htm",
    "index.html",
    "index.lua.htm",
    "index.fnl.htm",
    "index.lua.html",
    "index.fnl.html",
    "index.mmk",
    "index.lua.mmk",
    "index.fnl.mmk",
];

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

    // files to process, in that order
    let mut process = Vec::new();

    // depth first traversal of the directories
    let mut stack = vec![PathBuf::from(".")];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            let mut index = None;

            // directory? recurse
            for path in fs::read_dir(path)
                .into_lua_err()
                .context("Could not read directory")?
            {
                let path = path
                    .into_lua_err()
                    .context("Failed to read directory entry")?;

                // take depending on type
                if path
                    .file_name()
                    .to_str()
                    .map(|x| INDEX_FILES.contains(&x))
                    .unwrap_or(false)
                {
                    if index.is_none() {
                        index = Some(path.path())
                    } else {
                        return Err(mlua::Error::external(format!(
                            "Double index file found in directory `{:?}`",
                            path
                        )));
                    }
                } else if path
                    .file_type()
                    .into_lua_err()
                    .context("Failed to get file type")?
                    .is_file()
                {
                    files.push(path.path());
                } else {
                    dirs.push(path.path());
                }
            }

            // push all to stack
            // stack, so this is reversed
            // index last, if any, in order to build an index from all other files in the directory
            stack.extend(index.into_iter());

            // files go after directories
            stack.extend(files.into_iter());

            // directories first
            stack.extend(dirs.into_iter());
        } else if path.is_file() {
            process.push(path);
        }
    }

    println!("{:?}", process);

    // final files
    let mut files = BTreeMap::new();

    for path in process {
        files.insert(
            path.strip_prefix("./")
                .into_lua_err()
                .context("Failed to strip ./, this shouldn't happen")?
                .to_path_buf(),
            fs::read(path.clone())
                .into_lua_err()
                .context(format!("Failed to read `{:?}`", path))?,
        );
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
