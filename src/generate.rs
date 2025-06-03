use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    sync::Arc,
};

use glob::Pattern;
use latex2mathml::{DisplayStyle, latex_to_mathml};
use mlua::{ErrorContext, ExternalResult, Lua, ObjectLike, Result, Value, chunk};
use relative_path::{RelativePath, RelativePathBuf};
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxSet, SyntaxSetBuilder},
    util::LinesWithEndings,
};

use crate::{conf::Config, markdown::markdown, subset::subset_font, templates::template};

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
    "index.md",
    "index.lua.md",
    "index.fnl.md",
];

/// Escape html
fn escape_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    for c in html.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

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

    // load syntax highlighting
    // default one, has a good set of languages already
    let builtin_highlights = Arc::new(SyntaxSet::load_defaults_newlines());
    let mut external_highlights = SyntaxSetBuilder::new();

    // load the ones from the config file
    for path in config.syntaxes.iter() {
        external_highlights
            .add_from_folder(path, true)
            .into_lua_err()
            .with_context(|_| format!("Failed to load syntaxes from folder `{path}`"))?;
    }

    // build it
    let external_highlights = Arc::new(external_highlights.build());

    // load standard library
    let globals = lua.globals();
    globals.set("development", dev)?; // true if we are serving

    // math
    globals.set(
        "mathml",
        lua.create_function(move |_, (mathml, inline): (String, Option<bool>)| {
            latex_to_mathml(
                &mathml,
                if inline.unwrap_or(false) {
                    DisplayStyle::Inline
                } else {
                    DisplayStyle::Block
                },
            )
            .into_lua_err()
            .with_context(|_| format!("Failed to compile math"))
        })?,
    )?;

    // highlight code
    let (hl, ext) = (builtin_highlights.clone(), external_highlights.clone());
    globals.set(
        "highlight",
        lua.create_function(
            move |_, (language, code, prefix): (String, String, Option<String>)| {
                // finding by name doesn't seem to work?
                let (syn, set) = if let Some(syn) = hl.find_syntax_by_token(&language) {
                    (syn, &hl)
                } else if let Some(syn) = ext.find_syntax_by_token(&language) {
                    (syn, &ext)
                } else if language == "" {
                    (hl.find_syntax_plain_text(), &hl)
                } else {
                    return Err(mlua::Error::external(format!(
                        "No syntax found for `{language}`"
                    )));
                };
                let mut generator = ClassedHTMLGenerator::new_with_class_style(
                    syn,
                    set,
                    if let Some(prefix) = prefix {
                        ClassStyle::SpacedPrefixed {
                            // TODO: prevent leaking memory here
                            // probably cache this so it wont leak if there's no new ones?
                            prefix: prefix.leak(),
                        }
                    } else {
                        ClassStyle::Spaced
                    },
                );
                for line in LinesWithEndings::from(&code) {
                    generator
                        .parse_html_for_line_which_includes_newline(line)
                        .into_lua_err()
                        .context("Failed to parse line")?;
                }
                Ok(generator.finalize())
            },
        )?,
    )?;

    // read a file
    globals.set(
        "readfile",
        lua.create_function(|lua, path: String| {
            let path = RelativePathBuf::from(path);
            let data = fs::read(path.to_path("."))
                .into_lua_err()
                .with_context(|_| format!("Could not read file `{path}`"))?;

            // this is a string, but lua strings can represent any type of data
            lua.create_string(data)
        })?,
    )?;

    // escape html
    globals.set(
        "escapehtml",
        lua.create_function(|_, html: String| Ok(escape_html(&html)))?,
    )?;

    // emit a file TODO

    // list files in directory
    globals.set(
        "listfiles",
        lua.create_function(|lua, path: String| {
            let path = RelativePathBuf::from(path);
            let res = lua.create_table()?;
            for entry in path
                .to_path(".")
                .read_dir()
                .into_lua_err()
                .with_context(|_| format!("Failed to list files in `{path}`"))?
            {
                let entry = entry
                    .into_lua_err()
                    .with_context(|_| format!("Failed to list files in `{path}`"))?;

                // if it's a file, add
                if entry
                    .file_type()
                    .into_lua_err()
                    .with_context(|_| format!("Failed to list files in `{path}`"))?
                    .is_file()
                {
                    res.push(entry.file_name().into_string().map_err(|x| {
                        mlua::Error::external(format!(
                            "Could not convert filename `{}` to a utf8 string",
                            x.to_string_lossy()
                        ))
                    })?)?;
                }
            }
            Ok(res)
        })?,
    )?;

    // list directories in directory
    globals.set(
        "listdirs",
        lua.create_function(|lua, path: String| {
            let path = RelativePathBuf::from(path);
            let res = lua.create_table()?;
            for entry in path
                .to_path(".")
                .read_dir()
                .into_lua_err()
                .with_context(|_| format!("Failed to list directories in `{path}`"))?
            {
                let entry = entry
                    .into_lua_err()
                    .with_context(|_| format!("Failed to list directories in `{path}`"))?;

                // if it's a file, add
                if entry
                    .file_type()
                    .into_lua_err()
                    .with_context(|_| format!("Failed to list directories in `{path}`"))?
                    .is_dir()
                {
                    res.push(entry.file_name().into_string().map_err(|x| {
                        mlua::Error::external(format!(
                            "Could not convert filename `{}` to a utf8 string",
                            x.to_string_lossy()
                        ))
                    })?)?;
                }
            }
            Ok(res)
        })?,
    )?;

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

    // output directory, as we want to ignore this one
    let out_dir = RelativePathBuf::from(&config.output_dir);

    // pattterns to ignore
    let to_ignore = config
        .ignore
        .iter()
        .map(|x| {
            Pattern::new(x)
                .into_lua_err()
                .with_context(|_| format!("Failed to make glob pattern `{x}`"))
        })
        // stable try_collect
        .try_fold(Vec::new(), |mut v, p| {
            v.push(p?);
            Ok::<Vec<_>, mlua::Error>(v)
        })?;

    // setup script
    let post_process = if let Some(setup) = &config.setup {
        let path = RelativePathBuf::from(setup);
        let continuation: Value = match path.extension() {
            Some("lua") => lua.load(path.to_path(".")).eval()?,
            Some("fnl") => {
                let code = fs::read_to_string(path.to_path("."))
                    .into_lua_err()
                    .with_context(|_| format!("Could not load fennel file `{path}`"))?;
                let name = path.as_str();
                lua.load(
                    chunk! { require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })},
                )
                .eval()?
            }
            Some(_) | None => {
                return Err(mlua::Error::external(format!(
                    "File `{}` is not a lua or fennel file, and can't be run as setup script",
                    path
                )));
            }
        };
        Some(continuation)
    } else {
        None
    };

    // files to process, in that order
    let mut process = Vec::new();

    // depth first traversal of the directories
    let mut stack = vec![RelativePathBuf::new()];
    while let Some(path) = stack.pop() {
        // skip if this path is in the skippable list
        if path.starts_with(&out_dir)
            || path == "site.conf"
            || config
                .setup
                .as_ref()
                .map(|x| x.as_str() == path)
                .unwrap_or(false)
            || to_ignore.iter().any(|x| x.matches(path.as_str()))
        {
            continue;
        // if it's a directory, read all directories
        } else if path.to_path(".").is_dir() {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            let mut index = None;

            // directory? recurse
            for entry in fs::read_dir(path.to_path("."))
                .into_lua_err()
                .with_context(|_| format!("Could not read directory `{path}`"))?
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
            // normal file, just process
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
        // is it a minimark file?
        if path.extension().map(|x| x == "md").unwrap_or(false) {
            // parse
            let (res, functions) = markdown(
                &lua,
                &fs::read_to_string(path.to_path("."))
                    .into_lua_err()
                    .with_context(|_| format!("Failed to read `{path}`"))?,
                path.as_str(),
                &config,
                path.has_double_ext("lua") || path.has_double_ext("fnl"),
            )
            .with_context(|_| format!("Failed to template file `{path}`"))?;

            // make the final path
            let path =
                path.with_extension("html")
                    .without_double_ext()
                    .ok_or(mlua::Error::external(format!(
                        "Expected path `{path}` to have a second `.lua` or `.fnl` extension"
                    )))?;
            let path = path.html_to_index().unwrap_or(path);

            // template it
            to_template.push_back((path, res, functions));
        }
        // .fnl or .lua second ext? template
        else if path.has_double_ext("fnl") || path.has_double_ext("lua") {
            // process it now once
            let (res, functions) = template(
                &lua,
                &fs::read_to_string(path.to_path("."))
                    .into_lua_err()
                    .with_context(|_| format!("Failed to read `{path}`"))?,
                path.as_str(),
                &config,
            )
            .with_context(|_| format!("Failed to template file `{path}`"))?;

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
            if path.extension() != Some("ttf") || path.extension() != Some("otf") {
                return Err(mlua::Error::external(format!(
                    "Could not subset font `{path}`, as it is not an otf or ttf font",
                )));
            } else {
                to_subset.push(path);
            }
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
                    .with_context(|_| format!("Failed to read file `{path}`"))?,
            );
        }
    }

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

    // complete post-processing
    if let Some(fun) = post_process {
        if let Some(fun) = fun.as_function() {
            fun.call::<()>(())?;
        } else if let Some(fun) = fun.as_table() {
            fun.call::<()>(())?;
        } else if let Some(fun) = fun.as_userdata() {
            fun.call::<()>(())?;
        }
    }

    // do font subsetting
    // find what characters we have
    let mut charset = BTreeSet::new();
    // load from extra
    for c in config.extra.chars() {
        // TODO: find the actual proper characters
        charset.insert(c);
    }

    // load from files
    for (path, file) in files.iter() {
        // only work on html files
        if path.extension() == Some("htm") || path.extension() == Some("html") {
            // interpret as utf8
            let string = str::from_utf8(&file)
                .into_lua_err()
                .with_context(|_| format!("Failed to get utf8 characters from file `{path}`",))?;

            for c in string.chars() {
                charset.insert(c);
            }
        }
    }

    // subset fonts
    for path in to_subset {
        let font = fs::read(path.to_path("."))
            .into_lua_err()
            .with_context(|_| format!("Failed to read file `{path}`"))?;
        let subsetted = if config.subset {
            subset_font(&font, &charset)
                .with_context(|_| format!("Failed to subset font `{path}`"))?
        } else {
            font
        };
        let path = path
            .without_double_ext()
            .ok_or(mlua::Error::external(format!(
                "Expected path `{path}` to have a second `.subset` extension",
            )))?;
        files.insert(path, subsetted);
    }

    // go over all files, process them if needed
    Ok(Site {
        files,
        not_found: None,
    })
}
