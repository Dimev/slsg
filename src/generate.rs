use std::{collections::BTreeMap, fs};

use mlua::{ErrorContext, ExternalResult, Lua, Result, chunk};
use relative_path::RelativePathBuf;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use tera::Tera;

pub(crate) struct Files {
    /// resulting files
    pub files: BTreeMap<RelativePathBuf, Vec<u8>>,

    /// which file to use as 404
    pub not_found: Option<RelativePathBuf>,
}

pub(crate) struct Site {
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

    // TODO style, fonts?
    /// What pages to reload
    changed_pages: Vec<RelativePathBuf>,

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
        // load syntaxes
        let syntaxes = SyntaxSet::load_defaults_nonewlines();
        let user_syntaxes = if fs::exists("./syntaxes")
            .into_lua_err()
            .context("Failed to check if syntaxes exist")?
        {
            SyntaxSet::load_from_folder("./syntaxes")
                .into_lua_err()
                .context("Failed to load syntaxes")
        } else {
            Ok(SyntaxSet::new())
        };

        // load themes
        let themes = ThemeSet::load_defaults();
        let user_themes = if fs::exists("./themes")
            .into_lua_err()
            .context("Failed to check if themes exist")?
        {
            ThemeSet::load_from_folder("./themes")
                .into_lua_err()
                .context("Failed to load themes")
        } else {
            Ok(ThemeSet::new())
        };

        // templates
        let templates = Tera::new("./templates/**/*.tera")
            .into_lua_err()
            .context("Failed to load templates");

        // TODO markdown posts

        Ok(Site {
            syntaxes,
            user_syntaxes,
            themes,
            user_themes,
            changed_pages: Vec::new(),
            templates,
            changed_templates: false,
            changed_css: false,
            changed_lua: false,
            changed_syntaxes: false,
            changed_themes: false,
        })
    }

    pub fn generate_files(&mut self) -> Result<Files> {
        // if syntaxes changed, reload
        if self.changed_syntaxes {
            self.changed_syntaxes = false;
            self.user_syntaxes = if fs::exists("./syntaxes")
                .into_lua_err()
                .context("Failed to check if syntaxes exist")?
            {
                SyntaxSet::load_from_folder("./syntaxes")
                    .into_lua_err()
                    .context("Failed to load syntaxes")
            } else {
                Ok(SyntaxSet::new())
            };
        }

        // if themes changed, reload
        if self.changed_themes {
            self.changed_themes = false;
            self.user_themes = if fs::exists("./themes")
                .into_lua_err()
                .context("Failed to check if themes exist")?
            {
                ThemeSet::load_from_folder("./themes")
                    .into_lua_err()
                    .context("Failed to load themes")
            } else {
                Ok(ThemeSet::new())
            };
        }

        // if templates changed, reload
        if self.changed_templates {
            self.changed_templates = false;
            self.templates = Tera::new("./templates/**/*.tera")
                .into_lua_err()
                .context("Failed to load templates");
        }

        // TODO: markdown change detection
        for file in self.changed_pages.iter() {
            // TODO
        }

        // no more changes to process
        self.changed_pages.clear();

        // TODO: build index

        // load lua
        // SAFETY: we want all libraries
        let lua = unsafe { Lua::unsafe_new() };

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

        // TODO convert with lol_html
        // TODO copy out files
        // TODO only run lua on the changed pages?
        // TODO for lua: register lolhtml replacers, functions for making mathml and code are provided
        // TODO also lua: determine the 404 file

        todo!()
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: RelativePathBuf) {
        // TODO: update the file
    }
}
