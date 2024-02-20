use std::{
    fs,
    path::{Path, PathBuf},
};

use mlua::{AnyUserData, FromLua, Lua, UserData, UserDataFields, UserDataMethods, Value};

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
    /// Create a file from a given path
    pub(crate) fn from_path<P: AsRef<Path>>(path: &P) -> Self {
        Self::RelPath(path.as_ref().into())
    }

    /// Write the file to the given path
    pub(crate) fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        match self {
            Self::RelPath(p) => fs::copy(p, path).map(|_| ()),
            Self::New(content) => fs::write(path, content).map(|_| ()),
        }
        .map_err(|x| x.into())
    }

    /// Read the file to a string
    pub(crate) fn get_string(&self) -> Result<String, anyhow::Error> {
        match self {
            Self::RelPath(path) => fs::read_to_string(path).map_err(|x| x.into()),
            Self::New(str) => Ok(str.clone()),
        }
    }

    /// Get the path used, if any
    fn get_path(&self) -> Option<&Path> {
        match self {
            Self::RelPath(p) => Some(p),
            Self::New(_) => None,
        }
    }
}

impl UserData for File {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("stem", |_, this| {
            Ok(this
                .get_path()
                .and_then(|x| x.file_stem())
                .and_then(|x| x.to_str())
                .map(|x| x.to_owned()))
        });
        fields.add_field_method_get("name", |_, this| {
            Ok(this
                .get_path()
                .and_then(|x| x.file_name())
                .and_then(|x| x.to_str())
                .map(|x| x.to_owned()))
        });
        fields.add_field_method_get("extention", |_, this| {
            Ok(this
                .get_path()
                .and_then(|x| x.extension())
                .and_then(|x| x.to_str())
                .map(|x| x.to_owned()))
        });
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("parseMd", |_, this, ()| {
            Markdown::from_str(&this.get_string().map_err(|x| mlua::Error::external(x))?)
                .map_err(|x| mlua::Error::external(x))
        });
        methods.add_method("parseTxt", |_, this, ()| {
            this.get_string().map_err(|x| mlua::Error::external(x))
        });
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
