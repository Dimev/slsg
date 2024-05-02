use std::path::{Path, PathBuf};

use mlua::{AnyUserData, FromLua, Lua, Value};

#[derive(Clone)]
pub enum File {
    Path(PathBuf),
    New(Vec<u8>),
}

impl File {
    fn write_to(path: &Path) -> anyhow::Result<()> {
        todo!()
    }
    fn get_bytes() -> anyhow::Result<()> {
        todo!()
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
