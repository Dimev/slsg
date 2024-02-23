use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use mlua::{FromLua, Lua, Table, Value};

use super::file::File;

/// Generated page
#[derive(Debug)]
pub(crate) struct Page {
    /// files to add
    files: HashMap<String, File>,

    /// Pages to add
    pages: HashMap<String, Page>,

    /// Html content
    html: Option<String>,
}

impl Page {
    /// Render the page to a directory
    pub(crate) fn write_to_directory<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        // remove the previous contents
        if path.as_ref().exists() {
            fs::remove_dir_all(&path)?;
        }

        // create the directory if needed
        fs::create_dir_all(&path)?;

        // write the html, if any
        if let Some(html) = &self.html {
            // minify the html
            // TODO
            fs::write(path.as_ref().join("index.html"), html)?;
        }

        // write all files to it
        for (name, file) in self.files.iter() {
            file.write(path.as_ref().join(name))?;
        }

        // write all subpages to it
        for (name, page) in self.pages.iter() {
            page.write_to_directory(path.as_ref().join(name))?;
        }

        // success
        Ok(())
    }

    /// Render the page to a hashmap
    pub(crate) fn to_hashmap<P: AsRef<Path>>(self, root: P) -> HashMap<PathBuf, File> {
        let mut map = HashMap::new();

        // add the index.html, if any
        if let Some(html) = self.html {
            map.insert(root.as_ref().join("index.html"), File::New(html));
        }

        // add the files
        for (name, file) in self.files.into_iter() {
            map.insert(root.as_ref().join(name), file);
        }

        // add the subpages
        for (name, page) in self.pages.into_iter() {
            let sub = page.to_hashmap(root.as_ref().join(name));

            // write out all the subfiles
            map.extend(sub.into_iter());
        }

        map
    }
}

impl<'lua> FromLua<'lua> for Page {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        // it's a table
        let table = Table::from_lua(value, lua)?;

        // get the tables from the table
        let html: Option<String> = table.get("html")?;
        let files: HashMap<String, File> = table.get("files")?;
        let pages: HashMap<String, Page> = table.get("pages")?;

        // rebuild the page
        Ok(Page { html, files, pages })
    }
}
