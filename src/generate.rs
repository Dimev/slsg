use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use anyhow::anyhow;
use base64::Engine;
use grass::{Fs, OutputStyle};
use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{ErrorContext, FromLua, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, Value};
use nom_bibtex::Bibtex;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::{
    file::File,
    highlight::{highlight, highlight_html, HighlightRule},
    path::{concat_path, file_extension, file_name, file_stem, resolve_path},
};

/// Single set of regex rules, as strings
#[derive(Deserialize)]
struct Rules {
    /// Regex rules
    #[serde(flatten, with = "tuple_vec_map")]
    rules: Vec<(String, String)>,
}

/// Resulting website that is generated
pub struct Site {
    /// Files in the site
    pub files: HashMap<String, File>,

    /// 404 page, if any
    pub not_found: Option<String>,

    /// Emitted warnings
    pub warnings: Vec<String>,
}

impl<'lua> FromLua<'lua> for Site {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        // it's a table
        let table = Table::from_lua(value, lua)
            .context("Result needs to be a table with a table of files, and a NotFound entry")?;

        let files = table.get("files")?;
        let not_found = table.get("notFound")?;

        Ok(Site {
            files,
            not_found,
            warnings: Vec::new(),
        })
    }
}

/// Error when generation fails
pub struct GenerateError {
    /// Emitted warnings
    pub warnings: Vec<String>,

    /// Emitted errors
    pub error: anyhow::Error,
}

trait WithWarn<T, E> {
    fn with_warns<'a, I: Iterator<Item = &'a String>>(
        self,
        warnings: I,
    ) -> Result<T, GenerateError>;
}

impl<T, E: Into<GenerateError>> WithWarn<T, E> for Result<T, E> {
    fn with_warns<'a, I: Iterator<Item = &'a String>>(
        self,
        warnings: I,
    ) -> Result<T, GenerateError> {
        self.map_err(|x| {
            let mut gen = x.into();
            gen.warnings = warnings.map(|x| x.to_owned()).collect();
            gen
        })
    }
}

impl<E: Into<anyhow::Error>> From<E> for GenerateError {
    fn from(value: E) -> Self {
        Self {
            warnings: Vec::new(),
            error: value.into(),
        }
    }
}

/// Grass file system to resolve file paths
#[derive(Debug)]
struct GrassFS {
    /// Working directory for the program
    working_dir: PathBuf,

    /// Relative path to load from
    relative: PathBuf,
}

impl Fs for GrassFS {
    fn is_dir(&self, path: &Path) -> bool {
        let rel = self.relative.join(path);
        let path = if let Some(x) = rel.as_os_str().to_str() {
            x
        } else {
            return false;
        };

        let resolved = if let Some(x) = resolve_path(path) {
            x
        } else {
            return false;
        };

        self.working_dir.join(resolved).is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        let rel = self.relative.join(path);
        let path = if let Some(x) = rel.as_os_str().to_str() {
            x
        } else {
            return false;
        };

        let resolved = if let Some(x) = resolve_path(path) {
            x
        } else {
            return false;
        };

        self.working_dir.join(resolved).is_file()
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let rel = self.relative.join(path);
        let path = if let Some(x) = rel.as_os_str().to_str() {
            x
        } else {
            return Err(std::io::Error::other(anyhow!(
                "Could not represent path as UTF-8 string!"
            )));
        };

        let resolved = if let Some(x) = resolve_path(path) {
            x
        } else {
            return Err(std::io::Error::other(anyhow!(
                "Could not represent path as UTF-8 string!"
            )));
        };

        fs::read(self.working_dir.join(resolved))
    }
}

