use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

use mlua::{ErrorContext, ExternalError, ExternalResult, Lua, Result, Table, chunk};
use relative_path::RelativePathBuf;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxSet, SyntaxSetBuilder},
};
use tera::{Context, Tera};

use crate::page::Page;

/// Recurse a directory, iterate over every entry
struct RecursiveDir {
    stack: Vec<PathBuf>,
    current: ReadDir,
}

impl RecursiveDir {
    fn begin(root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            stack: Vec::new(),
            current: std::fs::read_dir(root)?,
        })
    }
}

impl<'a> Iterator for &'a mut RecursiveDir {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        let path = loop {
            let Some(entry) = self.current.next() else {
                let Some(dir) = self.stack.pop() else {
                    return None;
                };

                match std::fs::read_dir(dir) {
                    Ok(dir) => {
                        self.current = dir;
                    }
                    Err(e) => return Some(Err(e.into_lua_err())),
                }

                continue;
            };

            let path = match entry {
                Ok(entry) => entry.path(),
                Err(e) => return Some(Err(e.into_lua_err())),
            };

            if path.is_dir() {
                self.stack.push(path);
                continue;
            };

            break path;
        };

        Some(Ok(path))
    }
}

pub(crate) struct Files {
    /// resulting files
    pub files: BTreeMap<RelativePathBuf, Vec<u8>>,

    /// which file to use as 404
    pub not_found: Option<RelativePathBuf>,
}

impl Files {
    pub fn write_to_path(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() {
            // remove the directory if it does exist and we are forced to do it
            if force {
                fs::remove_dir_all(path)
                    .into_lua_err()
                    .context("Failed to remove output directory")?;
            }
            // fail if the output directory is not empty and we are not forced to overwrite it
            else if path
                .read_dir()
                .into_lua_err()
                .context("Failed to check if path is empty")?
                .next()
                .is_none()
            {
                return Err(mlua::Error::external("Output directory is not empty"));
            }
        }

        // write out all files here
        for (output, contents) in self.files.iter() {
            // output path
            let path = output.to_path(path);

            // ensure it exists
            fs::create_dir_all(&path.parent().ok_or_else(|| {
                mlua::Error::external(format!("Path `{}` did not have a parent", path.display()))
            })?)
            .into_lua_err()
            .with_context(|_| format!("Failed to create directories for `{output}`"))?;

            // write out
            fs::write(&path, contents)
                .into_lua_err()
                .with_context(|_| {
                    format!("Failet to write file `{output}` to `{}`", path.display())
                })?;
        }

        Ok(())
    }
}

// TODO:

/// Track changes
struct Part<T> {
    dirty: bool,
    value: Result<T>,
}

impl<T> Part<T> {
    /// Create a new part
    fn new(msg: &str) -> Self {
        Self {
            dirty: true,
            value: Err(mlua::Error::external(msg)),
        }
    }

    /// Mark the part as dirty
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Update the value inside the part
    fn update(&mut self, factory: impl FnOnce() -> Result<T>) -> bool {
        if self.dirty {
            self.dirty = false;
            self.value = factory();
            true
        } else {
            false
        }
    }

    /// Get a reference to the value, or error
    fn value(&self) -> &Result<T> {
        &self.value
    }
}

pub(crate) struct Site {
    /// Lua state
    lua: Part<Lua>,

    /// lua API table
    api: Option<Table>,

    /// loaded syntaxes
    syntaxes: SyntaxSet,

    /// loaded user syntaxes
    user_syntaxes: Part<SyntaxSet>,

    /// Loaded themes
    themes: ThemeSet,

    /// loaded user themes
    user_themes: Part<ThemeSet>,

    /// templates
    templates: Part<Tera>,

    /// Generated pages
    pages: BTreeMap<RelativePathBuf, Page>,

    /// Index for the pager
    page_index: Result<BTreeMap<String, Vec<RelativePathBuf>>>,

