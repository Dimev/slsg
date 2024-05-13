use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use mlua::{FromLua, Lua, UserData, UserDataFields, UserDataMethods, Value};

use image::{imageops::FilterType, io::Reader as ImageReader};

/// File for the file tree
#[derive(Clone, Debug)]
pub(crate) enum File {
    /// relative path to the original
    RelPath(PathBuf),

    /// Content of a new file
    New(String),

    /// Content of a new file, binary
    NewBin(Vec<u8>),

    /// Resized image, percentage of size
    ResizePercentage(PathBuf, f32),

    /// Resized image, x axis size
    ResizeX(PathBuf, u32),

    /// Resized image, y axis size
    ResizeY(PathBuf, u32),
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
            Self::NewBin(content) => fs::write(path, content).map(|_| ()),
            x => fs::write(path, x.get_bytes()?).map(|_| ()),
        }
        .map_err(|x| x.into())
    }

    /// Read the file to a string
    pub(crate) fn get_string(&self) -> Result<String, anyhow::Error> {
        match self {
            Self::RelPath(path) => fs::read_to_string(path).map_err(|x| x.into()),
            Self::New(str) => Ok(str.clone()),
            Self::NewBin(_) => Err(anyhow!("A binary file cannot be loaded to a string!")),
            _ => Err(anyhow!("A resized image cannot be loaded to a string!")),
        }
    }

    /// Read the file to bytes
    pub(crate) fn get_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        match self {
            Self::RelPath(path) => fs::read(path).map_err(|x| x.into()),
            Self::New(str) => Ok(str.as_bytes().to_owned()),
            Self::NewBin(bytes) => Ok(bytes.clone()),
            Self::ResizePercentage(path, perc) => {
                let img = ImageReader::open(path)?;
                let format = img
                    .format()
                    .ok_or(anyhow!("Image did not have a known format!"))?;
                let decoded = img.decode()?;
                let resized = decoded.resize(
                    (decoded.width() as f32 * perc * 0.01) as u32,
                    (decoded.height() as f32 * perc * 0.01) as u32,
                    FilterType::Lanczos3,
                );
                let mut bytes = Vec::<u8>::new();
                resized.write_to(&mut Cursor::new(&mut bytes), format)?;
                Ok(bytes)
            }
            Self::ResizeX(path, width) => {
                let img = ImageReader::open(path)?;
                let format = img
                    .format()
                    .ok_or(anyhow!("Image did not have a known format!"))?;
                let decoded = img.decode()?;
                let resized = decoded.resize(*width, std::u32::MAX, FilterType::Lanczos3);
                let mut bytes = Vec::<u8>::new();
                resized.write_to(&mut Cursor::new(&mut bytes), format)?;
                Ok(bytes)
            }
            Self::ResizeY(path, height) => {
                let img = ImageReader::open(path)?;
                let format = img
                    .format()
                    .ok_or(anyhow!("Image did not have a known format!"))?;
                let decoded = img.decode()?;
                let resized = decoded.resize(std::u32::MAX, *height, FilterType::Lanczos3);
                let mut bytes = Vec::<u8>::new();
                resized.write_to(&mut Cursor::new(&mut bytes), format)?;
                Ok(bytes)
            }
        }
    }

    /// Get the path used, if any
    fn get_path(&self) -> Option<&Path> {
        match self {
            Self::New(_) => None,
            Self::NewBin(_) => None,
            Self::RelPath(p) => Some(p),
            Self::ResizePercentage(p, _) => Some(p),
            Self::ResizeX(p, _) => Some(p),
            Self::ResizeY(p, _) => Some(p),
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
        methods.add_method("readString", |_, this, ()| {
            this.get_string().map_err(mlua::Error::external)
        });
        // TODO: readBase64/binary(?)
        methods.add_method("resizeImgPercentage", |_, this, size: f32| match this {
            Self::New(_) | Self::NewBin(_) => Err(mlua::Error::external(anyhow!(
                "A new file cannot be resized"
            ))),
            Self::RelPath(p) => Ok(Self::ResizePercentage(p.clone(), size)),
            Self::ResizeX(p, amount) => Ok(Self::ResizeX(
                p.clone(),
                (*amount as f32 * size * 0.01) as u32,
            )),
            Self::ResizeY(p, amount) => Ok(Self::ResizeY(
                p.clone(),
                (*amount as f32 * size * 0.01) as u32,
            )),
            Self::ResizePercentage(p, perc) => {
                Ok(Self::ResizePercentage(p.clone(), perc * size * 0.01))
            }
        });
        methods.add_method("resizeImgX", |_, this, size: u32| match this {
            Self::New(_) | Self::NewBin(_) => Err(mlua::Error::external(anyhow!(
                "A new file cannot be resized"
            ))),
            Self::RelPath(p) => Ok(Self::ResizeX(p.clone(), size)),
            Self::ResizeX(p, _) => Ok(Self::ResizeX(p.clone(), size)),
            Self::ResizeY(p, _) => Ok(Self::ResizeX(p.clone(), size)),
            Self::ResizePercentage(p, _) => Ok(Self::ResizeX(p.clone(), size)),
        });
        methods.add_method("resizeImgY", |_, this, size: u32| match this {
            Self::New(_) | Self::NewBin(_) => Err(mlua::Error::external(anyhow!(
                "A new file cannot be resized"
            ))),
            Self::RelPath(p) => Ok(Self::ResizeY(p.clone(), size)),
            Self::ResizeX(p, _) => Ok(Self::ResizeY(p.clone(), size)),
            Self::ResizeY(p, _) => Ok(Self::ResizeY(p.clone(), size)),
            Self::ResizePercentage(p, _) => Ok(Self::ResizeY(p.clone(), size)),
        });
    }
}

impl<'lua> FromLua<'lua> for File {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        // it's userdata
        if let Some(userdata) = value.as_userdata() {
            // get the file out of the userdata, and clone it
            let file: File = userdata.borrow::<File>()?.clone();

            // success
            Ok(file)
        } else if let Some(string) = value.as_str() {
            Ok(File::New(string.to_owned()))
        } else {
            Err(mlua::Error::external(anyhow!(
                "Expected either a string, or a file created with the `site.file` function"
            )))
        }
    }
}
