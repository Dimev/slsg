use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use mlua::{ErrorContext, ExternalResult, Lua, Result, chunk};
use relative_path::RelativePathBuf;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxSet, SyntaxSetBuilder},
};
use tera::Tera;

pub(crate) struct Files {
    /// resulting files
    pub files: BTreeMap<RelativePathBuf, Vec<u8>>,

    /// which file to use as 404
    pub not_found: Option<RelativePathBuf>,
}

impl Files {
    pub fn write_to_path(&self, path: &Path, force: bool) -> Result<()> {
        // fail if the output directory is not empty and we are not forced to overwrite it
        if !force
            && path.exists()
            && path
                .read_dir()
                .into_lua_err()
                .context("Failed to check if path is empty")?
                .next()
                .is_none()
        {
            return Err(mlua::Error::external("Output directory is not empty"));
        }

        // remove the directory if it does exist and we are forced to do it
        if force {
            fs::remove_dir_all(path)
                .into_lua_err()
                .context("Failed to remove output directory")?;
        }

        // write out all files here
        for (output, contents) in self.files.iter() {
            // output path
            let path = output.to_path(path);

            // ensure it exists
            fs::create_dir_all(&path)
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

pub(crate) struct Site {
    /// Lua state
    lua: Result<Lua>,

    /// loaded syntaxes
    syntaxes: SyntaxSet,

    /// loaded user syntaxes
    user_syntaxes: Result<SyntaxSet>,

    /// Loaded themes
    themes: ThemeSet,

    /// loaded user themes
    user_themes: Result<ThemeSet>,

    /// templates
    templates: Result<Tera>,

    // TODO fonts?
    /// What pages to reload
    changed_pages: BTreeSet<RelativePathBuf>,

    /// Reload templates?
    changed_templates: bool,

    /// Reload css?
    changed_css: bool,

    /// Reload lua?
    changed_lua: bool,

    /// Reload syntaxes?
    changed_syntaxes: bool,

    /// Reload themes?
    changed_themes: bool,
}

impl Site {
    pub fn new() -> Result<Site> {
        let mut site = Site {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
            lua: Err(mlua::Error::external("Lua not loaded yet")),
            user_syntaxes: Err(mlua::Error::external("User syntaxes not loaded yet")),
            user_themes: Err(mlua::Error::external("User themes not loaded yet")),
            templates: Err(mlua::Error::external("Templates not loaded yet")),
            changed_pages: BTreeSet::new(),
            changed_templates: true,
            changed_css: true,
            changed_lua: true,
            changed_syntaxes: true,
            changed_themes: true,
        };

        // load the templates and themes and syntaxes
        site.manage_changes()?;

        Ok(site)
    }

    /// Reload everything that has changed and update internal state
    fn manage_changes(&mut self) -> Result<()> {
        // if lua changed, reload
        if self.changed_lua {
            self.changed_lua = false;

            // function, because we want to catch the errors if anything fails to load
            self.lua = (|| {
                // SAFETY: we want all the libraries
                let lua = unsafe { Lua::unsafe_new() };

                // add the scripts directory to the loader
                // TODO

                // load fennel
                let fennel = lua
                    .load(include_str!("fennel.lua"))
                    .set_name("=fennel.lua")
                    .into_function()
                    .context("Failed to load fennel")?;
                lua.load(chunk! {
                    // load the fennel package
                    package.preload["fennel"] = $fennel;
                })
                .exec()
                .context("Failed to load fennel into lua")?;

                // TODO: load relevant functions

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
            })();
        }

        // if syntaxes changed, reload
        if self.changed_syntaxes {
            self.changed_syntaxes = false;
            self.user_syntaxes = if fs::exists("syntaxes")
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
            };
        }

        // if themes changed, reload
        if self.changed_themes {
            self.changed_themes = false;
            self.user_themes = if fs::exists("themes")
                .into_lua_err()
                .context("Failed to check if themes exist")?
            {
                ThemeSet::load_from_folder("themes")
                    .into_lua_err()
                    .context("Failed to load themes")
            } else {
                Ok(ThemeSet::new())
            };
        }

        // if templates changed, reload
        if self.changed_templates {
            self.changed_templates = false;
            self.templates = if fs::exists("templates")
                .into_lua_err()
                .context("Failed to check if templates exist")?
            {
                Tera::new("templates/**/*.tera")
                    .into_lua_err()
                    .context("Failed to load templates")
            } else {
                Ok(Tera::default())
            }
        }

        // TODO: markdown change detection
        for file in self.changed_pages.iter() {
            // TODO
            // read all .md files in ./pages
            // parse them
            // 
        }

        // no more changes to process
        self.changed_pages.clear();

        // TODO build index?

        Ok(())
    }

    /// Generate files from the current state.
    /// `development` is whether the development flag is set to true in tera and lua.
    pub fn generate_files(&mut self, development: bool) -> Result<Files> {
        // reload all files that changed
        self.manage_changes()?;

        // TODO: build index

        // TODO convert with lol_html
        // TODO copy out files
        // TODO only run lua on the changed pages?
        // TODO for lua: register lolhtml replacers, functions for making mathml and code are provided
        // TODO also lua: determine the 404 file

        Err(mlua::Error::external("Not yet implemented"))
    }

    /// Generate files from the current directory
    pub fn generate() -> Result<Files> {
        // set up self, then generate all files
        Self::new()?.generate_files(false)
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: RelativePathBuf) {
        // lua file changed?
        if [Some("fnl"), Some("lua")].contains(&path.extension()) || path.starts_with("scripts") {
            self.changed_lua = true;
        }

        // template changed?
        if path.starts_with("templates") {
            self.changed_templates = true;
        }

        // themes changed?
        if path.starts_with("themes") {
            self.changed_themes = true;
        }

        // syntaxes changed?
        if path.starts_with("syntaxes") {
            self.changed_syntaxes = true;
        }
    }
}
