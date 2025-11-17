use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowCtx, Result, anyhow, bail, ensure};
use globwalk::{GlobWalkerBuilder, glob};

use grass::OutputStyle;

use mlua::{Function, Lua, chunk};
use pulldown_cmark::{Options, Parser, html::push_html};
use relative_path::RelativePathBuf;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxSet, SyntaxSetBuilder},
};
//use tera::{Context, Tera};

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
            let path = path.join(output.to_logical_path(""));

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

    /// Stylesheets
    styles: Option<HashMap<String, String>>,
}

impl SiteCache {
    pub(crate) fn new() -> SiteCache {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
            user_syntaxes: None,
            user_themes: None,
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
                // all styles
                let mut styles = HashMap::new();

                // go over all styles that can be compiled
                // css, sass and scss
                for path in glob("styles/**/*.{css,sass,scss}")? {
                    let path = path?;

                    // options for grass
                    let opts = grass::Options::default()
                        .load_path("styles")
                        // TODO config?
                        .style(OutputStyle::Expanded)
                        .input_syntax(if path.path().extension() == Some(OsStr::new("scss")) {
                            grass::InputSyntax::Scss
                        } else if path.path().extension() == Some(OsStr::new("sass")) {
                            grass::InputSyntax::Sass
                        } else {
                            grass::InputSyntax::Css
                        });

                    // compile css
                    let css = grass::from_path(path.path(), &opts).with_context(|| {
                        format!("Failed to compile '{}' to css", path.path().display())
                    })?;

                    // insert
                    styles.insert(
                        path.path()
                            .file_stem()
                            .ok_or_else(|| {
                                anyhow!(
                                    "Path '{}' does not have a file stem",
                                    path.path().display()
                                )
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                anyhow!(
                                    "Could not convert path '{}' to an utf8 string",
                                    path.path().display()
                                )
                            })?
                            .to_string(),
                        css,
                    );
                }

                Some(styles)
            } else {
                Some(HashMap::new())
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
                Options::ENABLE_MATH
                    | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
                    | Options::ENABLE_HEADING_ATTRIBUTES
                    | Options::ENABLE_FOOTNOTES,
            );

            // rewrite math and code blocks to the preferred elements
            // TODO probably just copy/port the cmark-pulldown push-html code and insert the lua functions there?
            let mut html = String::with_capacity(md.len());
            push_html(&mut html, parser);

            // and page index
            for tag in tags.iter() {
                index.insert(tag.clone(), frontmatter.clone());
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

        // load lua
        // SAFETY: we want all lua libraries
        let lua = unsafe { Lua::unsafe_new() };

        // map of rewrite functions
        let rewrite_fns = lua.create_table()?;

        // map of files to output from lua
        let emitted_files = lua.create_table()?;

        // style sheets
        let styles = lua.create_table_from(
            self.styles
                .as_ref()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned())),
        )?;

        // load fennel
        let fennel = lua
            .load(include_str!("fennel.lua"))
            .set_name("=fennel.lua")
            .into_function()
            .context("Failed to load fennel")?;

        // set up lua context
        let rw = rewrite_fns.clone();
        let em = emitted_files.clone();
        lua.load(chunk! {
            // preload fennel
            package.preload.fennel = $fennel;

            // add scripts directory to the load path
            package.path = "./scripts/?.lua;" .. package.path;

            // site API table
            site = {
                // not found page is nil by default
                not_found = nil;

                // are we in development mode?
                dev = $development;

                // style sheet index
                styles = $styles;

                // rewrite functions
                rewrite = $rw;

                // files to emit
                emit = $em;
            };
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
                    // pinpoint errors with another character, as the default messes
                    // up terminal color output
                    .dofile("site.fnl", { ["error-pinpoint"] = { "\x02", "\x03" } });
            })
            .exec()?;
        } else {
            bail!("No 'site.lua' or 'site.fnl' present");
        }

        // rewrite functions
        let rewriters = rewrite_fns
            .pairs()
            .map(|x| {
                let (name, fun): (String, Function) = x?;
                Ok((name, fun))
            })
            .collect::<Result<HashMap<String, Function>>>()?;

        // all files to output
        let mut files = HashMap::with_capacity(pages.len());

        // template the files
        for page in pages {
            // TODO template with lua
            // JSX style, so replace elements with a function call
            // ALSO: returned functions are run again after everything is processed?
            // this allows doing things in stages, kinda?
            let html = page.html;

            // apply template lua stuff TODO

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

        // write out stylesheets
        for (path, style) in self.styles.as_ref().unwrap().iter() {
            files.insert(
                // no extention is present, so add the css one
                RelativePathBuf::from(format!("{path}.css")),
                style.as_bytes().to_vec(),
            );
        }

        // write out emitted files
        for pair in emitted_files.pairs() {
            let (path, content): (String, mlua::BString) = pair?;
        }

        Ok(Files {
            files,

            // load from the lua env
            not_found: lua
                .globals()
                .get::<mlua::Table>("site")?
                .get::<Option<String>>("not_found")?
                .map(RelativePathBuf::from),
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
            //self.templates = None;
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
