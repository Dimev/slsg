use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
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

use crate::page::Page;

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
        if force && path.exists() {
            fs::remove_dir_all(path)
                .into_lua_err()
                .context("Failed to remove output directory")?;
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

    /// Generated pages
    pages: BTreeMap<RelativePathBuf, Page>,

    /// Error when parsing the pages
    page_error: Option<mlua::Error>,

    /// Page set as the 'not found' page
    not_found: Option<RelativePathBuf>,

    // TODO fonts?
    /// What pages to reload
    changed_pages: bool,

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
            lua: Err(mlua::Error::external("Lua not loaded yet")),
            user_syntaxes: Err(mlua::Error::external("User syntaxes not loaded yet")),
            user_themes: Err(mlua::Error::external("User themes not loaded yet")),
            templates: Err(mlua::Error::external("Templates not loaded yet")),
            page_error: Some(mlua::Error::external("Pages not loaded yet")),
            pages: BTreeMap::new(),
            not_found: None,
            changed_pages: true,
            changed_templates: true,
            changed_css: true,
            changed_lua: true,
            changed_syntaxes: true,
            changed_themes: true,
        };

        // load the templates and themes and syntaxes
        site.manage_changes();

        Ok(site)
    }

    /// Reload everything that has changed and update internal state
    fn manage_changes(&mut self) {
        // if syntaxes changed, reload
        if self.changed_syntaxes {
            self.user_syntaxes = (|| {
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
            })();
        }

        // if themes changed, reload
        if self.changed_themes {
            self.user_themes = (|| {
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
            })();
        }

        // if templates changed, reload
        if self.changed_templates {
            self.templates = (|| {
                if fs::exists("templates")
                    .into_lua_err()
                    .context("Failed to check if templates exist")?
                {
                    Tera::new("templates/**/*")
                        .into_lua_err()
                        .context("Failed to load templates")
                } else {
                    Ok(Tera::default())
                }
            })();
        }

        // TODO css

        // update markdown
        // this is a bit more complex, as it happens in stages
        // here, just parse everything again
        if self.changed_pages {
            self.page_error = (|| {
                // go over all files in the pages directory
                let mut stack = vec![PathBuf::from("pages")];
                let mut pages = Vec::new();
                while let Some(path) = stack.pop() {
                    // read all files in the directory
                    for path in path.read_dir()? {
                        let path = path?.path();
                        if path.is_file()
                            && path.extension() == Some(OsString::from("md").as_os_str())
                        {
                            pages.push(
                                RelativePathBuf::from_path(&path)
                                    .into_lua_err()
                                    .with_context(|_| {
                                        format!(
                                            "Failed to convert path `{}` into relative path",
                                            path.display()
                                        )
                                    })?,
                            );
                        } else if path.is_dir() {
                            stack.push(path);
                        }
                    }
                }

                // read the markdown
                for path in pages {
                    // reload the page
                    let md = Page::from_path_and_previous(&path, self.pages.remove(&path))?;

                    // put it back, now that it has been processed again
                    self.pages.insert(path, md);
                }

                Ok(())
            })()
            .err();

            // TODO: update markdown
        }

        // TODO build index?

        // if lua changed, reload
        if self.changed_lua {
            // function, because we want to catch the errors if anything fails to load
            self.lua = (|| {
                // SAFETY: we want all the libraries
                let lua = unsafe { Lua::unsafe_new() };

                // TODO: load relevant functions

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

        // reset changes
        self.changed_pages = false;
        self.changed_templates = false;
        self.changed_css = false;
        self.changed_lua = false;
        self.changed_syntaxes = false;
        self.changed_themes = false;
    }

    /// Generate files from the current state.
    /// `development` is whether the development flag is set to true in tera and lua.
    pub fn generate_files(&mut self, development: bool) -> Result<Files> {
        // reload all files that changed
        self.manage_changes();

        // check if there are any errors
        self.page_error
            .as_ref()
            .map_or(Ok(()), |x| Err(x.clone()))?;
        self.user_syntaxes.as_ref().map_err(|x| x.clone())?;
        self.user_themes.as_ref().map_err(|x| x.clone())?;
        self.templates.as_ref().map_err(|x| x.clone())?;
        self.lua.as_ref().map_err(|x| x.clone())?;

        // copy over the pages
        Ok(Files {
            not_found: self.not_found.clone(),
            files: self
                .pages
                .iter()
                .map(|(k, v)| (k.clone(), v.html.bytes().collect()))
                .collect(),
        })
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
