use std::{
    fs::ReadDir,
    io::{Cursor, Read},
    path::PathBuf,
};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{
    Error, ErrorContext, ExternalResult, Function, Lua, MetaMethod, Result, Table, UserData,
    UserDataMethods,
};
use rsass::{
    input::{Context, Loader, SourceFile, SourceName},
    output::{Format, Style},
};

use crate::{highlight::Highlighter, luamark::Parser};

#[derive(Debug)]
struct LuaLoader(Option<Function>);

impl<'a> Loader for LuaLoader {
    type File = Box<dyn Read>;
    fn find_file(
        &self,
        url: &str,
    ) -> std::result::Result<Option<Self::File>, rsass::input::LoadError> {
        let Some(fun) = &self.0 else {
            return Err(rsass::input::LoadError::Input(
                url.to_string(),
                std::io::Error::other("No function for loading files was passed"),
            ));
        };

        let res: Option<String> = match fun.call(url) {
            Ok(res) => res,
            Err(e) => {
                return Err(rsass::input::LoadError::Input(
                    url.to_string(),
                    std::io::Error::other(e.context("Failed to call `loader`").to_string()),
                ))
            }
        };

        // convert to dyn box
        let opt: Option<Box<dyn Read>> = match res {
            Some(x) => Some(Box::new(Cursor::new(x))),
            None => None,
        };

        Ok(opt)
    }
}

struct DirIter(ReadDir);
struct FileIter(ReadDir);

impl UserData for DirIter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method_mut(MetaMethod::Call, |_, iter, ()| {
            while let Some(e) = iter.0.next() {
                // read the entry
                let res = e.into_lua_err().context("Failed to read directory entry")?;

                // skip if it's not a directory
                if !res
                    .file_type()
                    .into_lua_err()
                    .context("Failed to read directory entry type")?
                    .is_dir()
                {
                    continue;
                }

                // return the name
                return Ok(Some(res.file_name().into_string().map_err(|x| {
                    Error::external(format!("Failed to convert filename {:?} into String", x))
                        .context("Failed to read directory entry")
                })?));
            }

            Ok(None)
        });
    }
}

impl UserData for FileIter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method_mut(MetaMethod::Call, |_, iter, ()| {
            while let Some(e) = iter.0.next() {
                // read the entry
                let res = e.into_lua_err().context("Failed to read directory entry")?;

                // skip if it's not a file
                if !res
                    .file_type()
                    .into_lua_err()
                    .context("Failed to read directory entry type")?
                    .is_file()
                {
                    continue;
                }

                // return the name
                return Ok(Some(res.file_name().into_string().map_err(|x| {
                    Error::external(format!("Failed to convert filename {:?} into String", x))
                        .context("Failed to read directory entry")
                })?));
            }

            Ok(None)
        });
    }
}

pub(crate) fn stdlib(lua: &Lua) -> Result<Table> {
    let api = lua.create_table()?;
    // list directories
    api.set(
        "dirs",
        lua.create_function(|_, path: String| {
            let path = PathBuf::from(path);

            // read the entries
            let entries = std::fs::read_dir(&path)
                .into_lua_err()
                .context(format!("Failed to read directory {:?}", path))?;

            let iter = DirIter(entries);

            Ok(iter)
        })?,
    )?;

    // list files
    api.set(
        "files",
        lua.create_function(|_, path: String| {
            let path = PathBuf::from(path);

            // read the entries
            let entries = std::fs::read_dir(&path)
                .into_lua_err()
                .context(format!("Failed to read directory {:?}", path))?;

            let iter = FileIter(entries);

            Ok(iter)
        })?,
    )?;

    // does directory exist?
    api.set(
        "dir_exists",
        lua.create_function(|_, path: String| {
            let path = PathBuf::from(path);
            Ok(path.is_dir())
        })?,
    )?;

    // does a file exist?
    api.set(
        "file_exists",
        lua.create_function(|_, path: String| {
            let path = PathBuf::from(path);
            Ok(path.is_dir())
        })?,
    )?;

    // read file
    api.set(
        "read",
        lua.create_function(|lua: &Lua, path: String| {
            let path = PathBuf::from(path);
            let bytes = std::fs::read(&path)
                .into_lua_err()
                .context(format!("Failed to read file {:?}", path))?;

            // as raw bytes
            lua.create_string(bytes)
        })?,
    )?;

    // file name
    api.set(
        "file_name",
        lua.create_function(|_, path: String| {
            Ok(PathBuf::from(path)
                .file_name()
                .map(|x| x.to_str().map(String::from)))
        })?,
    )?;

    // file stem
    api.set(
        "file_stem",
        lua.create_function(|_, path: String| {
            Ok(PathBuf::from(path)
                .file_stem()
                .map(|x| x.to_str().map(String::from)))
        })?,
    )?;

    // file extention
    api.set(
        "file_ext",
        lua.create_function(|_, path: String| {
            Ok(PathBuf::from(path)
                .extension()
                .map(|x| x.to_str().map(String::from)))
        })?,
    )?;

    // latex to mathml
    api.set(
        "compile_tex",
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

    // sass parser
    api.set(
        "compile_sass",
        lua.create_function(
            |lua, (sass, loader, expand): (String, Option<Function>, Option<bool>)| {
                // loader so we can load our own files
                let loader = LuaLoader(loader);

                // compile the sass
                let compiled = Context::for_loader(loader)
                    .with_format(Format {
                        // expand if needed
                        style: if expand.unwrap_or(false) {
                            Style::Expanded
                        } else {
                            Style::Compressed
                        },
                        precision: 10,
                    })
                    .transform(SourceFile::scss_bytes(
                        sass.as_bytes(),
                        SourceName::root("-"),
                    ))
                    .into_lua_err()
                    .context("Failed to transform Sass")?;

                // string is from the output bytes
                lua.create_string(compiled)
            },
        )?,
    )?;

    // luamark parser, run the code
    api.set(
        "compile_luamark",
        lua.create_function(|lua, (string, macros): (String, Table)| {
            Parser::parse(lua, &string, macros, 1, 1)
        })?,
    )?;

    // syntax highlighting, create highlighter
    api.set(
        "create_highlighter",
        lua.create_function(|_, text| Highlighter::from_rules(text))?,
    )?;

    Ok(api)
}
