use std::{collections::HashMap, fs, path::Path};

use anyhow::anyhow;
use clap::error::Result;
use mlua::Lua;

use super::{file::File, script::Script};

/// Directory for the file tree
#[derive(Clone, Debug)]
pub(crate) struct Directory<'lua> {
    /// All colocated files
    files: HashMap<String, File>,

    /// All colocated directories
    directories: HashMap<String, Directory<'lua>>,

    /// All colocated scripts
    scripts: HashMap<String, Script<'lua>>,
}

impl<'lua> Directory<'lua> {
    /// Create an empty directory
    pub(crate) fn empty() -> Self {
        Self {
            files: HashMap::new(),
            directories: HashMap::new(),
            scripts: HashMap::new(),
        }
    }

    pub(crate) fn load<P: AsRef<Path>>(
        path: P,
        lua: &'lua Lua,
        static_files: Directory<'lua>,
        styles: HashMap<String, File>,
    ) -> Result<Self, anyhow::Error> {
        // empty
        let mut res = Self::empty();

        // load all files if needed
        for item in fs::read_dir(path)? {
            // read
            let item = item?;

            // if it's a lua file, load is as such
            if item.file_type()?.is_file()
                && item.path().extension().map(|x| x == "lua").unwrap_or(false)
            {
                let script = Script::load(&item.path(), lua, static_files.clone(), styles.clone())?;
                let name = item
                    .path()
                    .file_stem()
                    .ok_or_else(|| anyhow!("Filepath {:?} does not have a file stem", item.path()))?
                    .to_str()
                    .ok_or_else(|| {
                        anyhow!(
                            "The file stem of file {:?} can't be converted to utf-8",
                            item.path()
                        )
                    })?
                    .to_owned();

                // insert it
                res.scripts.insert(name, script);
            }
            // if it's a normal file, add it
            else if item.file_type()?.is_file() {
                let name = item
                    .file_name()
                    .into_string()
                    .map_err(|x| anyhow!("{:?} can't be converted to utf-8!", x))?;

                res.files.insert(name, File::from_path(&item.path()));
            }
            // if it's a directory with an index.lua file, load a script
            else if item.file_type()?.is_dir() && item.path().join("index.lua").exists() {
                let script = Script::load(&item.path(), lua, static_files.clone(), styles.clone())?;
                let name = item
                    .file_name()
                    .into_string()
                    .map_err(|x| anyhow!("{:?} can't be converted to utf-8!", x))?;

                // insert it
                res.scripts.insert(name, script);
            }
            // normal directory, load it
            else {
                let dir = Directory::load(&item.path(), lua, static_files.clone(), styles.clone())?;
                let name = item
                    .file_name()
                    .into_string()
                    .map_err(|x| anyhow!("{:?} can't be converted to utf-8!", x))?;

                // insert it
                res.directories.insert(name, dir);
            }
        }

        Ok(res)
    }
}
