use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    fs,
    path::PathBuf,
};

use mlua::{ErrorContext, ExternalResult, Lua, ObjectLike, Result, chunk};
use relative_path::{RelativePath, RelativePathBuf};
use syntect::parsing::SyntaxSet;

use crate::{conf::Config, templates::template};

trait DoubleFileExt {
    fn has_double_ext(&self, ext: &str) -> bool;
    fn without_double_ext(&self) -> Option<RelativePathBuf>;
}

impl<T: AsRef<RelativePath>> DoubleFileExt for T {
    fn has_double_ext(&self, ext: &str) -> bool {
        // no file name, stop
        if self
            .as_ref()
            .file_name()
            .map(|x| [".", ".."].contains(&x))
            .unwrap_or(true)
        {
            return false;
        }

        let mut splits = self.as_ref().as_str().rsplit(".");
        splits.next();
        let second_ext = splits.next();
        second_ext.map(|x| x == ext).unwrap_or(false)
    }

    fn without_double_ext(&self) -> Option<RelativePathBuf> {
        // no file name, stop
        if self
            .as_ref()
            .file_name()
            .map(|x| [".", ".."].contains(&x))
            .unwrap_or(true)
        {
            return None;
        }

        let mut splits = self.as_ref().as_str().rsplitn(3, ".");
        let first_ext = splits.next()?;
        let _ = splits.next()?;
        let root = splits.next()?;

        Some(RelativePathBuf::from(format!("{root}.{first_ext}")))
    }
}

trait HtmlToIndex {
    fn html_to_index(&self) -> Option<RelativePathBuf>;
}

impl<T: AsRef<RelativePath>> HtmlToIndex for T {
    fn html_to_index(&self) -> Option<RelativePathBuf> {
        let ext = self
            .as_ref()
            .extension()
            .filter(|x| ["htm", "html"].contains(x))?;
        let file_stem = self.as_ref().file_stem()?;

        if file_stem == "index" {
            // already done, don't do anything
            None
        } else {
            // else, build the new string
            Some(
                self.as_ref()
                    .parent()?
                    .join(file_stem)
                    .join("index.htm")
                    .with_extension(ext),
            )
        }
    }
}

pub(crate) struct Site {
    /// Generated files
    pub files: BTreeMap<RelativePathBuf, Vec<u8>>,

    /// What file to use for 404
    pub not_found: Option<Vec<u8>>,
}

const INDEX_FILES: &[&str] = &[
    "index.htm",
    "index.html",
    "index.lua.htm",
    "index.fnl.htm",
    "index.lua.html",
    "index.fnl.html",
    "index.mmk",
    "index.lua.mmk",
    "index.fnl.mmk",
];

/// Generate the site
/// Assumes that the current directory contains the site.conf file
pub(crate) fn generate(dev: bool) -> Result<Site> {
    // read the config file
    let config = fs::read_to_string("site.conf")
        .into_lua_err()
        .context("failed to read `site.conf`")?;

    let config = Config::parse(&config)?;

    // set up lua
    let lua = unsafe { Lua::unsafe_new() };

    // load standard library
    let globals = lua.globals();
    globals.set("development", dev)?; // true if we are serving

    // if fennel is enabled, add fennel
    if config.fennel {
        let fennel = include_str!("fennel.lua");
        let fennel = lua
            .load(fennel)
            .set_name("=fennel.lua")
            .into_function()
            .context("Failed to load fennel")?;
        lua.load(chunk! {
            package.preload["fennel"] = $fennel
        })
        .exec()
        .context("failed to install fennel")?;
    }

    // TODO: consider setup script?

    // load syntax highlighting
    let mut highlighters = Vec::new();
    highlighters.push(SyntaxSet::load_defaults_newlines());

    // files to process, in that order
    let mut process = Vec::new();

    // depth first traversal of the directories
    let mut stack = vec![RelativePathBuf::new()];
    while let Some(path) = stack.pop() {
        if path.to_path(".").is_dir() {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            let mut index = None;

            // directory? recurse
            for entry in fs::read_dir(path.to_path("."))
                .into_lua_err()
                .context("Could not read directory")?
            {
                let entry = entry
                    .into_lua_err()
                    .context("Failed to read directory entry")?;

                let file_path = path.join(entry.file_name().into_string().map_err(|x| {
                    mlua::Error::external(format!(
                        "Failed to convert path `{}` to a utf8 string",
                        x.to_string_lossy()
                    ))
                })?);

                // take depending on type
                if file_path
                    .file_name()
                    .map(|x| INDEX_FILES.contains(&x))
                    .unwrap_or(false)
                {
                    if index.is_none() {
                        index = Some(file_path)
                    } else {
                        return Err(mlua::Error::external(format!(
                            "Double index file found in directory `{}`",
                            entry.path().to_string_lossy()
                        )));
                    }
                } else if entry
                    .file_type()
                    .into_lua_err()
                    .context("Failed to get file type")?
                    .is_file()
                {
                    files.push(file_path);
                } else {
                    dirs.push(file_path);
                }
            }

            // push all to stack
            // stack, so this is reversed
            // index last, if any, in order to build an index from all other files in the directory
            stack.extend(index.into_iter());

            // files go after directories
            stack.extend(files.into_iter());

            // directories first
            stack.extend(dirs.into_iter());
        } else if path.to_path(".").is_file() {
            process.push(path);
        }
    }

    // final files
    let mut files = BTreeMap::new();

    // files to template with a lua function
    let mut to_template = VecDeque::new();

    // fonts to subset
    let mut to_subset = Vec::new();

    for path in process {
        // .fnl or .lua second ext? template
        if path.has_double_ext("fnl") || path.has_double_ext("lua") {
            // process it now once
            let (res, functions) = template(
                &lua,
                &fs::read_to_string(path.to_path("."))
                    .into_lua_err()
                    .context(format!("Failed to read `{}`", path))?,
                path.as_str(),
                &config,
            )
            .context(format!("Failed to template file `{}`", path))?;

            // make the final path
            let path = path
                .without_double_ext()
                .ok_or(mlua::Error::external(format!(
                    "Expected path `{}` to have a second `.lua` or `.fnl` extension",
                    path
                )))?;
            let path = path.html_to_index().unwrap_or(path);

            // template it
            to_template.push_back((path, res, functions));
        }
        // .subset second ext? subset
        else if path.has_double_ext("subset") {
            let path = path
                .without_double_ext()
                .ok_or(mlua::Error::external(format!(
                    "Expected path `{}` to have a second `.subset` extension",
                    path
                )))?;
            to_subset.push(path);
        }
        // else? emit normally
        else {
            // make the final path
            let path = path.html_to_index().unwrap_or(path);

            // insert it into the files
            files.insert(
                path.clone(),
                fs::read(path.to_path("."))
                    .into_lua_err()
                    .context(format!("Failed to read file `{}`", path))?,
            );
        }
    }

    // filter out all paths to ignore
    // with globbing
    // TODO

    // apply templating
    while let Some((path, mut res, mut functions)) = to_template.pop_front() {
        if let Some(fun) = functions.pop_front() {
            if let Some(fun) = fun.as_function() {
                res = fun.call(res.clone())?;
            } else if let Some(fun) = fun.as_table() {
                res = fun.call(res.clone())?;
            } else if let Some(fun) = fun.as_userdata() {
                res = fun.call(res.clone())?;
            }

            // need to process again
            to_template.push_back((path, res, functions));
        } else {
            files.insert(path.clone(), res.clone().into_bytes());
        }
    }

    // do font subsetting
    // TODO

    // go over all files, process them if needed
    Ok(Site {
        files,
        not_found: None,
    })
}
