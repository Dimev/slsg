use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{ErrorContext, ExternalResult, Lua, Result, Table};

pub(crate) fn stdlib(lua: &Lua) -> Result<Table<'_>> {
    let api = lua.create_table()?;

    // TODO: just use utf8 strings?

    // list files
    api.set(
        "dir",
        lua.create_function(|lua, path: mlua::String| {
            let path = PathBuf::from(OsString::from_vec(path.as_bytes().into()));

            // TODO: prevent escaping the folder?

            let entries = lua.create_table()?;
            for entry in std::fs::read_dir(&path)
                .into_lua_err()
                .context(format!("Failed to read directory {:?}", path))?
            {
                let entry = entry
                    .into_lua_err()
                    .context(format!("Failed to read directory entry in {:?}", path))?;

                println!("{:?}", entry);

                entries.push(lua.create_string(entry.file_name().into_encoded_bytes())?)?
            }

            Ok(entries)
        })?,
    )?;

    // read file
    api.set(
        "read",
        lua.create_function(|_, path: Vec<u8>| {
            let path = PathBuf::from(OsString::from_vec(path));

            // TODO: prevent escaping the folder?

            std::fs::read(&path)
                .into_lua_err()
                .context(format!("Failed to read file {:?}", path))
        })?,
    )?;

    // file name
    api.set(
        "filename",
        lua.create_function(|_, path: Vec<u8>| {
            Ok(PathBuf::from(OsString::from_vec(path))
                .file_name()
                .map(|x| x.to_os_string().into_encoded_bytes()))
        })?,
    )?;

    // file stem
    api.set(
        "filestem",
        lua.create_function(|_, path: Vec<u8>| {
            Ok(PathBuf::from(OsString::from_vec(path))
                .file_stem()
                .map(|x| x.to_os_string().into_encoded_bytes()))
        })?,
    )?;

    // file extention
    api.set(
        "fileext",
        lua.create_function(|_, path: Vec<u8>| {
            Ok(PathBuf::from(OsString::from_vec(path))
                .extension()
                .map(|x| x.to_os_string().into_encoded_bytes()))
        })?,
    )?;

    // latex to mathml
    api.set(
        "latex_to_mathml",
        lua.create_function(|_, (latex, inline): (String, Option<bool>)| {
            latex_to_mathml(
                &latex,
                if inline.unwrap_or(false) {
                    DisplayStyle::Inline
                } else {
                    DisplayStyle::Block
                },
            )
            .into_lua_err()
            .context("Failed to convert latex to mathml")
        })?,
    )?;

    Ok(api)
}
