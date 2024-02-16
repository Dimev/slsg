use std::{collections::HashMap, fs, path::Path};

use mlua::UserData;

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
    /// make a new one
    pub(crate) fn new() -> Self {
        Self {
            files: HashMap::new(),
            pages: HashMap::new(),
            html: None,
        }
    }

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
}

impl UserData for Page {}
