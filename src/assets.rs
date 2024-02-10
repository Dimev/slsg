use std::path::PathBuf;

use mlua::{Lua, Table};

pub(crate) fn load_asset_functions(lua: &Lua) {
    let load_md = lua.create_function(|lua, asset: Table| {
        let path: String = asset.get("path")?;
        let path = PathBuf::from(path);

        // load the file

        // make the table

        Ok(())
    });
}
