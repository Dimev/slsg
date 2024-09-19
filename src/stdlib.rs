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

use crate::luamark::Parser;

// TODO: improve?
pub(crate) fn contain_path(path: String) -> Result<PathBuf> {
    // backslashes means it's invalid
    if path.contains('\\') {
        return Err(Error::external("Path contains a \\, which is not allowed"));
    }

    // trim any initial /
    let path = path.trim_start_matches('/');

    // parts of the path
    let mut resolved = PathBuf::from("");
    let mut length = 0;
    // go over all parts of the original path
    for component in path.split('/') {
        if component == ".." {
            // break if this path is not valid due to not being able to drop a component
            if !resolved.pop() || length <= 0 {
                return Err(Error::external("Path tries to escape directory using `..`"));
            }
            length -= 1;
        }
        // only advance if this is not the current directory
        else if component != "." {
            length += 1;
            resolved.push(component);
        } else {
            resolved.push(component);
        }
    }

    Ok(resolved)
}

#[derive(Debug)]
struct LuaLoader<'a>(Option<Function<'a>>);

impl<'a> Loader for LuaLoader<'a> {
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
                    std::io::Error::other(e),
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

struct DirIter(ReadDir, u8);

impl UserData for DirIter {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method_mut(MetaMethod::Call, |_, iter, ()| {
            // make sure it matches lfs.dir
            if iter.1 == 0 {
                iter.1 += 1;
                return Ok(Some(String::from(".")));
            }
            if iter.1 == 1 {
                iter.1 += 1;
                return Ok(Some(String::from("..")));
            }

            // do the rest of the directory
            match iter.0.next() {
                Some(e) => {
                    let res = e.into_lua_err().context("Failed to read directory entry")?;
                    let file = res.file_name().into_string().map_err(|x| {
                        Error::external(format!("Failed to convert filename {:?} into String", x))
                            .context("Failed to read directory entry")
                    })?;
                    Ok(Some(file))
                }
                None => Ok(None),
            }
        });
    }
}

pub(crate) fn stdlib(lua: &Lua) -> Result<Table<'_>> {
    let api = lua.create_table()?;
    // list files
    api.set(
        "dir",
        lua.create_function(|_, path: String| {
            let path = contain_path(path)?;

            // read the entries
            let entries = std::fs::read_dir(&path)
                .into_lua_err()
                .context(format!("Failed to read directory {:?}", path))?;

            // make it an iterator
            // this is to matsh the lfs API, tho we don't include the . and .. entries
            let iter = DirIter(entries, 0);

            Ok(iter)
        })?,
    )?;

    // read file
    api.set(
        "read",
        lua.create_function(|lua: &Lua, path: String| {
            let path = contain_path(path)?;
            let bytes = std::fs::read(&path)
                .into_lua_err()
                .context(format!("Failed to read file {:?}", path))?;

            // as raw bytes
            lua.create_string(bytes)
        })?,
    )?;

    // file name
    api.set(
        "filename",
        lua.create_function(|_, path: String| {
            Ok(contain_path(path)?
                .file_name()
                .map(|x| x.to_str().map(String::from)))
        })?,
    )?;

    // file stem
    api.set(
        "filestem",
        lua.create_function(|_, path: String| {
            Ok(contain_path(path)?
                .file_stem()
                .map(|x| x.to_str().map(String::from)))
        })?,
    )?;

    // file extention
    api.set(
        "fileext",
        lua.create_function(|_, path: String| {
            Ok(contain_path(path)?
                .extension()
                .map(|x| x.to_str().map(String::from)))
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

    // sass parser
    api.set(
        "sass",
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

    // luamark parser
    api.set(
        "luamark",
        lua.create_function(|lua, (string, commands): (String, Table)| {
            Parser::parse(lua, commands, &string)
        })?,
    )?;

    Ok(api)
}
