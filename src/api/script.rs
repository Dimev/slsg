use std::{cell::RefCell, fs, path::Path, rc::Rc};

use anyhow::anyhow;
use clap::error::Result;
use mlua::{Function, Lua, Table, Value};

use super::{directory::Directory, page::Page};

/// Script to run
#[derive(Clone, Debug)]
pub(crate) struct Script<'lua> {
    /// script code
    pub(crate) script: Function<'lua>,

    /// name of the script
    pub(crate) name: String,
}

impl<'lua> Script<'lua> {
    /// Load the script, from the given path
    pub(crate) fn load(
        base: &impl AsRef<Path>,
        path: &impl AsRef<Path>,
        warnings: Rc<RefCell<Vec<String>>>,
        lua: &'lua Lua,
        static_files: &Directory<'lua>,
        styles: &Table<'lua>,
    ) -> Result<Self, anyhow::Error> {
        // if this is a file, simply load it, rest is empty
        if path.as_ref().is_file() {
            let script = fs::read_to_string(path)?;

            // set load the environment script
            let template = lua.create_table()?;

            // colocated files is empty
            template.set("colocated", lua.create_table()?)?;

            // name of the file
            let name = path
                .as_ref()
                .file_stem()
                .ok_or_else(|| anyhow!("{:?} does not have a file stem", path.as_ref()))?
                .to_str()
                .ok_or_else(|| anyhow!("{:?} does not have a utf-8 file stem", path.as_ref()))?
                .to_owned();

            let rel_path = base
                .as_ref()
                .to_str()
                .ok_or_else(|| anyhow!("{:?} does not have a utf-8 file name", base.as_ref()))?
                .to_owned();

            template.set("name", name.as_str())?;
            template.set("path", rel_path.clone())?;

            // static and styles
            template.set("static", &static_files.table)?;
            template.set("styles", styles)?;

            // TODO: find and findStatic

            // make the environment
            let env = clone_table(lua, lua.globals())?;
            env.set("template", template)?;

            // add warning func into the environment
            let warnings_cloned = warnings.clone();
            let warn = lua.create_function(move |_, text: String| {
                (*warnings_cloned)
                    .borrow_mut()
                    .push(format!("[{}]: {}", rel_path, text));
                Ok(())
            })?;

            env.set("warn", warn.clone())?;

            // standard lib
            lua.load(include_str!("lib.lua"))
                .set_environment(env.clone())
                .set_name("builtin://stdlib.lua")
                .exec()?;

            // load script to lua
            let script = lua
                .load(script)
                .set_environment(env)
                .set_name(base.as_ref().to_string_lossy().to_owned())
                .into_function()?;

            // went ok
            Ok(Self { script, name })
        } else {
            // find and read the index.lua
            let script = fs::read_to_string(path.as_ref().join("index.lua"))?;

            // read the directory
            let colocated =
                Directory::load(base, path, warnings.clone(), lua, static_files, styles)?;

            // set load the environment script
            let template = lua.create_table()?;

            // colocated files is the directory we loaded
            template.set("colocated", colocated.table)?;

            // name of the file
            let name = path
                .as_ref()
                .file_name()
                .ok_or_else(|| anyhow!("{:?} does not have a file name", path.as_ref()))?
                .to_str()
                .ok_or_else(|| anyhow!("{:?} does not have a utf-8 file name", path.as_ref()))?
                .to_owned();

            let rel_path = base
                .as_ref()
                .join("index.lua")
                .to_str()
                .ok_or_else(|| anyhow!("{:?} does not have a utf-8 file name", base.as_ref()))?
                .to_owned();

            template.set("name", name.as_str())?;
            template.set("path", rel_path.clone())?;

            // static and styles
            template.set("static", static_files.table.clone())?;
            template.set("styles", styles.clone())?;

            // TODO: find and findStatic

            // make the environment
            let env = clone_table(lua, lua.globals())?;
            env.set("template", template)?;

            // add warning func into the environment
            let warnings_cloned = warnings.clone();
            let warn = lua.create_function(move |_, text: String| {
                (*warnings_cloned)
                    .borrow_mut()
                    .push(format!("[{}]: {}", rel_path, text));
                Ok(())
            })?;

            env.set("warn", warn.clone())?;

            // standard lib
            lua.load(include_str!("lib.lua"))
                .set_environment(env.clone())
                .set_name("builtin://stdlib.lua")
                .exec()?;

            // load script to lua
            let script = lua
                .load(script)
                .set_environment(env)
                .set_name(base.as_ref().join("index.lua").to_string_lossy().to_owned())
                .into_function()?;

            Ok(Self { script, name })
        }
    }

    /// run the script, and get the resulting page
    pub(crate) fn run(&self) -> Result<Page, anyhow::Error> {
        // run the script, get a page out if possible
        self.script.call::<(), Page>(()).map_err(|x| x.into())
    }
}

fn clone_table<'lua>(lua: &'lua Lua, table: Table<'lua>) -> Result<Table<'lua>, anyhow::Error> {
    let pairs = table.pairs::<Value, Value>().map(|x| {
        let (k, v) = x.unwrap();
        (k, v)
    });

    lua.create_table_from(pairs).map_err(|x| x.into())
}
