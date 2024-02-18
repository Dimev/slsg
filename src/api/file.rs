use std::{
    fs,
    path::{Path, PathBuf},
};

use mlua::{AnyUserData, FromLua, Lua, UserData, Value};

/// File for the file tree
#[derive(Clone, Debug)]
pub(crate) enum File {
    /// relative path to the original
    RelPath(PathBuf),

    /// Content of a new file
    New(String),
}

impl File {
    pub fn from_path<P: AsRef<Path>>(path: &P) -> Self {
        Self::RelPath(path.as_ref().into())
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        match self {
            Self::RelPath(p) => fs::copy(p, path).map(|_| ()),
            Self::New(content) => fs::write(path, content).map(|_| ()),
        }
        .map_err(|x| x.into())
    }
}

impl UserData for File {
    // TODO: the file loading funcs
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
