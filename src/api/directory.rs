use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use mlua::{Lua, Table};

use super::{file::File, script::Script};

/// Directory for the file tree
#[derive(Clone, Debug)]
pub(crate) struct Directory<'lua> {
    /// the resulting table
    pub(crate) table: Table<'lua>,
}

impl<'lua> Directory<'lua> {
    /// Load a static directory, assuming no scripts
    pub(crate) fn load_static<P: AsRef<Path>>(
        path: P,
        lua: &'lua Lua,
    ) -> Result<Self, anyhow::Error> {
        // tables
        // we want scripts, directories and files
        let scripts = lua.create_table()?;
        let directories = lua.create_table()?;

        // empty!
        let files = lua.create_table()?;

        // only fill if the directory exists
        if path.as_ref().exists() {
            // load all files if needed
            for item in fs::read_dir(&path)? {
                // read
                let item = item?;

                // normal file
                if item.file_type()?.is_file() {
                    let name = item.file_name().into_string().map_err(|_| {
                        anyhow!("{:?} does not have a utf-8 file name", path.as_ref())
                    })?;

                    files.set(name, File::from_path(&item.path()))?;
                }
                // normal directory
                else {
                    let dir = Directory::load_static(&item.path(), lua)?;
                    let name = item
                        .file_name()
                        .into_string()
                        .map_err(|x| anyhow!("{:?} can't be converted to utf-8!", x))?;

                    // insert it
                    directories.set(name, dir.table)?;
                }
            }
        }

        // make last table
        let res = lua.create_table_from([
            ("files", files),
            ("directories", directories),
            ("scripts", scripts),
        ])?;

        Ok(Self { table: res })
    }

    pub(crate) fn load<P: AsRef<Path>>(
        path: P,
        lua: &'lua Lua,
        static_files: &Directory<'lua>,
        styles: &Table<'lua>,
    ) -> Result<Self, anyhow::Error> {
        // tables
        // we want scripts, directories and files
        let scripts = lua.create_table()?;
        let directories = lua.create_table()?;
        let files = lua.create_table()?;

        // load all files if needed
        for item in fs::read_dir(&path)? {
            // read
            let item = item?;

            // skip if index.lua, these are already loaded as script
            if item
                .path()
                .file_name()
                .map(|x| x == "index.lua")
                .unwrap_or(false)
            {
                continue;
            }

            // if it's a lua file, that's not an index.lua file, load is as such
            if item.file_type()?.is_file()
                && item.path().extension().map(|x| x == "lua").unwrap_or(false)
            {
                let script = Script::load(&item.path(), lua, static_files, styles)?;

                // insert it
                scripts.set(script.name, script.script)?;
            }
            // if it's a normal file, add it
            else if item.file_type()?.is_file() {
                let name = item
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow!("{:?} does not have a utf-8 file name", path.as_ref()))?;

                files.set(name, File::from_path(&item.path()))?;
            }
            // if it's a directory with an index.lua file, load a script
            else if item.file_type()?.is_dir() && item.path().join("index.lua").exists() {
                let script = Script::load(&item.path(), lua, static_files, styles)?;

                // insert it
                scripts.set(script.name, script.script)?;
            }
            // normal directory, load it
            else {
                let dir = Directory::load(&item.path(), lua, static_files, styles)?;
                let name = item.file_name().into_string().map_err(|_| {
                    anyhow!("{:?} does not have a utf-8 directory name", path.as_ref())
                })?;

                // insert it
                directories.set(name, dir.table)?;
            }
        }

        // make last table
        let res = lua.create_table_from([
            ("files", files),
            ("directories", directories),
            ("scripts", scripts),
        ])?;

        Ok(Self { table: res })
    }
}