/// Generate the site from the given lua file
pub fn generate(path: &Path, dev: bool) -> Result<Site, GenerateError> {
    // lua
    let lua = Lua::new_with(
        StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH,
        LuaOptions::new(),
    )?;

    // path to the working directory
    let working_dir = if path.is_file() {
        path.parent()
            .expect("File does not have a parent in it's path")
    } else {
        path
    };
    let script_path = if path.is_file() {
        path.to_owned()
    } else {
        path.join("index.lua")
    };

    // set up our own require function to only load files from this directory
    let path_owned = working_dir.to_owned();
    let require = lua.create_function(move |lua, script: String| {
        // find script
        let path = path_owned
            .join(resolve_path(&script).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?);

        // load script
        let code = fs::read_to_string(path).map_err(mlua::Error::external)?;
        let function = lua.load(code).into_function()?;
        lua.load_from_function::<Value>(&script, function)
    })?;

    lua.globals().set("require", require)?;

    // language highlighters
    let highlighters = Rc::new(RefCell::new(HashMap::<String, Vec<HighlightRule>>::new()));

    // load our library functions
    let lib = lua.create_table()?;

    // list directories in directory
    let path_owned = working_dir.to_owned();
    let list_dirs = lua.create_function(move |lua, path: String| {
        let resolved =
            resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?;

        let path = path_owned.join(&resolved);

        let entries = lua.create_table()?;

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            // append entry if it's a directory
            if entry.file_type()?.is_dir() {
                let name = entry
                    .file_name()
                    .to_os_string()
                    .into_string()
                    .map_err(|x| {
                        mlua::Error::external(anyhow!(
                            "Name {:?} can't be converted to a UTF-8 string",
                            x
                        ))
                    })?;

                entries.set(
                    name,
                    resolved
                        .join(entry.file_name())
                        .into_os_string()
                        .into_string()
                        .map_err(|x| {
                            mlua::Error::external(anyhow!(
                                "Directory {:?} can't be converted to a UTF-8 string",
                                x
                            ))
                        })?,
                )?;
            }
        }

        Ok(entries)
    })?;

    // list files in directory
    let path_owned = working_dir.to_owned();
    let list_files = lua.create_function(move |lua, path: String| {
        let resolved =
            resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?;

        let path = path_owned.join(&resolved);

        let entries = lua.create_table()?;

        for entry in fs::read_dir(&path)? {
            let entry = entry?;

            // append entry if it's a file
            if entry.file_type()?.is_file() {
                let name = entry
                    .file_name()
                    .to_os_string()
                    .into_string()
                    .map_err(|x| {
                        mlua::Error::external(anyhow!(
                            "Name {:?} can't be converted to a UTF-8 string",
                            x
                        ))
                    })?;

                entries.set(
                    name,
                    resolved
                        .join(entry.file_name())
                        .into_os_string()
                        .into_string()
                        .map_err(|x| {
                            mlua::Error::external(anyhow!(
                                "File {:?} can't be converted to a UTF-8 string",
                                x
                            ))
                        })?,
                )?;
            }
        }

        Ok(entries)
    })?;

    // open file file
    let path_owned = working_dir.to_owned();
    let open_file = lua.create_function(move |_, path: String| {
        let path = path_owned
            .join(resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?);
        if path.is_file() {
            Ok(File::from_path(&path))
        } else {
            Err(mlua::Error::external(anyhow!(
                "File {:?} does not exist",
                path
            )))
        }
    })?;

    // read file to string
    let path_owned = working_dir.to_owned();
    let read_file = lua.create_function(move |_, path: String| {
        let path = path_owned
            .join(resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?);
        if path.is_file() {
            Ok(fs::read_to_string(&path)?)
        } else {
            Err(mlua::Error::external(anyhow!(
                "File {:?} does not exist",
                path
            )))
        }
    })?;

    // does a file exist
    let path_owned = working_dir.to_owned();
    let file_exists = lua.create_function(move |_, path: String| {
        let path = path_owned
            .join(resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?);

        Ok(path.is_file())
    })?;

    // does a directory exist
    let path_owned = working_dir.to_owned();
    let dir_exists = lua.create_function(move |_, path: String| {
        let path = path_owned
            .join(resolve_path(&path).ok_or(mlua::Error::external(anyhow!("Invalid path!")))?);

        Ok(path.is_dir())
    })?;

    // concat file paths
    let concat_paths = lua.create_function(|_, (left, right): (String, String)| {
        concat_path(&left, &right).ok_or(mlua::Error::external(anyhow!(
            "Paths could not be concatted"
        )))
    })?;

    // file name
    let file_name =
        lua.create_function(|_, path: String| Ok(file_name(&path).map(|x| x.to_owned())))?;

    // file extention
    let file_extension =
        lua.create_function(|_, path: String| Ok(file_extension(&path).map(|x| x.to_owned())))?;

    // file stem
    let file_stem =
        lua.create_function(|_, path: String| Ok(file_stem(&path).map(|x| x.to_owned())))?;

    // new file
    let new_file = lua.create_function(|_, content| Ok(File::New(content)))?;

    // new binary file
    let new_binary_file = lua.create_function(|_, content| Ok(File::NewBin(content)))?;

    // new base64 file
    let new_base64_file = lua.create_function(|_, text: String| {
        let base = base64::prelude::BASE64_STANDARD
            .decode(text)
            .map_err(mlua::Error::external)?;

        Ok(File::NewBin(base))
    })?;

    let encode_base64 = lua
        .create_function(|_, bytes: Vec<u8>| Ok(base64::prelude::BASE64_STANDARD.encode(bytes)))?;

    let decode_base64 = lua.create_function(|_, text: String| {
        base64::prelude::BASE64_STANDARD
            .decode(text)
            .map_err(mlua::Error::external)
    })?;

    // parse toml
    let parse_toml = lua.create_function(|lua, text: String| {
        let toml: toml::Value = toml::from_str(&text).map_err(mlua::Error::external)?;
        lua.to_value(&toml)
    })?;

    // parse yaml
    let parse_yaml = lua.create_function(|lua, text: String| {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&text).map_err(mlua::Error::external)?;
        lua.to_value(&yaml)
    })?;

    // parse json
    let parse_json = lua.create_function(|lua, text: String| {
        let json: serde_json::Value = serde_json::from_str(&text).map_err(mlua::Error::external)?;
        lua.to_value(&json)
    })?;

    // parse bibtex
    let parse_bibtex = lua.create_function(|lua, text: String| {
        let bib = Bibtex::parse(&text)
            .map_err(|x| mlua::Error::external(anyhow!("Failed to parse bibtex: {:?}", x)))?;
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
    })?;

    // parse sass, from a given working directory
    let path_owned = working_dir.to_owned();
    let parse_sass =
        lua.create_function(move |_, (sass, directory): (String, Option<String>)| {
            // filesystem to use
            let fs = GrassFS {
                working_dir: path_owned.clone(),
                relative: directory
                    .map(|x| PathBuf::from(x))
                    .unwrap_or(PathBuf::new()),
            };

            // parse options
            let options = grass::Options::default()
                .style(OutputStyle::Compressed)
                .fs(&fs);

            // make css
            let css = grass::from_string(sass, &options).map_err(mlua::Error::external)?;

            Ok(css)
        })?;

    // read and eval mdl
    // TODO

    // add highlighters
    let highlighters_cloned = highlighters.clone();
    let add_highlighters = lua.create_function(move |_, text: String| {
        // parse the highlighter
        let raw = toml::from_str::<HashMap<String, Rules>>(&text).map_err(mlua::Error::external)?;
        let mut highlight = highlighters_cloned.borrow_mut();

        // add the language to the highlighters
        for (key, value) in raw.into_iter() {
            highlight.insert(
                key,
                value
                    .rules
                    .into_iter()
                    .map(|(rule, regex)| HighlightRule::Raw(rule, regex))
                    .collect(),
            );
        }

        Ok(())
    })?;

    // highlight code
    let highlighters_cloned = highlighters.clone();
    let highlight_code = lua.create_function(
        move |_, (lang, code, prefix): (String, String, Option<String>)| {
            // get the language
            let mut rules = highlighters_cloned.borrow_mut();
            let mut rules = rules.get_mut(&lang).ok_or(mlua::Error::external(anyhow!(
                "Language {lang} not in highlighters!"
            )))?;

            // highlight
            highlight_html(&mut rules, &code, prefix)
                .map_err(|x| mlua::Error::external(x.context("Failed to highlight code")))
        },
    )?;

    // highlight code to ast
    let highlighters_cloned = highlighters.clone();
    let highlight_ast = lua.create_function(move |lua, (lang, code): (String, String)| {
        // get the language
        let mut rules = highlighters_cloned.borrow_mut();
        let mut rules = rules.get_mut(&lang).ok_or(mlua::Error::external(anyhow!(
            "Language {lang} not in highlighters!"
        )))?;

        // highlight
        let ranges = highlight(&mut rules, &code)
            .map_err(|x| mlua::Error::external(x.context("Failed to highlight code")))?;

        // make it into a table
        let table = lua.create_table()?;
        for range in ranges {
            let t = lua.create_table()?;
            t.set("text", range.text)?;
            t.set("style", range.style)?;
            table.push(t)?;
        }

        Ok(table)
    })?;

    // highlight latex math as mathml
    let mathml = lua.create_function(|_, (text, inline): (String, Option<bool>)| {
        latex_to_mathml(
            &text,
            if inline.unwrap_or(false) {
                DisplayStyle::Inline
            } else {
                DisplayStyle::Block
            },
        )
        .map_err(mlua::Error::external)
    })?;

    // minify/bundle(?)

    // dev mode?
    lib.set("dev", dev)?;

    // add all to the site
    // TODO: better naming?
    lib.set("latex2Mathml", mathml)?;
    lib.set("addHighlighters", add_highlighters)?;
    lib.set("highlightCodeHtml", highlight_code)?;
    lib.set("highlightCodeAst", highlight_ast)?;

    lib.set("parseToml", parse_toml)?;
    lib.set("parseYaml", parse_yaml)?;
    lib.set("parseJson", parse_json)?;
    lib.set("parseBibtex", parse_bibtex)?;
    lib.set("parseSass", parse_sass)?;
    //lib.set("parseMdl")?;

    lib.set("listFiles", list_files)?;
    lib.set("listDirectories", list_dirs)?;

    lib.set("fileExists", file_exists)?;
    lib.set("dirExists", dir_exists)?;

    lib.set("concatPath", concat_paths)?;
    lib.set("filename", file_name)?;
    lib.set("fileExtension", file_extension)?;
    lib.set("fileStem", file_stem)?;

    lib.set("openFile", open_file)?;
    lib.set("readFile", read_file)?;
    lib.set("newFile", new_file)?;
    lib.set("newBinaryFile", new_binary_file)?;
    lib.set("newBase64File", new_base64_file)?;

    //lib.set("filename")
    //lib.set("fileExtention")
    //lib.set("fileStem")

    lib.set("encodeBase64", encode_base64)?;
    lib.set("decodeBase64", decode_base64)?;

    lua.globals().set("site", lib)?;

    // set our own warning function
    let warnings = Rc::new(RefCell::new(Vec::<String>::new()));
    let warnings_cloned = warnings.clone();
    lua.set_warning_function(move |lua, text, _| {
        // Get the stack trace
        let mut trace = Vec::new();
        for frame in (0..).map_while(|i| lua.inspect_stack(i)) {
            let name = frame.source().short_src.unwrap_or("?".into());
            let what = frame.names().name_what;
            let func = frame
                .names()
                .name
                .unwrap_or(if frame.source().what == "main" {
                    "main chunk".into()
                } else {
                    "?".into()
                });
            let line = frame.curr_line();
            let line = if line < 0 {
                String::new()
            } else {
                format!(":{}", line)
            };
            if let Some(what) = what {
                trace.push(format!("\t{}{}: in {} '{}'", name, line, what, func));
            } else {
                trace.push(format!("\t{}{}: in {}", name, line, func));
            }
        }

        // give the stack trace to the warnings
        let warning = format!(
            "runtime warning: {}\nstack traceback:\n{}",
            text,
            trace.join("\n")
        );
        warnings_cloned.borrow_mut().push(warning);
        Ok(())
    });

    // load our own standard library
    let _: Value = lua.load_from_function(
        "slsg",
        lua.load(include_str!("stdlib.lua")).into_function()?,
    )?;

    // run file
    let script = fs::read_to_string(script_path)?;
    lua.load(script)
        .set_name("site.lua")
        .eval()
        .map(|x| Site {
            warnings: warnings.take(),
            ..x
        })
        .map_err(|x| GenerateError {
            warnings: warnings.take(),
            error: x.into(),
        })
}

// TODO: mdl
// file consists of paragraphs
// you can do some standard markdown functions
// per paragraph, a function is called to eval the paragraph
// => evals an inline lua function
// ==> evals a block lua function (outside paragraph)
