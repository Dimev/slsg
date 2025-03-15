use std::{
    fs::ReadDir,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Mutex,
};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use lazy_static::lazy_static;
use mlua::{
    Error, ErrorContext, ExternalResult, Function, IntoLua, Lua, MetaMethod, Result, Table,
    UserData,
};
use rsass::{
    compile_scss_path,
    output::{Format, Style},
};

pub(crate) fn stdlib(lua: &Lua, output: &Table) -> Result<()> {
    // compile luamark
    lua.globals().set(
        "compile_tex",
        lua.create_function(|_, (tex, inline): (String, Option<bool>)| {
            latex_to_mathml(
                &tex,
                if inline == Some(true) {
                    DisplayStyle::Inline
                } else {
                    DisplayStyle::Block
                },
            )
            .into_lua_err()
            .context("Failed to compile mathml")
        })?,
    )?;

    // compile sass
    lua.globals().set(
        "compile_sass",
        lua.create_function(|_, path: String| {
            compile_scss_path(
                &PathBuf::from(path),
                Format {
                    style: Style::Compressed,
                    ..Format::default()
                },
            )
            .into_lua_err()
            .context("Failed to compile sass")
        })?,
    )?;

    // highlight some code, to html
    lua.globals().set(
        "highlight",
        lua.create_function(|_, (lang, code): (String, String)| {
            Err::<(), _>("TODO!")
                .into_lua_err()
                .context("Failed to highlight code")
        })?,
    )?;

    lua.globals().set(
        "add_highlights",
        lua.create_function(|_, path: String| {
            Err::<(), _>("TODO!")
                .into_lua_err()
                .context("Failed to add highligher")
        })?,
    )?;

    // load lua version of the stdlib
    lua.load(include_str!("stdlib.lua"))
        .set_name("=stdlib.lua")
        .call::<()>(output)
        .context("Failed to load stdlib")?;

    // TODO: set 404

    Ok(())
}
