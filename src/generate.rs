use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    rc::Rc,
};

use codemap::SpanLoc;
use glob::Pattern;
use grass::{Logger, Options};
use latex2mathml::{DisplayStyle, latex_to_mathml};
use mlua::{ErrorContext, ExternalResult, Lua, ObjectLike, Result, Value, chunk};
use relative_path::RelativePathBuf;

use crate::{
    conf::Config,
    font::{chars_from_html, subset_font},
    highlight::Highlighter,
    markdown::markdown,
    path::{DoubleFileExt, HtmlToIndex},
    print::print_warning,
    templates::template,
};

#[derive(Debug)]
struct SassLogger();

impl Logger for SassLogger {
    fn debug(&self, location: SpanLoc, message: &str) {
        print_warning(
            &format!(
                "While parsing `{}:{}` [DEBUG]",
                location.file.name(),
                location.begin.line + 1
            ),
            &message,
        );
    }

    fn warn(&self, location: SpanLoc, message: &str) {
        print_warning(
            &format!(
                "While parsing `{}:{}:{}`",
                location.file.name(),
                location.begin.line + 1,
                location.begin.column + 1
            ),
            &message,
        );
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

thread_local! {
    /// Previous charset
    static CHARSET: RefCell<BTreeSet<char>> = RefCell::new(BTreeSet::new());

    /// Cached fonts
    static SUBSETTED: RefCell<BTreeMap<Vec<u8>, Vec<u8>>> = RefCell::new(BTreeMap::new());

    /// Syntax cache, only stores the built-in ones
    static SYNTAXES: RefCell<Vec<Highlighter>> = RefCell::new(Vec::new());
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

    // register a syntax
    let syntaxes = Rc::new(RefCell::new(Vec::new()));
    let syntaxes_clone = syntaxes.clone();
    globals.set(
        "registersyntax",
        lua.create_function(move |lua, table| {
            let highlighter = Highlighter::from_table(lua, table)?;
            syntaxes_clone.borrow_mut().push(highlighter);
            Ok(())
        })?,
    )?;

    // highlight code
    let syntaxes_clone = syntaxes.clone();
    globals.set(
        "highlight",
        lua.create_function(
            move |_, (language, code, prefix): (String, String, Option<String>)| {
                // find the highlighter to use
                let syntaxes = syntaxes_clone.borrow();

                // no try_find yet
                // search from the back, as we add syntaxes to the back of the list
                for h in syntaxes.iter().rev() {
                    if h.match_filename(&language)? {
                        // highlight!
                        return h.highlight(&code, &prefix.unwrap_or(String::new()));
                    }
                }

                // return error if no language was found
                Err(mlua::Error::external(format!(
                    "Could not find a highlighter for language `{language}`"
                )))
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

    // emit a file
    let emit_extra = Rc::new(RefCell::new(BTreeMap::new()));
    let emit_extra_clone = emit_extra.clone();
    globals.set(
        "emitfile",
        lua.create_function(move |_, (path, content): (String, mlua::String)| {
            emit_extra_clone
                .borrow_mut()
                .insert(RelativePathBuf::from(path), content.as_bytes().to_owned());
            Ok(())
        })?,
    )?;

    // ignore a file
    let ignore_extra = Rc::new(RefCell::new(BTreeSet::new()));
    let ignore_extra_clone = ignore_extra.clone();
    globals.set(
        "ignorefile",
        lua.create_function(move |_, path: String| {
            ignore_extra_clone
                .borrow_mut()
                .insert(RelativePathBuf::from(path));
            Ok(())
        })?,
    )?;

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

    // add chars to subset
    let subset_chars = Rc::new(RefCell::new(BTreeSet::new()));
    let subset_cloned = subset_chars.clone();
    globals.set(
        "extendsubset",
        lua.create_function(move |_, chars: String| {
            subset_cloned.borrow_mut().extend(chars.chars());
            Ok(())
        })?,
    )?;

    // load syntaxes
    // cache them to reuse the regexes and avoid having to reload the lua file
    // this should only be run the first time the program starts, and is done
    // before loading any of the site functions, so no custom syntaxes can
    // be cached here
    if SYNTAXES.with_borrow(|x| x.is_empty()) {
        // load
        lua.load(include_str!("syntaxes.lua"))
            .set_name("@syntaxes.lua")
            .exec()?;

        // add the loaded set to the cache
        SYNTAXES.set(syntaxes.clone().borrow().clone());
    } else {
        // else, load from cache
        *syntaxes.borrow_mut() = SYNTAXES.with_borrow(|x| x.clone());
    };

    // load fennel
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
                    "File `{path}` is not a lua or fennel file, and can't be run as setup script",
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

    // sass filet to compile
    let mut to_sass = Vec::new();

    for path in process {
        // is it a minimark file?
        if path.extension().map(|x| x == "md").unwrap_or(false) {
            // parse
            let (res, functions) = markdown(
                &lua,
                &fs::read_to_string(path.to_path("."))
                    .into_lua_err()
                    .with_context(|_| format!("Failed to read `{path}`"))?,
                &path,
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
                &path,
            )
            .with_context(|_| format!("Failed to template file `{path}`"))?;

            // make the final path
            let path = path
                .without_double_ext()
                .ok_or(mlua::Error::external(format!(
                    "Expected path `{path}` to have a second `.lua` or `.fnl` extension",
                )))?;
            let path = path.html_to_index().unwrap_or(path);

            // template it
            to_template.push_back((path, res, functions));
        }
        // .subset second ext? subset
        else if path.has_double_ext("subset") {
            if path.extension() != Some("ttf") && path.extension() != Some("otf") {
                return Err(mlua::Error::external(format!(
                    "Could not subset font `{path}`, as it is not an otf or ttf font",
                )));
            } else {
                to_subset.push(path);
            }
        }
        // .sass or .scss? compile
        else if path
            .extension()
            .map(|x| ["sass", "scss"].contains(&x))
            .unwrap_or(false)
        {
            to_sass.push(path);
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
                res = fun
                    .call(res.clone())
                    .with_context(|_| format!("Failed to template `{path}`"))?;
            } else if let Some(fun) = fun.as_table() {
                res = fun
                    .call(res.clone())
                    .with_context(|_| format!("Failed to template `{path}`"))?;
            } else if let Some(fun) = fun.as_userdata() {
                res = fun
                    .call(res.clone())
                    .with_context(|_| format!("Failed to template `{path}`"))?;
            }

            // need to process again
            to_template.push_back((path, res, functions));
        } else {
            files.insert(path, res.into_bytes());
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

    // add emitted files
    files.extend(emit_extra.replace(Default::default()).into_iter());

    // we got all files to ignore, filter
    files.retain(|k, _| !ignore_extra.borrow().contains(k));

    // do sass
    for path in to_sass
        .into_iter()
        .filter(|x| !ignore_extra.borrow().contains(x))
    {
        let logger = SassLogger();
        let opts = Options::default()
            .style(if dev {
                grass::OutputStyle::Expanded
            } else {
                grass::OutputStyle::Compressed
            })
            .logger(&logger);

        let res = grass::from_path(path.to_path("."), &opts)
            .into_lua_err()
            .with_context(|_| format!("Failed to compile sass file `{path}`"))?;

        // export a css file
        files.insert(path.with_extension("css"), res.into_bytes());
    }

    // do font subsetting
    // find what characters we have
    let mut charset = BTreeSet::new();

    // load from extra
    charset.extend(config.extra.chars());
    charset.extend(subset_chars.borrow().iter());

    // load from files
    for (path, file) in files
        .iter()
        .filter(|x| !ignore_extra.borrow().contains(x.0))
    {
        // only work on html files
        if path.extension() == Some("htm") || path.extension() == Some("html") {
            // interpret as utf8
            let string = str::from_utf8(&file)
                .into_lua_err()
                .with_context(|_| format!("Failed to get utf8 characters from file `{path}`",))?;

            // parse the html into chars
            let chars = chars_from_html(&string)?;

            // and extend
            charset.extend(chars);
        }
    }

    // check for charset difference
    let charset_changed = CHARSET.with_borrow(|x| x != &charset);

    // clear cache because the charset is new
    if charset_changed {
        SUBSETTED.set(BTreeMap::new());
        CHARSET.set(charset.clone());
    }

    // subset fonts
    for path in to_subset
        .iter()
        .filter(|x| !ignore_extra.borrow().contains(*x))
    {
        let font = fs::read(path.to_path("."))
            .into_lua_err()
            .with_context(|_| format!("Failed to read file `{path}`"))?;
        let subsetted = if config.subset {
            if let Some(subsetted) = SUBSETTED.with_borrow(|x| x.get(&font).cloned()) {
                // get from cache
                subsetted
            } else {
                // else, subset and add
                let subsetted = subset_font(&font, &charset)
                    .with_context(|_| format!("Failed to subset font `{path}`"))?;

                // add
                SUBSETTED.with_borrow_mut(|x| x.insert(font.clone(), subsetted.clone()));

                // return the font we have
                subsetted
            }
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

    // set the not found file
    let not_found = if let Some(path) = config.not_found {
        Some(
            files
                .get(&RelativePathBuf::from(&path))
                .ok_or_else(|| mlua::Error::external(format!("404 page `{path}` not found")))?
                .clone(),
        )
    } else {
        None
    };

    // done
    Ok(Site { files, not_found })
}
