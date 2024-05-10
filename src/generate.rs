use std::{
    cell::RefCell,
    collections::HashMap,
    iter::{FilterMap, Peekable},
    rc::Rc,
};

use anyhow::anyhow;
use latex2mathml::{latex_to_mathml, DisplayStyle};
use mlua::{ErrorContext, FromLua, Lua, LuaOptions, StdLib, Table, Value};
use nom_bibtex::Bibtex;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::{
    file::File,
    highlight::{self, highlight, highlight_html, HighlightRule},
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
        let not_found = table.get("NotFound")?;

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
        let path = path_owned.join(&script);
        let code = fs::read_to_string(path).map_err(mlua::Error::external)?;
        let function = lua.load(code).into_function()?;
        lua.load_from_function::<Value>(&script, function)
    })?;

    lua.globals().set("require", require)?;

    // language highlighters
    let highlighters = Rc::new(RefCell::new(HashMap::<String, Vec<HighlightRule>>::new()));

    // load our library functions
    let lib = lua.create_table()?;

    // TODO
    // list directories
    // list all directories

    // list files
    // list all files

    // read file
    // TODO: file api(?)
    // TODO: maybe only support loading to string, not have a full file
    // so pages are easier to do

    // new file
    let file = lua.create_function(|_, content| Ok(File::New(content)))?;

    // parse toml
    // parse yaml
    // parse json
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

    lib.set("parseBibtex", parse_bibtex)?;

    lib.set("file", file)?;

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
