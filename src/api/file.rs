use std::{
    fs,
    path::{Path, PathBuf},
};

use mlua::{
    AnyUserData, FromLua, Lua, LuaSerdeExt, Table, UserData, UserDataFields, UserDataMethods, Value,
};
use nom_bibtex::Bibtex;

use super::markdown::Markdown;
use anyhow::anyhow;

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

    /// Read the file to bytes
    pub(crate) fn get_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        match self {
            Self::RelPath(path) => fs::read(path).map_err(|x| x.into()),
            Self::New(str) => Ok(str.as_bytes().to_owned()),
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
            Ok(Markdown::from_str(
                &this.get_string().map_err(|x| mlua::Error::external(x))?,
            ))
        });
        methods.add_method("parseTxt", |_, this, ()| {
            this.get_string().map_err(|x| mlua::Error::external(x))
        });
        methods.add_method("parseJson", |lua, this, ()| {
            let str = this.get_string().map_err(|x| mlua::Error::external(x))?;
            let json: serde_json::Value =
                serde_json::from_str(&str).map_err(|x| mlua::Error::external(x))?;
            lua.to_value(&json)
        });
        methods.add_method("parseYaml", |lua, this, ()| {
            let str = this.get_string().map_err(|x| mlua::Error::external(x))?;
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&str).map_err(|x| mlua::Error::external(x))?;
            lua.to_value(&yaml)
        });
        methods.add_method("parseToml", |lua, this, ()| {
            let str = this.get_string().map_err(|x| mlua::Error::external(x))?;
            let toml: toml::Value = toml::from_str(&str).map_err(|x| mlua::Error::external(x))?;
            lua.to_value(&toml)
        });
        methods.add_method("parseBibtex", |lua, this, ()| {
            let str = this.get_string().map_err(|x| mlua::Error::external(x))?;
            let bibtex: Bibtex = Bibtex::parse(&str)
                .map_err(|x| mlua::Error::external(anyhow!("failed to parse bibtex: {:?}", x)))?;
            biblatex_to_table(lua, bibtex)
        });
    }
}

fn biblatex_to_table(lua: &Lua, bib: Bibtex) -> Result<Table, mlua::Error> {
    let table = lua.create_table()?;
    table.set("comments", bib.comments())?;
    table.set("variables", bib.variables().clone())?;

    // add all entries
    let bibliographies = lua.create_table()?;
    for biblio in bib.bibliographies() {
        let entry = lua.create_table()?;
        entry.set("type", biblio.entry_type())?;
        entry.set("tags", biblio.tags().clone())?;
        bibliographies.set(biblio.citation_key(), entry)?;
    }

    table.set("bibliographies", bibliographies)?;

    Ok(table)
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
