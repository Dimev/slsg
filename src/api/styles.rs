use std::{fs, path::Path};

use anyhow::{anyhow, Context};
use mlua::{Lua, Table};
use rsass::{
    compile_scss_path,
    output::{Format, Style},
};

use crate::api::file::File;

pub(crate) fn load_styles<P: AsRef<Path>>(lua: &Lua, path: P) -> Result<Table, anyhow::Error> {
    // ceate the lua table
    let table = lua.create_table()?;

    // go over all files in the directory
    for item in fs::read_dir(path.as_ref())? {
        let item = item?;

        // if it's a file, load it as sass
        if item.file_type()?.is_file() {
            let css = compile_scss_path(
                &item.path(),
                Format {
                    style: Style::Compressed,
                    precision: 10,
                },
            )?;

            // create the file
            let file = File::New(
                String::from_utf8(css)
                    .context(format!("stylesheet {:?} is not utf-8", item.path()))?,
            );

            // add it to the table
            let name = item
                .path()
                .file_stem()
                .ok_or_else(|| anyhow!("{:?} does not have a file stem", item.path()))?
                .to_str()
                .ok_or_else(|| anyhow!("{:?} does not have a utf-8 file stem", item.path()))?
                .to_owned();

            table.set(name, file)?;
        }
    }

    Ok(table)
}
