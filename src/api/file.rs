use std::{
    fs,
    path::{Path, PathBuf},
};

use mlua::{AnyUserData, FromLua, Lua, UserData, UserDataMethods, Value};

use super::markdown::Markdown;

/// File for the file tree
#[derive(Clone, Debug)]
pub(crate) enum File {
    /// relative path to the original
    RelPath(PathBuf),

    /// Content of a new file
    New(String),
}

impl File {
    pub(crate) fn from_path<P: AsRef<Path>>(path: &P) -> Self {
        Self::RelPath(path.as_ref().into())
    }

    pub(crate) fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        match self {
            Self::RelPath(p) => fs::copy(p, path).map(|_| ()),
            Self::New(content) => fs::write(path, content).map(|_| ()),
        }
        .map_err(|x| x.into())
    }

    fn get_string(&self) -> Result<String, mlua::Error> {
        match self {
            Self::RelPath(path) => fs::read_to_string(path).map_err(|x| mlua::Error::external(x)),
            Self::New(str) => Ok(str.clone()),
        }
    }
}

impl UserData for File {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("parseMd", |_, this, ()| {
            Markdown::from_str(&this.get_string()?).map_err(|x| mlua::Error::external(x))
        });
        methods.add_method("parseTxt", |_, this, ()| this.get_string());
    }
}

impl<'lua> FromLua<'lua> for File {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        // it's userdata
        let userdata = AnyUserData::from_lua(value, lua)?;

        // get the file out of the userdata, and clone it
        let file: File = userdata.borrow::<File>()?.clone();

        // success
        Ok(file)
    }
}