    /// Page set as the 'not found' page
    not_found: Option<RelativePathBuf>,

    // TODO fonts?
    /// What pages to reload
    changed_pages: bool,

    /// Reload css?
    changed_css: bool,
}

impl Site {
    pub fn new() -> Result<Site> {
        // at least the site.lua or site.fnl file must exist and the pages directory
        if !(PathBuf::from("site.lua").is_file() || PathBuf::from("site.fnl").is_file())
            || !PathBuf::from("pages").is_dir()
        {
            return Err(mlua::Error::external(
                "`site.lua` or `site.fnl` and the `pages/` directory must exist",
            ));
        }

        // load defaults
        let mut site = Site {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
            lua: Part::new("Lua not loaded yet"),
            api: None,
            user_syntaxes: Part::new("User syntaxes not loaded yet"),
            user_themes: Part::new("User themes not loaded yet"),
            templates: Part::new("User templates not loaded yet"),
            page_index: Err(mlua::Error::external("Pages not loaded yet")),
            pages: BTreeMap::new(),
            not_found: None,
            changed_pages: true,

            changed_css: true,
        };

        // load the templates and themes and syntaxes
        site.manage_changes();

        Ok(site)
    }

    /// Reload everything that has changed and update internal state
    fn manage_changes(&mut self) {
        // if syntaxes changed, reload
        self.user_syntaxes.update(|| {
            if fs::exists("syntaxes")
                .into_lua_err()
                .context("Failed to check if syntaxes exist")?
            {
                let mut set = SyntaxSetBuilder::new();

                // add newlines
                set.add_from_folder("syntaxes", true)
                    .into_lua_err()
                    .context("Failed to load syntaxes")
                    .and_then(|_| Ok(set.build()))
            } else {
                Ok(SyntaxSet::new())
            }
        });

        // if themes changed, reload
        self.user_themes.update(|| {
            if fs::exists("themes")
                .into_lua_err()
                .context("Failed to check if themes exist")?
            {
                ThemeSet::load_from_folder("themes")
                    .into_lua_err()
                    .context("Failed to load themes")
            } else {
                Ok(ThemeSet::new())
            }
        });

        // if templates changed, reload
        self.templates.update(|| {
            if fs::exists("templates")
                .into_lua_err()
                .context("Failed to check if templates exist")?
            {
                // TODO also recurse directory?
                Tera::new("templates/**/*")
                    .into_lua_err()
                    .context("Failed to load templates")
            } else {
                Ok(Tera::default())
            }
        });

        // TODO css

        // update markdown
        // this is a bit more complex, as it happens in stages
        // here, just parse everything again
        if self.changed_pages {
            self.page_index = (|| {
                // index to build
                let mut index: BTreeMap<String, Vec<RelativePathBuf>> = BTreeMap::new();

                let md_ext = OsStr::new("md");

                // update pages
                self.pages = RecursiveDir::begin("pages")
                    .into_lua_err()?
                    .filter(|x| {
                        // only go over markdown pages
                        x.as_ref().is_ok_and(|x| x.extension() == Some(md_ext))
                    })
                    .map(|x| {
                        // for each page, rebuild it
                        let path = RelativePathBuf::from_path(x?).into_lua_err()?;

                        // new page
                        let page = Page::from_path_and_previous(&path, self.pages.remove(&path))?;

                        // add to the index
                        for tag in page.tags.iter() {
                            index
                                .entry(tag.clone())
                                .or_default()
                                .push(page.output.clone());
                        }

                        Ok((path, page))
                    })
                    .collect::<Result<_>>()?;

                Ok(index)
            })();
        }

        // if lua changed, reload
        self.lua.update(|| {
            // SAFETY: we want all the libraries
            let lua = unsafe { Lua::unsafe_new() };

            // TODO: load relevant functions

            // load fennel
            let fennel = lua
                .load(include_str!("fennel.lua"))
                .set_name("=fennel.lua")
                .into_function()
                .context("Failed to load fennel")?;

            // API table
            let api = lua.create_table()?;

            // load API
            lua.load(include_str!("api.lua"))
                .call::<()>((fennel, &api))
                .context("Failed to load API")?;

            // set it
            self.api = Some(api);

            // load the lua scripts
            if fs::exists("site.lua")? {
                let code = fs::read_to_string("site.lua")
                    .into_lua_err()
                    .context("Failed to load `site.lua`")?;
                lua.load(code).exec().context("Failed to run `site.lua`")?;
            } else if fs::exists("site.fnl")? {
                let code = fs::read_to_string("site.fnl")
                    .into_lua_err()
                    .context("Failed to load `site.fnl`")?;
                lua.load(chunk! {
                    require("fennel").eval(
                        $code,
                        { ["error-pinpoint"] = false, filename = "site.fnl" }
                    );
                })
                .exec()
                .context("Failed to run `site.fnl`")?;
            } else {
                return Err(mlua::Error::external(
                    "No `site.lua` or `site.fnl` file found",
                ));
            }

            // set the lua state
            Ok(lua)
        });

        // TODO create lolhtml settings and other settings

        // update the output of all pages
        // only do so if we have a page index
        if let Ok(index) = self.page_index.as_ref()
            && let Ok(templates) = self.templates.value()
            && let Ok(lua) = self.lua.value()
            && let Some(api) = self.api.as_ref()
        {
            for page in self.pages.values_mut() {
                // TODO only redo these if the things changed
                // TODO add index
                // TODO add other stuff maybe (css pages?)
                // context for the page
                let mut context = Context::new();
                context.insert("content", &page.html);

                // only update if the template changed
                page.result = templates
                    .render(&page.template, &context)
                    .into_lua_err()
                    .context("Failed to apply template");

                // TODO run lolhtml
            }
        }
    }

