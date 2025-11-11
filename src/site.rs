use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowCtx, Error, Result, anyhow, bail, ensure};
use globwalk::glob;

use mlua::{ExternalError, ExternalResult, Lua, Table, chunk};
use pulldown_cmark::{Options, Parser, html::push_html};
use relative_path::RelativePathBuf;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxSet, SyntaxSetBuilder},
};
use tera::{Context, Tera};

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
                fs::remove_dir_all(path).context("Failed to remove output directory")?;
            }
            // fail if the output directory is not empty and we are not forced to overwrite it
            else if path
                .read_dir()
                .context("Failed to check if path is empty")?
                .next()
                .is_none()
            {
                return Err(anyhow!("Output directory is not empty"));
            }
        }

        // write out all files here
        for (output, contents) in self.files.iter() {
            // output path
            let path = output.to_path(path);

            // ensure it exists
            fs::create_dir_all(
                &path
                    .parent()
                    .ok_or_else(|| anyhow!("Path `{}` did not have a parent", path.display()))?,
            )
            .with_context(|| format!("Failed to create directories for `{output}`"))?;

            // write out
            fs::write(&path, contents).with_context(|| {
                format!("Failet to write file `{output}` to `{}`", path.display())
            })?;
        }

        Ok(())
    }
}

pub(crate) struct SiteCache {
    /// loaded syntaxes
    syntaxes: SyntaxSet,

    /// loaded user syntaxes
    user_syntaxes: Option<SyntaxSet>,

    /// Loaded themes
    themes: ThemeSet,

    /// loaded user themes
    user_themes: Option<ThemeSet>,

    /// templates
    templates: Option<Tera>,

    /// Stylesheets
    styles: Option<()>,
}

