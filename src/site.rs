use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowCtx, Result, anyhow, bail, ensure};
use globwalk::{GlobWalkerBuilder, glob};

use lol_html::{RewriteStrSettings, rewrite_str};
use mlua::{Lua, chunk};
use pulldown_cmark::{Options, Parser, html::push_html};
use relative_path::RelativePathBuf;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxSet, SyntaxSetBuilder},
};
use tera::{Context, Tera};

pub(crate) struct Files {
    /// resulting files
    pub files: HashMap<RelativePathBuf, Vec<u8>>,

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

        // ensure the path exists
        fs::create_dir_all(path)?;

        // write out all files here
        for (output, contents) in self.files.iter() {
            // output path
            let path = output.to_path(path);

            // ensure it exists
            fs::create_dir_all(
                &path
                    .parent()
                    .ok_or_else(|| anyhow!("Path '{}' did not have a parent", path.display()))?,
            )
            .with_context(|| format!("Failed to create directories for '{output}'"))?;

            // write out
            fs::write(&path, contents).with_context(|| {
                format!("Failet to write file '{output}' to '{}'", path.display())
            })?;
        }

        Ok(())
    }
}

/// Page entry for the pages to generate later
struct Page {
    /// Filepath
    path: PathBuf,

    /// Where to output to
    output: String,

    /// Generated html from the markdown
    html: String,

    /// template to use
    template: String,

    /// tags to use
    tags: Vec<String>,

    /// Directory to include files from, as glob pattern
    include: Option<String>,

    /// frontmatter metadata
    frontmatter: toml::Table,
}

/// Cached state to generate a site with
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

    /// Generate the files for the current site state, from the current working directory
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

        // ensure lua files, pages and templates directory are present
        ensure!(
            fs::exists("site.lua")? || fs::exists("site.fnl")?,
            "'site.lua' or 'site.fnl' not present"
        );
        ensure!(
            fs::exists("templates")?,
            "'templates' directory not present"
        );
        ensure!(fs::exists("pages")?, "'pages' directory not present");

        // load templates
        if self.templates.is_none() {
            self.templates = Some(Tera::new("templates/**/*").context("Failed to load templates")?);
        }

        // load pages and create the page index
        let mut pages = Vec::new();
        let mut index = HashMap::new();
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
                        "No toml-style ('+++' delimited) frontmatter for page '{}'",
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

            // files to include
            let include = frontmatter
                .get("include")
                .map(|x| {
                    x.as_str().ok_or(anyhow!(
                        "No 'template = \"page.html\"' value for page '{}'",
                        path.path().display()
                    ))
                })
                .transpose()?;

            // tag(s)
            let tags = if let Some(tag) = frontmatter.get("tag") {
                // single value
                vec![
                    tag.as_str()
                        .ok_or(anyhow!(
                            "Value 'tag = \"tag\" needs to be a string for page '{}'",
                            path.path().display()
                        ))?
                        .to_string(),
                ]
            } else if let Some(tags) = frontmatter.get("tags") {
                // multiple values
                tags.as_array()
                    .and_then(|x| x.iter().map(|x| x.as_str().map(|x| x.to_string())).collect())
                    .ok_or(anyhow!(
                        "Value 'tags = [\"tag1\", \"tag2\"]' must be an array of strings for page '{}'",
                        path.path().display(),
                    ))?
            } else {
                // no tags
                Vec::new()
            };

            // parse the markdown
            let parser = Parser::new_ext(
                &md,
                Options::ENABLE_MATH | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
            );

            // convert to html
            let mut html = String::with_capacity(md.len());
            push_html(&mut html, parser);

            // and page index
            for tag in tags.iter() {
                index.insert(tag.clone(), out.to_string());
            }

            // these are processed later, as a complete index is needed
            pages.push(Page {
                path: path.into_path(),
                output: out.to_string(),
                html,
                template: template.to_string(),
                include: include.map(|x| x.to_string()),
                tags: tags,
                frontmatter: frontmatter,
            });
        }

        // TODO cache this as well?
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
                require("fennel")
                    .install()
                    // pinpoint errors using html, as this highlights it in the preview
                    // the terminal error reporter also renders html TODO
                    .dofile("site.fnl", { ["error-pinpoint"] = { "*", "*" } });
            })
            .exec()?;
        } else {
            bail!("No 'site.lua' or 'site.fnl' present");
        }

        // all files to output
        let mut files = HashMap::with_capacity(pages.len());

        // global context for all the templates
        let mut ctx = Context::new();

        // TODO insert page index here

        // TODO insert style sheets here

        // TODO lua functions?

        // template the files
        for page in pages {
            // page content
            ctx.insert("content", &page.html);

            // apply template
            let html = self
                .templates
                .as_ref()
                .expect("templates was none, this should not be able to happen")
                .render(&page.template, &ctx)
                .with_context(|| format!("Failed to template page '{}'", page.path.display()))?;

            // apply lolhtml
            let html = rewrite_str(&html, RewriteStrSettings::new())
                .with_context(|| format!("Failed to rewrite page '{}'", page.path.display()))?;

            // write out
            // TODO handle index.html properly here
            files.insert(RelativePathBuf::from(page.output), html.into_bytes());

            // write out included files
            if let Some(incl) = page.include {
                for file in GlobWalkerBuilder::new(page.path, incl).build()? {
                    todo!()
                }
            }
        }

        // TODO write out stylesheets

        Ok(Files {
            files,
            // load from the lua env
            // TODO
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
    pub fn mark_dirty(&mut self, path: &Path) {
        // stylesheets changed?
        if ["sass", "scss", "css"]
            .map(|x| Some(OsStr::new(x)))
            .contains(&path.extension())
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
