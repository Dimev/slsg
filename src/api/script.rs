use std::{collections::HashMap, fs, path::Path};

use clap::error::Result;
use mlua::{AnyUserData, Function, Lua, UserData};

use super::{directory::Directory, file::File, page::Page};

/// Script to run
#[derive(Clone, Debug)]
pub(crate) struct Script<'lua> {
    /// script code
    script: Function<'lua>,

    /// directories that are colocated for this script
    colocated: Directory<'lua>,

    /// static files
    static_files: Directory<'lua>,

    /// style files
    styles: HashMap<String, File>,
}

impl<'lua> Script<'lua> {
    /// Load the script, from the given path
    pub(crate) fn load<P: AsRef<Path>>(
        path: &P,
        lua: &'lua Lua, // TODO: do the static file and style as reference
        static_files: Directory<'lua>,
        styles: HashMap<String, File>,
    ) -> Result<Self, anyhow::Error> {
        // if this is a file, simply load it, rest is empty
        if path.as_ref().is_file() {
            let script = fs::read_to_string(path)?;

            // load script to lua
            let script = lua
                .load(script)
                .set_name(path.as_ref().to_string_lossy().to_owned())
                .into_function()?;

            Ok(Self {
                script,
                colocated: Directory::empty(),
                static_files,
                styles,
            })
        } else {
            // find and read the index.lua
            let script = fs::read_to_string(path.as_ref().join("index.lua"))?;

            // load script to lua
            let script = lua
                .load(script)
                .set_name(path.as_ref().join("index.lua").to_string_lossy().to_owned())
                .into_function()?;

            // read the directory
            let colocated = Directory::load(path, lua, static_files.clone(), styles.clone())?;

            Ok(Self {
                script,
                colocated,
                static_files,
                styles,
            })
        }
    }

    /// run the script, and get the resulting page
    pub(crate) fn run(&self, lua: &'lua Lua) -> Result<Page, anyhow::Error> {
        // run with context
        lua.scope(|scope| {
            // make ourselves
            let template = scope.create_nonstatic_userdata(self)?;

            // insert ourselves into the template slot
            lua.globals().set("template", template)?;

            // run the script, get a page out if possible
            self.script.call::<(), AnyUserData>(())?.take::<Page>()
        })
        .map_err(|x| x.into())
    }
}

impl<'a> UserData for &Script<'a> {}