    /// Generate files from the current state.
    /// `development` is whether the development flag is set to true in tera and lua.
    pub fn generate_files(&mut self, development: bool) -> Result<Files> {
        // reload all files that changed
        self.manage_changes();

        // check if there are any errors
        self.page_index.as_ref().map_err(|x| x.clone())?;
        self.user_syntaxes.value().as_ref().map_err(|x| x.clone())?;
        self.user_themes.value().as_ref().map_err(|x| x.clone())?;
        self.templates.value().as_ref().map_err(|x| x.clone())?;
        self.lua.value().as_ref().map_err(|x| x.clone())?;

        let mut files = Files {
            not_found: self.not_found.clone(),
            files: BTreeMap::new(),
        };

        for page in self.pages.values() {
            // insert the page
            files.files.insert(
                page.output.clone(),
                page.result
                    .clone()
                    .map(|x| x.into_bytes())
                    .with_context(|_| format!("Could not output page `{}`", &page.output))?,
            );
        }

        // copy over the pages
        Ok(files)
    }

    /// Generate files from the current directory
    pub fn generate() -> Result<Files> {
        // set up self, then generate all files
        // no development mode
        Self::new()?.generate_files(false)
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: RelativePathBuf) {
        // markdown file changed
        if path.extension() == Some("md") || path.starts_with("pages") {
            self.changed_pages = true;
        }

        // stylesheet changed
        if [Some("sass"), Some("scss"), Some("css")].contains(&path.extension())
            && path.starts_with("styles")
        {
            self.changed_pages = true;
        }

        // lua file changed?
        if [Some("fnl"), Some("lua")].contains(&path.extension()) || path.starts_with("scripts") {
            self.lua.mark_dirty();
        }

        // template changed?
        if path.starts_with("templates") {
            self.templates.mark_dirty();
        }

        // themes changed?
        if path.starts_with("themes") {
            self.user_themes.mark_dirty();
        }

        // syntaxes changed?
        if path.starts_with("syntaxes") {
            self.user_syntaxes.mark_dirty();
        }
    }
}