impl SiteCache {
    pub(crate) fn new() -> SiteCache {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
            user_syntaxes: None,
            user_themes: None,
            templates: None,
            styles: None,
        }
    }

    /// Generate the files
    /// `development` is passed on to the lua state
    pub(crate) fn generate(&mut self, development: bool) -> Result<Files> {
        // load syntaxes
        if self.user_syntaxes.is_none() {
            // if the folder exists, load from there
            self.user_syntaxes = if fs::exists("syntaxes")? {
                let mut set = SyntaxSetBuilder::new();

                // add with newlines
                set.add_from_folder("syntaxes", true)
                    .context("Failed to load syntaxes")?;

                Some(set.build())
            } else {
                // empty default set
                Some(SyntaxSet::new())
            }
        }

        // load themes
        if self.user_themes.is_none() {
            self.user_themes = if fs::exists("themes")? {
                Some(ThemeSet::load_from_folder("themes").context("Failed to load themes")?)
            } else {
                // default empty set
                Some(ThemeSet::new())
            }
        }

        // load styles
        if self.styles.is_none() {
            self.styles = if fs::exists("styles")? {
                // go over all styles that can be compiled
                // css, sass and scss
                for path in glob("styles/**/*.{css,sass,scss}")? {
                    let path = path?;
                    dbg!(path);
                }

                Some(())
            } else {
                Some(())
            };
        }

        // load templates
        if self.templates.is_none() {
            // templates need to exist
            ensure!(
                fs::exists("templates")?,
                "'templates' directory needs to exist"
            );

            // and load
            self.templates = Some(Tera::new("templates/**/*").context("Failed to load templates")?);
        }

        // load pages and create the page index
        ensure!(fs::exists("pages")?, "Pages directory needs to exist");
        let mut pages = Vec::new();
        let mut index = BTreeMap::new();
        for path in glob("pages/**/*.md")? {
            let path = path?;

            // load markdown
            let md = fs::read_to_string(path.path()).with_context(|| {
                format!("Failed to load markdown at '{}'", path.path().display())
            })?;

            // parse front matter
            // pulldown-cmark expects the metadata block to start and end with a +++
            // without any preceding spaces
            // find the range of the first metadata block
            let frontmatter = md
                // opens with a +++, so no preceding newline
                .starts_with("+++")
                .then_some(3)
                // or if there's spaces between, there is a preceding newline
                .or_else(|| md.find("\n+++").map(|x| x + 4))
                // then, find the closing tag
                .and_then(|start| {
                    // offset to not search inside the opening tag
                    // + 3 to skip the opening tag
                    md[start..]
                        // it must come after the opening tag
                        .find("\n+++")
                        // starts at the first +++, ends at the second +++
                        // start + end because the end is relative
                        .map(|end| &md[start..start + end])
                })
                .ok_or_else(|| {
                    anyhow!(
                        "Page '{}' does not have toml-style ('+++' delimited) frontmatter",
                        path.path().display()
                    )
                })?
                // parse into a toml table
                .parse::<toml::Table>()
                .with_context(|| {
                    format!(
                        "Failed to parse frontmatter for page '{}'",
                        path.path().display()
                    )
                })?;

            // recover the front matter tags
            // output directory
            let out = frontmatter
                .get("out")
                .and_then(|x| x.as_str())
                .ok_or(anyhow!(
                    "No 'out = \"output.html\"' value for page '{}'",
                    path.path().display()
                ))?;

            // template to use
            let template = frontmatter
                .get("template")
                .and_then(|x| x.as_str())
                .ok_or(anyhow!(
                    "No 'template = \"page.html\"' value for page '{}'",
                    path.path().display()
                ))?;

            // tag(s)
            let tags = frontmatter
                .get("tag")
                .and_then(|x| {
                    // single tag
                    Some(vec![x.as_str()?])
                })
                .or_else(|| {
                    frontmatter
                        .get("tags")
                        // many tags
                        .and_then(|x| x.as_array())
                        // collect them into an array
                        .and_then(|x| x.iter().map(|x| x.as_str()).collect::<Option<_>>())
                })
                .ok_or(anyhow!(
                    "No 'tag = \"tag\"' or 'tags = [\"tag1\", \"tag2\"]' value for page '{}'",
                    path.path().display()
                ))?;

            // parse the markdown
            let parser = Parser::new_ext(
                &md,
                Options::ENABLE_MATH | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
            );

            // convert to html
            let mut html = String::with_capacity(md.len());
            let html = push_html(&mut html, parser);

            // ad
            pages.push(html.clone());
            index.insert(path.into_path(), html);
        }

        // load lua
        // SAFETY: we want all lua libraries
        let lua = unsafe { Lua::unsafe_new() };

        // load api
        // TODO relevant functions

        // load fennel
        let fennel = lua
            .load(include_str!("fennel.lua"))
            .set_name("=fennel.lua")
            .into_function()
            .context("Failed to load fennel")?;

        // set up lua context
        lua.load(chunk! {
            // preload fennel
            package.preload.fennel = $fennel;

            // add scripts directory to the load path
            package.path = "./scripts/?.lua;" .. package.path;
        })
        .exec()
        .context("Failed to load fennel")?;

        // load site.lua
        if fs::exists("site.lua")? {
            lua.load(Path::new("site.lua")).exec()?;
        } else if fs::exists("site.fnl")? {
            lua.load(chunk! {
                // load and install fennel, then run the file
                // disable error pinpoint as this messes with existing coloration of the terminal
                require("fennel").install().dofile("site.fnl", { ["error-pinpoint"] = false });
            })
            .exec()?;
        } else {
            bail!("No 'site.lua' or 'site.fnl' present");
        }

        Ok(Files {
            files: BTreeMap::new(),
            not_found: None,
        })
    }

    /// Generate files from the current directory
    pub fn new_site() -> Result<Files> {
        // set up self, then generate all files
        // no development mode
        Self::new().generate(false)
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: RelativePathBuf) {
        // stylesheet changed
        if [Some("sass"), Some("scss"), Some("css")].contains(&path.extension())
            && path.starts_with("styles")
        {
            self.styles = None;
        }

        // template changed?
        if path.starts_with("templates") {
            self.templates = None;
        }

        // themes changed?
        if path.starts_with("themes") {
            self.user_themes = None;
        }

        // syntaxes changed?
        if path.starts_with("syntaxes") {
            self.user_syntaxes = None;
        }
    }
}
